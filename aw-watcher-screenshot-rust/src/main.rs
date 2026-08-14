//! Cattura periodica di TUTTI i monitor uniti in una sola immagine,
//! salvata come JPEG compresso in locale — dentro static/screenshots
//! della webui, così può essere servita come un qualsiasi altro asset
//! statico. Porting 1:1 da aw_watcher_screenshot/main.py (Python, che
//! usava la libreria `mss`).
//!
//! `mss` esponeva `monitors[0]` già come il rettangolo che contiene
//! tutti i monitor, posizionati come lo sono fisicamente. Il crate Rust
//! `xcap` non ha questa scorciatoia — cattura un monitor alla volta,
//! quindi la composizione va fatta a mano: bounding box di tutti i
//! monitor, poi ognuno incollato alla sua posizione relativa.
//!
//! Ogni scatto è un evento istantaneo (duration=0): il bucket registra
//! solo il nome del file, non l'immagine stessa.

use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration as StdDuration;

use chrono::{DateTime, NaiveDateTime, Utc};
use clap::Parser;
use image::{imageops, ImageBuffer, Rgba, RgbaImage};
use serde_json::{json, Map};
use xcap::Monitor;

const CLIENT_NAME: &str = "aw-watcher-screenshot";
const BUCKET_ID: &str = "aw-watcher-screenshot";
const BUCKET_TYPE: &str = "general.screenshot";

/// Cartella scrivibile dall'utente standard, condivisa con gli altri
/// watcher (app-icons) e con la webui — non la cartella risorse
/// dell'app, tipicamente sotto Program Files e di sola lettura senza
/// elevazione in un'installazione reale (stesso problema, stessa
/// soluzione delle icone, vedi BLUEPRINT.md Fase 4). Il server la serve
/// con `--custom-static app-data=<questa cartella>`, montata su
/// `/pages/app-data/` — gli screenshot finiscono quindi sotto
/// `/pages/app-data/screenshots/<file>`.
fn default_app_data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_default()
        .join("TrackFlow")
        .join("app-data")
}

/// Cattura tutti i monitor collegati e li compone in una singola
/// immagine, ognuno posizionato dove si trova realmente sul desktop
/// virtuale (stesso risultato di mss's monitors[0]).
fn cattura_tutti_i_monitor() -> Result<RgbaImage, String> {
    let monitors = Monitor::all().map_err(|e| e.to_string())?;
    if monitors.is_empty() {
        return Err("Nessun monitor trovato".to_string());
    }

    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;

    let mut catture: Vec<(i32, i32, RgbaImage)> = Vec::new();
    for monitor in &monitors {
        let x = monitor.x().map_err(|e| e.to_string())?;
        let y = monitor.y().map_err(|e| e.to_string())?;
        let img = monitor.capture_image().map_err(|e| e.to_string())?;

        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x + img.width() as i32);
        max_y = max_y.max(y + img.height() as i32);

        catture.push((x, y, img));
    }

    let larghezza_totale = (max_x - min_x) as u32;
    let altezza_totale = (max_y - min_y) as u32;
    let mut combinata: RgbaImage = ImageBuffer::from_pixel(
        larghezza_totale,
        altezza_totale,
        Rgba([0, 0, 0, 255]),
    );

    for (x, y, img) in catture {
        let offset_x = (x - min_x) as i64;
        let offset_y = (y - min_y) as i64;
        imageops::overlay(&mut combinata, &img, offset_x, offset_y);
    }

    Ok(combinata)
}

/// Cattura, ridimensiona/comprime e salva su disco. Ritorna il nome del
/// file salvato (non il percorso intero) e il timestamp dello scatto.
fn cattura_e_salva(
    screenshots_dir: &Path,
    max_width: u32,
    max_height: u32,
    quality: u8,
) -> Result<(String, chrono::DateTime<Utc>), String> {
    let img = cattura_tutti_i_monitor()?;

    // Scala uniforme sul lato più vincolante, non solo la larghezza: due
    // monitor affiancati danno un'immagine molto larga (la larghezza
    // vincola), uno sopra l'altro un'immagine molto alta (l'altezza
    // vincola).
    let scale = (max_width as f64 / img.width() as f64)
        .min(max_height as f64 / img.height() as f64)
        .min(1.0);

    let img = if scale < 1.0 {
        let nuova_larghezza = (img.width() as f64 * scale).round() as u32;
        let nuova_altezza = (img.height() as f64 * scale).round() as u32;
        imageops::resize(&img, nuova_larghezza, nuova_altezza, imageops::FilterType::Lanczos3)
    } else {
        img
    };

    let adesso = Utc::now();
    // Stesso formato Python: "%Y%m%d-%H%M%S-%f"[:-3] + ".jpg" (millisecondi,
    // non microsecondi: i primi 3 delle 6 cifre di %f).
    let filename = format!(
        "{}-{:03}.jpg",
        adesso.format("%Y%m%d-%H%M%S"),
        adesso.timestamp_subsec_millis()
    );

    let rgb_img = image::DynamicImage::ImageRgba8(img).to_rgb8();
    let mut buffer = Cursor::new(Vec::new());
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, quality);
    rgb_img
        .write_with_encoder(encoder)
        .map_err(|e| e.to_string())?;

    std::fs::create_dir_all(screenshots_dir).map_err(|e| e.to_string())?;
    std::fs::write(screenshots_dir.join(&filename), buffer.into_inner())
        .map_err(|e| e.to_string())?;

    Ok((filename, adesso))
}

/// Legge l'intervallo da un file locale scritto dal processo Tauri quando
/// l'utente cambia l'impostazione 'screenshotIntervalSeconds' dalla webui
/// (nessun errore se manca: resta il default da riga di comando) — non
/// più una richiesta di rete a aw-server, coerente col resto di questo
/// watcher (vedi BLUEPRINT.md, Fase 5). Il file viene semplicemente
/// riletto ad ogni giro, stesso costo/comportamento di prima.
fn leggi_intervallo(override_path: &Path, default: u64) -> u64 {
    std::fs::read_to_string(override_path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(default)
}

/// Stesso principio di `leggi_intervallo`: rilegge da un piccolo file
/// locale scritto dal processo Tauri quando l'utente cambia
/// 'screenshotRetentionDays' dalla webui (vedi ScreenshotSettings.vue).
fn leggi_retention_giorni(override_path: &Path, default: u64) -> u64 {
    std::fs::read_to_string(override_path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(default)
}

/// Elimina gli screenshot più vecchi di `retention_days`, leggendo la
/// data/ora direttamente dal NOME del file (che la incorpora già, vedi
/// `cattura_e_salva`) invece che dalla data di modifica sul filesystem —
/// più affidabile: la mtime cambia se il file viene copiato/spostato
/// (come successo migrando lo storico nella nuova cartella scrivibile
/// durante la Fase 5), la data nel nome no. Un file il cui nome non
/// rispetta il formato atteso viene ignorato, non eliminato per errore.
fn pulisci_vecchi_screenshot(screenshots_dir: &Path, retention_days: u64) {
    let soglia = Utc::now() - chrono::Duration::days(retention_days as i64);
    let Ok(entries) = std::fs::read_dir(screenshots_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        // "20260809-214343-439" -> prendiamo solo "20260809-214343"
        // (data+ora, i millisecondi finali non servono per la soglia).
        let Some(data_ora) = stem.get(0..15) else {
            continue;
        };
        let Ok(naive) = NaiveDateTime::parse_from_str(data_ora, "%Y%m%d-%H%M%S") else {
            continue;
        };
        let quando = DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc);
        if quando < soglia {
            let _ = std::fs::remove_file(&path);
        }
    }
}

#[derive(Parser)]
#[command(about = "Watcher per screenshot periodici del desktop")]
struct Args {
    /// Gira in modalità test: porta 5666, dati separati da quelli reali
    #[arg(long)]
    testing: bool,

    /// Ogni quanti secondi catturare uno screenshot (default: 30).
    /// Sovrascritto dalla impostazione 'screenshotIntervalSeconds' su
    /// aw-server, se presente.
    #[arg(long, default_value_t = 30)]
    interval: u64,

    /// Larghezza massima in pixel, ridimensionato mantenendo le proporzioni
    #[arg(long, default_value_t = 1920)]
    max_width: u32,

    /// Altezza massima in pixel, ridimensionato mantenendo le proporzioni
    #[arg(long, default_value_t = 1080)]
    max_height: u32,

    /// Qualità di compressione JPEG, 1-95
    #[arg(long, default_value_t = 70)]
    quality: u8,

    /// Cartella scrivibile condivisa (icone, screenshot, impostazioni) -
    /// default: %LOCALAPPDATA%\TrackFlow\app-data
    #[arg(long)]
    app_data_dir: Option<PathBuf>,

    /// Cartella dove salvare gli screenshot (default: <app-data-dir>/screenshots)
    #[arg(long)]
    screenshots_dir: Option<String>,

    /// Dopo quanti giorni eliminare da soli gli screenshot più vecchi
    /// (default: 14). Sovrascritto dalla impostazione
    /// 'screenshotRetentionDays' sulla webui, se presente.
    #[arg(long, default_value_t = 14)]
    retention_days: u64,
}

fn main() {
    let args = Args::parse();

    let app_data_dir = args.app_data_dir.unwrap_or_else(default_app_data_dir);

    let screenshots_dir = match &args.screenshots_dir {
        Some(dir) => PathBuf::from(dir),
        None => app_data_dir.join("screenshots"),
    };

    let interval_override_file = app_data_dir.join("screenshot-interval-override.txt");
    let retention_override_file = app_data_dir.join("screenshot-retention-days-override.txt");

    println!(
        "Modalità: {}",
        if args.testing { "testing (porta 5666)" } else { "normale (porta 5600)" }
    );
    println!("Cartella screenshot: {}", screenshots_dir.display());
    println!(
        "Intervallo di partenza: {}s (rilette ad ogni giro da '{}' se presente)",
        args.interval,
        interval_override_file.display()
    );
    println!(
        "Conservazione di partenza: {} giorni (rilette ad ogni giro da '{}' se presente)",
        args.retention_days,
        retention_override_file.display()
    );
    println!(
        "Dimensione max: {}x{}px, qualità: {}",
        args.max_width, args.max_height, args.quality
    );

    loop {
        let intervallo = leggi_intervallo(&interval_override_file, args.interval);
        let retention_giorni = leggi_retention_giorni(&retention_override_file, args.retention_days);
        pulisci_vecchi_screenshot(&screenshots_dir, retention_giorni);
        match cattura_e_salva(&screenshots_dir, args.max_width, args.max_height, args.quality) {
            Ok((filename, quando)) => {
                let mut data = Map::new();
                data.insert("filename".to_string(), filename.clone().into());
                let envelope = json!({
                    "bucket_id": BUCKET_ID,
                    "bucket_type": BUCKET_TYPE,
                    "client": CLIENT_NAME,
                    "op": "event",
                    "event": {
                        "timestamp": quando.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                        "duration": 0.0,
                        "data": data,
                    },
                });
                let mut stdout = std::io::stdout();
                let _ = writeln!(stdout, "{envelope}");
                let _ = stdout.flush();
                println!("Screenshot salvato: {filename}");
            }
            Err(e) => {
                // Un singolo scatto fallito (es. schermo bloccato, nessun
                // display disponibile) non deve fermare il watcher.
                eprintln!("Errore durante la cattura: {e}");
            }
        }
        thread::sleep(StdDuration::from_secs(intervallo));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filename_format_matches_python_convention() {
        let dt = chrono::DateTime::parse_from_rfc3339("2026-08-09T15:30:00.123456Z")
            .unwrap()
            .with_timezone(&Utc);
        let filename = format!("{}-{:03}.jpg", dt.format("%Y%m%d-%H%M%S"), dt.timestamp_subsec_millis());
        assert_eq!(filename, "20260809-153000-123.jpg");
    }

    #[test]
    fn leggi_intervallo_falls_back_to_default_when_file_missing() {
        let path = std::env::temp_dir().join(format!("aw-ss-interval-missing-{}.txt", std::process::id()));
        let _ = std::fs::remove_file(&path);
        assert_eq!(leggi_intervallo(&path, 30), 30);
    }

    #[test]
    fn leggi_intervallo_reads_override_when_present() {
        let path = std::env::temp_dir().join(format!("aw-ss-interval-override-{}.txt", std::process::id()));
        std::fs::write(&path, "45\n").unwrap();
        assert_eq!(leggi_intervallo(&path, 30), 45);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn pulisci_vecchi_screenshot_deletes_only_files_older_than_retention() {
        let dir = std::env::temp_dir().join(format!("aw-ss-retention-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let vecchio = dir.join("20200101-120000-000.jpg"); // anni fa
        let recente_name = format!(
            "{}-000.jpg",
            Utc::now().format("%Y%m%d-%H%M%S")
        );
        let recente = dir.join(&recente_name);
        let non_conforme = dir.join("qualcosa-non-standard.jpg");

        std::fs::write(&vecchio, b"x").unwrap();
        std::fs::write(&recente, b"x").unwrap();
        std::fs::write(&non_conforme, b"x").unwrap();

        pulisci_vecchi_screenshot(&dir, 14);

        assert!(!vecchio.exists(), "il file vecchio doveva essere eliminato");
        assert!(recente.exists(), "il file recente NON doveva essere eliminato");
        assert!(non_conforme.exists(), "il file dal nome non standard NON doveva essere toccato");

        std::fs::remove_dir_all(&dir).ok();
    }
}

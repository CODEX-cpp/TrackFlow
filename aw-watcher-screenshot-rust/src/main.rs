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
use xcap::{Monitor, Window};

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

/// Cattura solo la finestra attualmente in primo piano, invece di tutti
/// i monitor uniti — richiesta esplicita dell'utente per privacy (le
/// finestre/schermi non attivi non finiscono mai catturati) e per
/// qualità (una singola finestra normale non soffre del problema di
/// area combinata di più monitor, vedi `scala_per_area`). Se nessuna
/// finestra risulta focalizzata (es. desktop cliccato, tutte le finestre
/// minimizzate), l'errore fa ricadere il chiamante sulla cattura di
/// tutti i monitor (vedi `cattura_e_salva`).
fn cattura_finestra_attiva() -> Result<RgbaImage, String> {
    let windows = Window::all().map_err(|e| e.to_string())?;
    let finestra_attiva = windows
        .into_iter()
        .find(|w| w.is_focused().unwrap_or(false))
        .ok_or_else(|| "Nessuna finestra in primo piano trovata".to_string())?;

    finestra_attiva.capture_image().map_err(|e| e.to_string())
}

/// Modalità di cattura, letta dall'impostazione 'screenshotOnlyActiveWindow'.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Modalita {
    TuttiIMonitor,
    FinestraAttiva,
}

/// Fattore di scala da applicare all'immagine catturata (che con più
/// monitor, o anche un solo monitor molto grande, può essere molto più
/// larga/alta di un singolo schermo normale) per restare dentro un
/// budget di AREA invece che di larghezza/altezza fisse.
///
/// Bug reale segnalato dall'utente: "con più schermi la qualità cala
/// drasticamente, anche solo con un monitor molto grande". La versione
/// precedente scalava sulla dimensione più vincolante tra `max_width` e
/// `max_height` — pensata per UN solo schermo, ma applicata
/// all'immagine COMBINATA di tutti i monitor. Con 3 monitor 1920×1080
/// affiancati (combinata 5760×1080), quel calcolo dava una scala di
/// 1920/5760 ≈ 0,33 — ogni monitor finiva rimpicciolito a soli 640×360,
/// una perdita di dettaglio catastrofica. Un solo monitor 4K/ultrawide
/// molto più largo di 1920px aveva lo stesso problema.
///
/// Qui invece il budget è un'AREA totale (max_width × max_height,
/// default ≈2 megapixel, la stessa "quantità di dettaglio" pensata per
/// uno schermo normale) — la scala è `sqrt(area_massima / area_reale)`
/// perché scalare linearmente entrambe le dimensioni di un fattore `s`
/// scala l'AREA di `s²`, quindi serve la radice quadrata per ottenere
/// esattamente l'area target. Un solo schermo normale non viene
/// toccato (stesso comportamento di prima, `.min(1.0)` non scala mai
/// verso l'alto); con più monitor la qualità cala in modo proporzionato
/// e prevedibile invece che catastrofico — ognuno riceve una fetta equa
/// del budget invece che un terzo (o un quarto, ecc.) della sola
/// larghezza.
fn scala_per_area(larghezza: u32, altezza: u32, max_width: u32, max_height: u32) -> f64 {
    let area_massima = max_width as f64 * max_height as f64;
    let area_reale = larghezza as f64 * altezza as f64;
    (area_massima / area_reale).sqrt().min(1.0)
}

/// Cattura, ridimensiona/comprime e salva su disco. Ritorna il nome del
/// file salvato (non il percorso intero) e il timestamp dello scatto.
fn cattura_e_salva(
    screenshots_dir: &Path,
    max_width: u32,
    max_height: u32,
    quality: u8,
    modalita: Modalita,
) -> Result<(String, chrono::DateTime<Utc>), String> {
    let img = match modalita {
        Modalita::TuttiIMonitor => cattura_tutti_i_monitor()?,
        // Se la finestra attiva non è catturabile (nessuna finestra a
        // fuoco), non facciamo fallire lo scatto: ricadiamo su tutti i
        // monitor, coerente col comportamento di sempre.
        Modalita::FinestraAttiva => {
            cattura_finestra_attiva().or_else(|_| cattura_tutti_i_monitor())?
        }
    };

    let scale = scala_per_area(img.width(), img.height(), max_width, max_height);

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

/// Stesso principio di `leggi_intervallo`: rilegge da un piccolo file
/// locale scritto dal processo Tauri quando l'utente cambia
/// 'screenshotOnlyActiveWindow' dalla webui (vedi ScreenshotSettings.vue).
/// Convenzione: il file contiene la stringa letterale "true" quando
/// l'opzione è attiva, qualsiasi altro contenuto (o file mancante)
/// equivale a "false" (comportamento di sempre, tutti i monitor).
fn leggi_modalita(override_path: &Path) -> Modalita {
    let attiva = std::fs::read_to_string(override_path)
        .map(|s| s.trim() == "true")
        .unwrap_or(false);
    if attiva {
        Modalita::FinestraAttiva
    } else {
        Modalita::TuttiIMonitor
    }
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

    /// Larghezza di riferimento per il budget di area (vedi
    /// scala_per_area) — non un limite rigido di larghezza, l'immagine
    /// combinata di più monitor può restare più larga di questo se
    /// l'altezza compensa. Alzata da 1920 a 2560 su richiesta esplicita
    /// dell'utente: a 1920×1080 (~2 megapixel) il testo di dimensione
    /// media negli screenshot non si leggeva bene nemmeno su un solo
    /// schermo normale.
    #[arg(long, default_value_t = 2560)]
    max_width: u32,

    /// Altezza di riferimento per il budget di area — vedi max_width.
    #[arg(long, default_value_t = 1440)]
    max_height: u32,

    /// Qualità di compressione JPEG, 1-95 — alzata da 70 a 85 insieme al
    /// budget di area sopra, stesso motivo (testo poco leggibile).
    #[arg(long, default_value_t = 85)]
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
    let modalita_override_file = app_data_dir.join("screenshot-mode-override.txt");

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
    println!(
        "Modalità cattura: rilette ad ogni giro da '{}' se presente (default: tutti i monitor)",
        modalita_override_file.display()
    );

    loop {
        let intervallo = leggi_intervallo(&interval_override_file, args.interval);
        let retention_giorni = leggi_retention_giorni(&retention_override_file, args.retention_days);
        let modalita = leggi_modalita(&modalita_override_file);
        pulisci_vecchi_screenshot(&screenshots_dir, retention_giorni);
        match cattura_e_salva(&screenshots_dir, args.max_width, args.max_height, args.quality, modalita) {
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
    fn scala_per_area_non_tocca_un_singolo_schermo_normale() {
        // Un solo monitor, esattamente al budget massimo — nessuna scala.
        assert_eq!(scala_per_area(1920, 1080, 1920, 1080), 1.0);
    }

    #[test]
    fn scala_per_area_non_ingrandisce_mai() {
        // Immagine più piccola del budget — `.min(1.0)` non deve mai far
        // ingrandire (bug facile da introdurre invertendo la formula).
        assert_eq!(scala_per_area(800, 600, 1920, 1080), 1.0);
    }

    #[test]
    fn scala_per_area_tre_monitor_affiancati() {
        // Bug reale segnalato dall'utente: 3 monitor 1920×1080 affiancati
        // (combinata 5760×1080) — la vecchia scala sulla larghezza dava
        // 1920/5760 ≈ 0,333 (ogni monitor a 640×360, pessimo). Con la
        // scala per area, ogni monitor riceve una fetta equa del budget:
        // sqrt(2073600 / 6220800) ≈ 0,577 (ogni monitor ≈ 1109×624,
        // molto meglio).
        let scala = scala_per_area(5760, 1080, 1920, 1080);
        assert!((scala - 0.5774).abs() < 0.001, "scala inattesa: {scala}");
    }

    #[test]
    fn scala_per_area_singolo_monitor_4k() {
        // Un solo monitor 4K (3840×2160, area 4× quella target) — anche
        // senza monitor multipli, lo stesso problema si presentava per
        // uno schermo molto grande. sqrt(1/4) = 0,5 esatto.
        assert_eq!(scala_per_area(3840, 2160, 1920, 1080), 0.5);
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
    fn leggi_modalita_falls_back_to_tutti_i_monitor_when_file_missing() {
        let path = std::env::temp_dir().join(format!("aw-ss-modalita-missing-{}.txt", std::process::id()));
        let _ = std::fs::remove_file(&path);
        assert_eq!(leggi_modalita(&path), Modalita::TuttiIMonitor);
    }

    #[test]
    fn leggi_modalita_reads_finestra_attiva_when_true() {
        let path = std::env::temp_dir().join(format!("aw-ss-modalita-true-{}.txt", std::process::id()));
        std::fs::write(&path, "true\n").unwrap();
        assert_eq!(leggi_modalita(&path), Modalita::FinestraAttiva);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn leggi_modalita_reads_tutti_i_monitor_when_false() {
        let path = std::env::temp_dir().join(format!("aw-ss-modalita-false-{}.txt", std::process::id()));
        std::fs::write(&path, "false\n").unwrap();
        assert_eq!(leggi_modalita(&path), Modalita::TuttiIMonitor);
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

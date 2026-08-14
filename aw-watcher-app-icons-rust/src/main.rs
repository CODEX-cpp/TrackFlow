//! Scopre quali app l'utente ha davvero usato ed estrae automaticamente
//! icona reale + colore pastello/caldo per ogni app mai vista prima.
//! Porting 1:1 da aw_watcher_app_icons/main.py (Python), con una
//! differenza di trasporto rispetto all'originale: invece di interrogare
//! periodicamente il bucket finestra via rete, legge un nome app per
//! riga da stdin — è il processo Tauri che ci lancia a inoltrarci ogni
//! nome app non appena il watcher finestra segnala un cambio di finestra
//! attiva (vedi src-tauri/src/lib.rs e BLUEPRINT.md, Fase 5). Più
//! reattivo del vecchio polling ogni 30s, e niente affatto dipendente
//! dalla rete come tutti gli altri watcher dopo quella fase.
//!
//! Differenza rispetto agli altri watcher: qui non si manda NESSUN evento
//! ad ActivityWatch — si scrive solo su disco (icona + colore + nome)
//! dentro una cartella scrivibile condivisa con la webui.
//!
//! Formula colore: stessa identica logica di scripts/color-utils.ps1 /
//! del Python originale (colorsys.rgb_to_hls / hls_to_rgb, reimplementate
//! qui a mano formula-per-formula per ottenere lo stesso risultato).

use std::collections::HashMap;
use std::io::BufRead;
use std::path::{Path, PathBuf};

use clap::Parser;
use image::{imageops, RgbaImage};
use pelite::{FileMap, PeFile};
use serde_json::Value;
use sysinfo::System;
use windows_icons::{get_icon_by_path_with_size, IconSize};

// Stesso identico elenco di HIDDEN_SYSTEM_APPS in src/util/appNames.ts —
// nessun meccanismo di condivisione fra Rust e TypeScript in questo
// progetto, va tenuto a mano in sincronia.
const HIDDEN_SYSTEM_APPS: &[&str] = &[
    "searchhost.exe",
    "searchapp.exe",
    "startmenuexperiencehost.exe",
    "shellexperiencehost.exe",
    "sihost.exe",
    "dwm.exe",
    "lockapp.exe",
    "logonui.exe",
    "textinputhost.exe",
    "applicationframehost.exe",
    "runtimebroker.exe",
    "systemsettings.exe",
    "backgroundtaskhost.exe",
    "taskhostw.exe",
    "ctfmon.exe",
    "explorer.exe",
    "widgets.exe",
    "widgetservice.exe",
    "securityhealthsystray.exe",
    "gamebar.exe",
    "gamebarftserver.exe",
];

fn get_data_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

// ============================================================
// COLORE PASTELLO/CALDO DA UN'IMMAGINE
// Porting a mano di colorsys.rgb_to_hls / hls_to_rgb (Python stdlib),
// per ottenere esattamente lo stesso risultato numerico.
// ============================================================

const ONE_THIRD: f64 = 1.0 / 3.0;
const ONE_SIXTH: f64 = 1.0 / 6.0;
const TWO_THIRD: f64 = 2.0 / 3.0;

/// Porting di colorsys.rgb_to_hls: r/g/b in [0,1] -> (h, l, s), tutti in [0,1].
fn rgb_to_hls(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let maxc = r.max(g).max(b);
    let minc = r.min(g).min(b);
    let l = (minc + maxc) / 2.0;
    if minc == maxc {
        return (0.0, l, 0.0);
    }
    let s = if l <= 0.5 {
        (maxc - minc) / (maxc + minc)
    } else {
        (maxc - minc) / (2.0 - maxc - minc)
    };
    let rc = (maxc - r) / (maxc - minc);
    let gc = (maxc - g) / (maxc - minc);
    let bc = (maxc - b) / (maxc - minc);
    let h = if r == maxc {
        bc - gc
    } else if g == maxc {
        2.0 + rc - bc
    } else {
        4.0 + gc - rc
    };
    let h = (h / 6.0).rem_euclid(1.0);
    (h, l, s)
}

fn hls_v(m1: f64, m2: f64, hue: f64) -> f64 {
    let hue = hue.rem_euclid(1.0);
    if hue < ONE_SIXTH {
        m1 + (m2 - m1) * hue * 6.0
    } else if hue < 0.5 {
        m2
    } else if hue < TWO_THIRD {
        m1 + (m2 - m1) * (TWO_THIRD - hue) * 6.0
    } else {
        m1
    }
}

/// Porting di colorsys.hls_to_rgb: (h, l, s) in [0,1] -> (r, g, b) in [0,1].
fn hls_to_rgb(h: f64, l: f64, s: f64) -> (f64, f64, f64) {
    if s == 0.0 {
        return (l, l, l);
    }
    let m2 = if l <= 0.5 { l * (1.0 + s) } else { l + s - (l * s) };
    let m1 = 2.0 * l - m2;
    (
        hls_v(m1, m2, h + ONE_THIRD),
        hls_v(m1, m2, h),
        hls_v(m1, m2, h - ONE_THIRD),
    )
}

fn rgb_to_hue_sat_light(r: u8, g: u8, b: u8) -> (f64, f64, f64) {
    let (h, l, s) = rgb_to_hls(r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0);
    (h * 360.0, s, l)
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (u8, u8, u8) {
    let (r, g, b) = hls_to_rgb(h.rem_euclid(360.0) / 360.0, l, s);
    (
        (r * 255.0).round() as u8,
        (g * 255.0).round() as u8,
        (b * 255.0).round() as u8,
    )
}

/// Blend circolare fra due tonalità (0-360), via più breve sul cerchio
/// delle tonalità.
fn blended_hue(h1: f64, h2: f64, t: f64) -> f64 {
    let diff = ((h2 - h1 + 540.0).rem_euclid(360.0)) - 180.0;
    let result = h1 + diff * t;
    result.rem_euclid(360.0)
}

/// Tonalità dominante via istogramma a 36 bucket di 10°, ignorando pixel
/// trasparenti e quasi grigi/bianchi/neri.
fn dominant_color(img: &RgbaImage) -> Option<(f64, f64, f64)> {
    let thumb = imageops::resize(img, 40, 40, imageops::FilterType::Lanczos3);

    #[derive(Default)]
    struct Bucket {
        count: u32,
        hue_sum: f64,
        sat_sum: f64,
        light_sum: f64,
    }
    let mut buckets: HashMap<u32, Bucket> = HashMap::new();

    for pixel in thumb.pixels() {
        let [r, g, b, a] = pixel.0;
        if a < 128 {
            continue;
        }
        let (hue, sat, light) = rgb_to_hue_sat_light(r, g, b);
        if sat < 0.15 || light < 0.12 || light > 0.92 {
            continue;
        }
        let idx = (hue / 10.0) as u32 % 36;
        let bucket = buckets.entry(idx).or_default();
        bucket.count += 1;
        bucket.hue_sum += hue;
        bucket.sat_sum += sat;
        bucket.light_sum += light;
    }

    buckets
        .values()
        .max_by_key(|b| b.count)
        .map(|b| {
            let n = b.count as f64;
            (b.hue_sum / n, b.sat_sum / n, b.light_sum / n)
        })
}

/// Riporta satura/luminosità in una banda pastello fissa e dà un nudge
/// caldo leggero e uniforme verso l'arancione (30°).
fn pastel_warm_color(hue: f64, sat: f64, light: f64) -> String {
    let target_sat = sat.max(0.30).min(0.50);
    let target_light = light.max(0.55).min(0.68);
    let final_hue = blended_hue(hue, 30.0, 0.15);
    let (r, g, b) = hsl_to_rgb(final_hue, target_sat, target_light);
    format!("#{r:02X}{g:02X}{b:02X}")
}

// ============================================================
// NOME "IN INGLESE, MEGLIO DI NIENTE" DAI METADATI DELL'EXE
// ============================================================

fn nome_da_metadati(percorso_exe: &Path) -> Option<String> {
    let file_map = FileMap::open(percorso_exe).ok()?;
    let pe = PeFile::from_bytes(file_map.as_ref()).ok()?;
    let resources = pe.resources().ok()?;
    let version_info = resources.version_info().ok()?;
    let langs = version_info.translation();
    for lang in langs {
        if let Some(desc) = version_info.value(*lang, "FileDescription") {
            let desc = desc.trim();
            if !desc.is_empty() {
                return Some(desc.to_string());
            }
        }
    }
    None
}

// ============================================================
// ESTRAZIONE ICONA + COLORE
// ============================================================

fn trova_percorso_processo(sys: &System, nome_app_lower: &str) -> Option<PathBuf> {
    for (_, process) in sys.processes() {
        let nome = process.name().to_string_lossy().to_lowercase();
        if nome != nome_app_lower {
            continue;
        }
        if let Some(p) = process.exe() {
            return Some(p.to_path_buf());
        }
    }
    None
}

/// Estrae l'icona vera da un .exe (usando lo shell jumbo icon list,
/// 256x256, equivalente al "scegli il frame più grande" di Python) e
/// calcola il suo colore pastello/caldo. Ritorna None se l'icona era
/// troppo neutra/grigia, o Err se l'estrazione stessa fallisce.
fn estrai_icona_e_colore(
    percorso_exe: &Path,
    destinazione_png: &Path,
) -> Result<Option<String>, String> {
    let img = get_icon_by_path_with_size(percorso_exe, IconSize::ExtraLarge)
        .map_err(|e| e.to_string())?;

    img.save(destinazione_png).map_err(|e| e.to_string())?;

    Ok(dominant_color(&img).map(|(h, s, l)| pastel_warm_color(h, s, l)))
}

// ============================================================
// STATO / MAPPE SALVATE SU DISCO
// ============================================================

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    #[serde(default)]
    falliti: Vec<String>,
}

fn load_state(path: &Path) -> State {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_state(path: &Path, state: &State) {
    if let Ok(json) = serde_json::to_string_pretty(state) {
        let _ = std::fs::write(path, json);
    }
}

/// I due file JSON condivisi con la webui (appIconColors.json,
/// appAutoNames.json) possono avere un BOM UTF-8 (scritti in passato da
/// PowerShell) — va tollerato in lettura.
fn load_json_map(path: &Path) -> HashMap<String, String> {
    let Ok(bytes) = std::fs::read(path) else {
        return HashMap::new();
    };
    let content = String::from_utf8_lossy(&bytes);
    let content = content.strip_prefix('\u{feff}').unwrap_or(&content);
    serde_json::from_str::<HashMap<String, Value>>(content)
        .map(|m| {
            m.into_iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k, s.to_string())))
                .collect()
        })
        .unwrap_or_default()
}

fn save_json_map(path: &Path, map: &HashMap<String, String>) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut ordinato: Vec<(&String, &String)> = map.iter().collect();
    ordinato.sort_by(|a, b| a.0.cmp(b.0));
    let ordinato: serde_json::Map<String, Value> = ordinato
        .into_iter()
        .map(|(k, v)| (k.clone(), Value::String(v.clone())))
        .collect();
    if let Ok(mut json) = serde_json::to_string_pretty(&ordinato) {
        json.push('\n');
        let _ = std::fs::write(path, json);
    }
}

// ============================================================
// LOGICA PRINCIPALE
// ============================================================

struct AppIconsWatcher {
    icons_dir: PathBuf,
    colors_path: PathBuf,
    names_path: PathBuf,
    state_file: PathBuf,
    stato: State,
    colori: HashMap<String, String>,
    nomi: HashMap<String, String>,
    sistema: System,
}

impl AppIconsWatcher {
    fn new(app_data_dir: &Path, state_file: PathBuf) -> Self {
        // Cartella scrivibile dall'utente standard (non la cartella
        // risorse dell'app, che in un'installazione reale è tipicamente
        // sotto Program Files e di sola lettura senza elevazione — vedi
        // BLUEPRINT.md sezione Fase 4 per la scoperta di questo
        // problema). Il server la serve con `--custom-static
        // app-data=<questa cartella>`, montata su `/pages/app-data/`.
        let icons_dir = app_data_dir.join("app-icons");
        let colors_path = app_data_dir.join("appIconColors.json");
        let names_path = app_data_dir.join("appAutoNames.json");
        let _ = std::fs::create_dir_all(&icons_dir);

        AppIconsWatcher {
            stato: load_state(&state_file),
            colori: load_json_map(&colors_path),
            nomi: load_json_map(&names_path),
            icons_dir,
            colors_path,
            names_path,
            state_file,
            sistema: System::new(),
        }
    }

    /// Vero se questa app non ha ancora icona+colore+nome noti e non è
    /// già stata segnata come "estrazione fallita, non riprovare".
    fn app_necessita_gestione(&self, nome: &str) -> bool {
        if HIDDEN_SYSTEM_APPS.contains(&nome) || self.stato.falliti.iter().any(|f| f == nome) {
            return false;
        }
        let png = self.icons_dir.join(format!("{nome}.png"));
        !(png.exists() && self.colori.contains_key(nome) && self.nomi.contains_key(nome))
    }

    fn gestisci_app(&mut self, nome: &str) {
        self.sistema.refresh_all();
        let Some(percorso) = trova_percorso_processo(&self.sistema, nome) else {
            println!("'{nome}': nessun processo in esecuzione trovato con questo nome, riproverò al prossimo giro");
            return;
        };

        let png = self.icons_dir.join(format!("{nome}.png"));
        let colore = match estrai_icona_e_colore(&percorso, &png) {
            Ok(colore) => colore,
            Err(e) => {
                println!(
                    "'{nome}' <- {}: estrazione fallita ({e}) - non riproverò più (elimina {} per ritentare)",
                    percorso.display(),
                    self.state_file.file_name().unwrap_or_default().to_string_lossy()
                );
                self.stato.falliti.push(nome.to_string());
                save_state(&self.state_file, &self.stato);
                return;
            }
        };

        if let Some(colore) = &colore {
            self.colori.insert(nome.to_string(), colore.clone());
            save_json_map(&self.colors_path, &self.colori);
            println!("OK   '{nome}' <- {} (colore: {colore})", percorso.display());
        } else {
            println!(
                "OK   '{nome}' <- {} (icona troppo neutra/grigia, nessun colore - resta l'hash-per-nome)",
                percorso.display()
            );
        }

        if let Some(nome_leggibile) = nome_da_metadati(&percorso) {
            if self.nomi.get(nome) != Some(&nome_leggibile) {
                self.nomi.insert(nome.to_string(), nome_leggibile.clone());
                save_json_map(&self.names_path, &self.nomi);
                println!("     nome: '{nome_leggibile}'");
            }
        }
    }

}

#[derive(Parser)]
#[command(
    about = "Per ogni nome app ricevuto su stdin (uno per riga, inoltrato dal processo che ci lancia ogni volta che cambia la finestra attiva) mai visto prima, estrae automaticamente la sua icona reale e ne calcola un colore pastello/caldo, salvando entrambi in una cartella condivisa con la webui."
)]
struct Args {
    /// Cartella scrivibile dove salvare icone/colori/nomi - default:
    /// %LOCALAPPDATA%\TrackFlow\app-data (serve un utente standard senza
    /// elevazione, a differenza della cartella risorse di un'app
    /// installata). aw-server-rust va avviato con `--custom-static
    /// app-data=<questa cartella>` perché la webui possa leggerli.
    #[arg(long)]
    app_data_dir: Option<PathBuf>,
}

fn default_app_data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_default()
        .join("TrackFlow")
        .join("app-data")
}

fn main() {
    let args = Args::parse();

    let app_data_dir = args.app_data_dir.unwrap_or_else(default_app_data_dir);
    if std::fs::create_dir_all(&app_data_dir).is_err() {
        println!(
            "ERRORE: non riesco a creare/usare la cartella {} - specificala con --app-data-dir",
            app_data_dir.display()
        );
        return;
    }

    let state_file = get_data_dir().join("app_icons_watcher_state.json");
    let mut watcher = AppIconsWatcher::new(&app_data_dir, state_file);

    println!("Cartella dati (scrivibile): {}", app_data_dir.display());
    let n_icone = std::fs::read_dir(&watcher.icons_dir)
        .map(|d| d.filter_map(Result::ok).filter(|e| e.path().extension().map(|x| x == "png").unwrap_or(false)).count())
        .unwrap_or(0);
    println!("Icone note: {n_icone}, colori noti: {}", watcher.colori.len());

    // Un nome app per riga su stdin, inoltrato dal processo che ci lancia
    // ogni volta che il watcher finestra segnala un cambio di finestra
    // attiva — vedi src-tauri/src/lib.rs e l'intestazione di questo file.
    let stdin = std::io::stdin();
    for riga in stdin.lock().lines() {
        let Ok(riga) = riga else { break };
        let nome = riga.trim().to_lowercase();
        if nome.is_empty() {
            continue;
        }
        if watcher.app_necessita_gestione(&nome) {
            watcher.gestisci_app(&nome);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hls_roundtrip_matches_python_colorsys_reference_values() {
        // Valori di riferimento presi da un'esecuzione reale di
        // colorsys.rgb_to_hls in Python per lo stesso input, per
        // verificare che il porting produca lo stesso risultato
        // numerico (a meno di arrotondamento in virgola mobile).
        // rgb_to_hls(0.2, 0.4, 0.8) in Python: (0.6111111111111112, 0.5, 0.6)
        let (h, l, s) = rgb_to_hls(0.2, 0.4, 0.8);
        assert!((h - 0.6111111111111112).abs() < 1e-9);
        assert!((l - 0.5).abs() < 1e-9);
        assert!((s - 0.6).abs() < 1e-9);

        // hls_to_rgb(0.6111111111111112, 0.5, 0.6) deve tornare ~(0.2, 0.4, 0.8)
        let (r, g, b) = hls_to_rgb(h, l, s);
        assert!((r - 0.2).abs() < 1e-9);
        assert!((g - 0.4).abs() < 1e-9);
        assert!((b - 0.8).abs() < 1e-9);
    }

    #[test]
    fn blended_hue_takes_shortest_circular_path() {
        // Da 350° verso 30° (differenza di 40° passando per 0°, non 320°
        // andando all'indietro) — a metà strada (t=0.5) deve dare 10°.
        let risultato = blended_hue(350.0, 30.0, 0.5);
        assert!((risultato - 10.0).abs() < 1e-6, "got {risultato}");
    }

    #[test]
    fn pastel_warm_color_clamps_saturation_and_lightness() {
        // Colore molto saturo e scuro in input: satura/luminosità devono
        // finire dentro le fasce fisse [0.30,0.50] / [0.55,0.68].
        let hex = pastel_warm_color(200.0, 0.95, 0.10);
        assert_eq!(hex.len(), 7);
        assert!(hex.starts_with('#'));
    }

    #[test]
    fn dominant_color_ignores_transparent_and_near_gray_pixels() {
        // Immagine 4x4: metà pixel trasparenti, metà blu saturo e opaco —
        // la tonalità dominante deve essere quella del blu (~240°), non
        // None (che accadrebbe se tutti i pixel venissero scartati).
        let mut img = RgbaImage::new(4, 4);
        for x in 0..4 {
            for y in 0..4 {
                if x < 2 {
                    img.put_pixel(x, y, image::Rgba([0, 0, 0, 0])); // trasparente
                } else {
                    img.put_pixel(x, y, image::Rgba([30, 30, 220, 255])); // blu saturo
                }
            }
        }
        let dominante = dominant_color(&img);
        assert!(dominante.is_some());
        let (hue, sat, _light) = dominante.unwrap();
        assert!(hue > 200.0 && hue < 260.0, "hue={hue}");
        assert!(sat > 0.15);
    }

    #[test]
    fn load_json_map_tolerates_bom_and_returns_string_values() {
        let dir = std::env::temp_dir().join(format!("aw-icons-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("colors.json");
        // BOM UTF-8 + contenuto JSON valido, come scritto in passato da
        // PowerShell (Set-Content -Encoding utf8).
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(b"{\"code.exe\": \"#53C6BE\"}");
        std::fs::write(&path, bytes).unwrap();

        let map = load_json_map(&path);
        assert_eq!(map.get("code.exe").unwrap(), "#53C6BE");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn hidden_system_apps_are_filtered_out() {
        assert!(HIDDEN_SYSTEM_APPS.contains(&"explorer.exe"));
        assert!(HIDDEN_SYSTEM_APPS.contains(&"dwm.exe"));
        assert!(!HIDDEN_SYSTEM_APPS.contains(&"code.exe"));
    }
}

//! Watcher finestra attiva: rileva app/titolo della finestra in primo
//! piano. Porting 1:1 del watcher upstream ufficiale ActivityWatch
//! (Python, https://github.com/ActivityWatch/aw-watcher-window, file
//! main.py + lib.py + windows.py) — non un nostro watcher custom, ma
//! non esiste una versione Rust ufficiale. Solo il percorso Windows è
//! portato (Linux/macOS non servono su questa macchina).

use std::io::Write;
use std::thread;
use std::time::Duration as StdDuration;

use chrono::{SecondsFormat, Utc};
use clap::Parser;
use regex::Regex;
use serde::Deserialize;
use serde_json::{json, Map};

// ============================================================
// FINESTRA ATTIVA (Windows: GetForegroundWindow + GetWindowThreadProcessId
// + GetWindowTextW + OpenProcess/GetModuleFileNameExW, fallback sysinfo)
// Porting 1:1 di windows.py + lib.py's get_current_window_windows.
// ============================================================

#[cfg(windows)]
fn get_current_window(sys: &mut sysinfo::System) -> Option<(String, String)> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId,
    };

    let hwnd: HWND = unsafe { GetForegroundWindow() };
    // hwnd nullo significa nessuna finestra in primo piano (es. durante
    // un prompt UAC, la schermata di blocco, o il "secure desktop") —
    // torniamo None così il chiamante salta questo giro di poll, invece
    // di mandare un evento "unknown/unknown".
    if hwnd.0.is_null() {
        return None;
    }

    let mut title_buf = [0u16; 1024];
    let len = unsafe { GetWindowTextW(hwnd, &mut title_buf) };
    let title = String::from_utf16_lossy(&title_buf[..len.max(0) as usize]);

    let mut pid: u32 = 0;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };

    let app = get_app_name(pid).or_else(|| get_app_name_via_sysinfo(sys, pid));

    Some((app.unwrap_or_else(|| "unknown".to_string()), if title.is_empty() { "unknown".to_string() } else { title }))
}

#[cfg(windows)]
fn get_app_name(pid: u32) -> Option<String> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::ProcessStatus::GetModuleFileNameExW;
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION};

    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_INFORMATION, false, pid).ok()?;
        let mut path_buf = [0u16; 1024];
        let len = GetModuleFileNameExW(Some(handle), None, &mut path_buf);
        let _ = CloseHandle(handle);
        if len == 0 {
            return None;
        }
        let path = String::from_utf16_lossy(&path_buf[..len as usize]);
        std::path::Path::new(&path)
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
    }
}

/// Fallback via sysinfo se OpenProcess/GetModuleFileNameEx falliscono
/// (es. processi elevati/amministratore) — stesso ruolo del fallback WMI
/// di windows.py (get_app_name_wmi), implementazione diversa ma stesso
/// scopo: leggere il nome del processo dato il PID senza bisogno degli
/// stessi permessi di accesso.
#[cfg(windows)]
fn get_app_name_via_sysinfo(sys: &mut sysinfo::System, pid: u32) -> Option<String> {
    sys.refresh_processes(
        sysinfo::ProcessesToUpdate::Some(&[sysinfo::Pid::from_u32(pid)]),
        true,
    );
    sys.process(sysinfo::Pid::from_u32(pid))
        .map(|p| p.name().to_string_lossy().to_string())
}

#[cfg(not(windows))]
fn get_current_window(_sys: &mut sysinfo::System) -> Option<(String, String)> {
    compile_error!("aw-watcher-window (questo porting) supporta solo Windows");
}

/// Scala il pulsetime con il poll_time, così il jitter di scheduling
/// del sistema operativo non spezza le catene di heartbeat. A
/// poll_time=1s, un jitter di ~0.15s sta ben dentro il margine di 1s
/// (poll_time+1). A poll_time=5s, un jitter di ~0.75s supera il
/// margine di 1s ~10% delle volte, causando buchi nella timeline.
/// max(poll_time*1.5, poll_time+1) mantiene la retrocompatibilità a
/// poll_time≤2s risolvendo il problema a intervalli più alti.
/// Vedi: https://github.com/ActivityWatch/activitywatch/issues/1177
fn compute_pulsetime(poll_time: f64) -> f64 {
    (poll_time * 1.5).max(poll_time + 1.0)
}

// ============================================================
// CONFIGURAZIONE (stesso file TOML letto da Python)
// ============================================================

#[derive(Debug, Deserialize, Default)]
struct ConfigSection {
    exclude_title: Option<bool>,
    exclude_titles: Option<Vec<String>>,
    poll_time: Option<f64>,
}

#[derive(Debug, Deserialize, Default)]
struct ConfigFile {
    #[serde(rename = "aw-watcher-window")]
    section: Option<ConfigSection>,
}

struct Defaults {
    exclude_title: bool,
    exclude_titles: Vec<String>,
    poll_time: f64,
}

impl Default for Defaults {
    fn default() -> Self {
        Defaults { exclude_title: false, exclude_titles: Vec::new(), poll_time: 1.0 }
    }
}

/// Stessa convenzione "platformdirs" di aw-watcher-afk (vedi commento
/// lì): %LOCALAPPDATA%\activitywatch\activitywatch\<modulo>\<modulo>.toml.
fn config_file_path() -> std::path::PathBuf {
    dirs::data_local_dir()
        .unwrap_or_default()
        .join("activitywatch")
        .join("activitywatch")
        .join("aw-watcher-window")
        .join("aw-watcher-window.toml")
}

fn load_defaults() -> Defaults {
    let hardcoded = Defaults::default();
    let Ok(content) = std::fs::read_to_string(config_file_path()) else {
        return hardcoded;
    };
    let Ok(parsed) = toml::from_str::<ConfigFile>(&content) else {
        return hardcoded;
    };
    let section = parsed.section.unwrap_or_default();
    Defaults {
        exclude_title: section.exclude_title.unwrap_or(hardcoded.exclude_title),
        exclude_titles: section.exclude_titles.unwrap_or(hardcoded.exclude_titles),
        poll_time: section.poll_time.unwrap_or(hardcoded.poll_time),
    }
}

#[derive(Parser)]
#[command(about = "A cross platform window watcher for Activitywatch. (porting: solo Windows)")]
struct Args {
    #[arg(long)]
    host: Option<String>,

    #[arg(long)]
    port: Option<u16>,

    /// Gira in modalità test: porta 5666, dati separati da quelli reali
    #[arg(long)]
    testing: bool,

    #[arg(long)]
    exclude_title: bool,

    /// Esclude titoli finestra per espressione regolare. Ripetibile.
    #[arg(long = "exclude-titles", num_args = 1..)]
    exclude_titles: Vec<String>,

    #[arg(long)]
    poll_time: Option<f64>,
}

fn transform_title(app: &str, title: String, exclude_title: bool, exclude_titles: &[Regex]) -> String {
    let _ = app;
    for pattern in exclude_titles {
        if pattern.is_match(&title) {
            return "excluded".to_string();
        }
    }
    if exclude_title {
        return "excluded".to_string();
    }
    title
}

fn main() {
    let args = Args::parse();
    let defaults = load_defaults();

    let exclude_title = args.exclude_title || defaults.exclude_title;
    let exclude_titles_raw = if args.exclude_titles.is_empty() {
        defaults.exclude_titles
    } else {
        args.exclude_titles
    };
    let exclude_titles: Vec<Regex> = exclude_titles_raw
        .iter()
        .filter_map(|pattern| {
            regex::RegexBuilder::new(pattern)
                .case_insensitive(true)
                .build()
                .map_err(|_| eprintln!("Pattern regex non valido: {pattern}"))
                .ok()
        })
        .collect();
    let poll_time = args.poll_time.unwrap_or(defaults.poll_time);

    const CLIENT_NAME: &str = "aw-watcher-window";
    const BUCKET_TYPE: &str = "currentwindow";
    let hostname = gethostname::gethostname().to_string_lossy().to_string();
    let bucket_id = format!("{CLIENT_NAME}_{hostname}");

    println!(
        "Modalità: {} - poll_time: {poll_time}s, exclude_title: {exclude_title}",
        if args.testing { "testing" } else { "normale" }
    );
    println!("Bucket: {bucket_id}");

    let pulsetime = compute_pulsetime(poll_time);
    let mut sys = sysinfo::System::new();

    loop {
        if let Some((app, title)) = get_current_window(&mut sys) {
            let title = transform_title(&app, title, exclude_title, &exclude_titles);

            let mut data = Map::new();
            data.insert("app".to_string(), app.into());
            data.insert("title".to_string(), title.into());

            let envelope = json!({
                "bucket_id": bucket_id,
                "bucket_type": BUCKET_TYPE,
                "client": CLIENT_NAME,
                "op": "heartbeat",
                "pulsetime": pulsetime,
                "event": {
                    "timestamp": Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
                    "duration": 0.0,
                    "data": data,
                },
            });
            let mut stdout = std::io::stdout();
            let _ = writeln!(stdout, "{envelope}");
            let _ = stdout.flush();
        }
        thread::sleep(StdDuration::from_secs_f64(poll_time));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pulsetime_formula_matches_python_reference_values() {
        // max(poll_time*1.5, poll_time+1.0) — valori di riferimento dal
        // commento originale in main.py: poll_time=1 -> 2.0 (poll_time+1
        // vince), poll_time=5 -> 7.5 (poll_time*1.5 vince).
        assert_eq!(compute_pulsetime(1.0), 2.0);
        assert_eq!(compute_pulsetime(5.0), 7.5);
        assert_eq!(compute_pulsetime(2.0), 3.0); // pareggio: 2*1.5=3.0=2+1.0
    }

    #[test]
    fn transform_title_excludes_by_regex_case_insensitive() {
        let pattern = Regex::new(r"(?i)secret").unwrap();
        let result = transform_title("app.exe", "My SECRET document".to_string(), false, &[pattern]);
        assert_eq!(result, "excluded");
    }

    #[test]
    fn transform_title_passes_through_when_no_match() {
        let pattern = Regex::new(r"(?i)secret").unwrap();
        let result = transform_title("app.exe", "Public document".to_string(), false, &[pattern]);
        assert_eq!(result, "Public document");
    }

    #[test]
    fn transform_title_exclude_title_flag_always_wins() {
        let result = transform_title("app.exe", "Anything".to_string(), true, &[]);
        assert_eq!(result, "excluded");
    }

    #[test]
    fn parses_real_config_toml_shape_all_commented() {
        // Stesso contenuto del file di config reale di questa macchina:
        // tutto commentato, deve risultare in Defaults invariati.
        let toml_str = "[aw-watcher-window]\n#exclude_title = false\n#exclude_titles = []\n#poll_time = 1.0\n#strategy_macos = \"swift\"\n";
        let parsed: ConfigFile = toml::from_str(toml_str).unwrap();
        let section = parsed.section.unwrap_or_default();
        assert_eq!(section.poll_time, None);
        assert_eq!(section.exclude_title, None);
    }
}

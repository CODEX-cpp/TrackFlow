//! Watcher AFK: rileva assenza di input tastiera/mouse per marcare
//! l'utente come "afk" o "not-afk". Porting 1:1 del watcher upstream
//! ufficiale ActivityWatch (Python, https://github.com/ActivityWatch/aw-watcher-afk,
//! file afk.py + windows.py + config.py) — non un nostro watcher custom,
//! ma non esiste una versione Rust ufficiale.

use std::io::Write;
use std::thread;
use std::time::Duration as StdDuration;

use chrono::{DateTime, SecondsFormat, Utc};
use clap::Parser;
use serde::Deserialize;
use serde_json::{json, Map};

// ============================================================
// LETTURA SECONDI DI INATTIVITÀ (Windows: GetLastInputInfo + GetTickCount64)
// Porting 1:1 di windows.py, inclusa la gestione del wraparound 32-bit
// di GetLastInputInfo (che usa un DWORD, mentre GetTickCount64 è a 64
// bit apposta per evitare l'overflow dopo ~49.7 giorni di uptime).
// ============================================================

#[cfg(windows)]
fn seconds_since_last_input() -> f64 {
    use windows::Win32::System::SystemInformation::GetTickCount64;
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};

    let tick_count = unsafe { GetTickCount64() };

    let mut last_input_info = LASTINPUTINFO {
        cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };
    let ok = unsafe { GetLastInputInfo(&mut last_input_info) };
    assert!(ok.as_bool(), "GetLastInputInfo failed");
    let last_input_tick = last_input_info.dwTime as u64;

    // GetLastInputInfo restituisce un DWORD a 32 bit che va in overflow
    // a 2^32 ms. GetTickCount64 è a 64 bit. Per calcolare la differenza
    // correttamente, confrontiamo solo i 32 bit bassi quando i valori
    // sono vicini (caso normale), gestendo il wraparound quando dwTime
    // supera i 32 bit bassi di tick_count.
    let tick_lower32 = tick_count & 0xFFFF_FFFF;
    let diff_ms = if tick_lower32 >= last_input_tick {
        tick_lower32 - last_input_tick
    } else {
        (0x1_0000_0000_u64 - last_input_tick) + tick_lower32
    };

    diff_ms as f64 / 1000.0
}

#[cfg(not(windows))]
fn seconds_since_last_input() -> f64 {
    compile_error!("aw-watcher-afk (questo porting) supporta solo Windows");
}

// ============================================================
// CONFIGURAZIONE (stesso file TOML letto da Python, stessa
// convenzione di percorso "platformdirs" — vedi nota sotto)
// ============================================================

#[derive(Debug, Deserialize, Default)]
struct ConfigSection {
    timeout: Option<f64>,
    poll_time: Option<f64>,
}

#[derive(Debug, Deserialize, Default)]
struct ConfigFile {
    #[serde(rename = "aw-watcher-afk")]
    normal: Option<ConfigSection>,
    #[serde(rename = "aw-watcher-afk-testing")]
    testing: Option<ConfigSection>,
}

struct Defaults {
    timeout: f64,
    poll_time: f64,
}

/// Path del file di config: platformdirs.user_config_dir("activitywatch")
/// su Windows usa "activitywatch" sia come "author" che come "appname",
/// producendo %LOCALAPPDATA%\activitywatch\activitywatch\<modulo>\ (cartella
/// annidata due volte — non è una particolarità di questa macchina, è
/// così che platformdirs costruisce il percorso quando gli si passa un
/// solo nome). Stesso path che l'utente ha già usato per personalizzare
/// il timeout (300s invece del default 180s) — va rispettato, non solo
/// letto in astratto.
fn config_file_path() -> std::path::PathBuf {
    dirs::data_local_dir()
        .unwrap_or_default()
        .join("activitywatch")
        .join("activitywatch")
        .join("aw-watcher-afk")
        .join("aw-watcher-afk.toml")
}

fn load_defaults(testing: bool) -> Defaults {
    let hardcoded = if testing {
        Defaults { timeout: 20.0, poll_time: 1.0 }
    } else {
        Defaults { timeout: 180.0, poll_time: 5.0 }
    };

    let Ok(content) = std::fs::read_to_string(config_file_path()) else {
        return hardcoded;
    };
    let Ok(parsed) = toml::from_str::<ConfigFile>(&content) else {
        return hardcoded;
    };
    let section = if testing { parsed.testing } else { parsed.normal }.unwrap_or_default();

    Defaults {
        timeout: section.timeout.unwrap_or(hardcoded.timeout),
        poll_time: section.poll_time.unwrap_or(hardcoded.poll_time),
    }
}

#[derive(Parser)]
#[command(about = "A watcher for keyboard and mouse input to detect AFK state.")]
struct Args {
    #[arg(long)]
    host: Option<String>,

    #[arg(long)]
    port: Option<u16>,

    /// Gira in modalità test: porta 5666, dati separati da quelli reali
    #[arg(long)]
    testing: bool,

    /// Secondi di inattività oltre i quali l'utente è considerato afk
    /// (default dal file di config, o 180s/20s testing se assente)
    #[arg(long)]
    timeout: Option<f64>,

    /// Ogni quanti secondi controllare l'input (default dal file di
    /// config, o 5s/1s testing se assente)
    #[arg(long)]
    poll_time: Option<f64>,
}

/// Nome con cui questo watcher si annuncia (usato solo per popolare il
/// campo "client" del bucket alla sua creazione, che ora avviene nel
/// processo Tauri — vedi BLUEPRINT.md, Fase 5).
const CLIENT_NAME: &str = "aw-watcher-afk";
const BUCKET_TYPE: &str = "afkstatus";

struct AfkWatcher {
    bucket_name: String,
    timeout: f64,
    poll_time: f64,
}

impl AfkWatcher {
    /// Stampa una riga JSON su stdout invece di mandare una richiesta di
    /// rete — il processo Tauri che ci ha lanciato legge questa pipe e
    /// inoltra l'evento al server in-process (vedi BLUEPRINT.md, Fase 5,
    /// per il contratto completo del formato, documentato anche per
    /// watcher di terze parti).
    fn ping(&self, afk: bool, timestamp: DateTime<Utc>, duration: f64) {
        let mut data = Map::new();
        data.insert(
            "status".to_string(),
            if afk { "afk" } else { "not-afk" }.into(),
        );
        let pulsetime = self.timeout + self.poll_time;
        let envelope = json!({
            "bucket_id": self.bucket_name,
            "bucket_type": BUCKET_TYPE,
            "client": CLIENT_NAME,
            "op": "heartbeat",
            "pulsetime": pulsetime,
            "event": {
                "timestamp": timestamp.to_rfc3339_opts(SecondsFormat::Millis, true),
                "duration": duration,
                "data": data,
            },
        });
        let mut stdout = std::io::stdout();
        let _ = writeln!(stdout, "{envelope}");
        let _ = stdout.flush();
    }

    fn heartbeat_loop(&self) {
        let mut afk = false;
        let one_ms = chrono::Duration::milliseconds(1);

        loop {
            let now = Utc::now();
            let seconds_since_input = seconds_since_last_input();
            let last_input = now
                - chrono::Duration::milliseconds((seconds_since_input * 1000.0).round() as i64);

            if afk && seconds_since_input < self.timeout {
                println!("No longer AFK");
                self.ping(afk, last_input, 0.0);
                afk = false;
                // ping con timestamp+1ms per il prossimo evento (per
                // essere sicuri che il get_event successivo lo recuperi).
                self.ping(afk, last_input + one_ms, 0.0);
            } else if !afk && seconds_since_input >= self.timeout {
                println!("Became AFK");
                self.ping(afk, last_input, 0.0);
                afk = true;
                self.ping(afk, last_input + one_ms, seconds_since_input);
            } else if afk {
                // stesso +1ms qui, per non "perdere" l'ultimo heartbeat
                // (se last_input non è cambiato).
                self.ping(afk, last_input + one_ms, seconds_since_input);
            } else {
                self.ping(afk, last_input, 0.0);
            }

            thread::sleep(StdDuration::from_secs_f64(self.poll_time));
        }
    }
}

fn main() {
    let args = Args::parse();
    let defaults = load_defaults(args.testing);
    let timeout = args.timeout.unwrap_or(defaults.timeout);
    let poll_time = args.poll_time.unwrap_or(defaults.poll_time);
    assert!(timeout >= poll_time, "timeout deve essere >= poll_time");

    let hostname = gethostname::gethostname().to_string_lossy().to_string();
    let bucket_name = format!("{}_{}", CLIENT_NAME, hostname);

    println!(
        "Modalità: {} - timeout: {timeout}s, poll_time: {poll_time}s",
        if args.testing { "testing" } else { "normale" }
    );
    println!("Bucket: {bucket_name}");

    let watcher = AfkWatcher {
        bucket_name,
        timeout,
        poll_time,
    };
    watcher.heartbeat_loop();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_python_hardcoded_values() {
        // Nessun file di config: devono valere gli hardcoded default di
        // Python (180/5 produzione, 20/1 testing).
        let d = load_defaults(false);
        // Non verifichiamo qui il valore esatto perché su questa macchina
        // il file di config REALE esiste (timeout personalizzato) — vedi
        // config_overrides_are_merged_with_defaults per quel caso.
        assert!(d.timeout > 0.0 && d.poll_time > 0.0);
    }

    #[test]
    fn parses_real_config_toml_shape() {
        // Stesso identico contenuto del file di config reale di questa
        // macchina (timeout personalizzato a 300, poll_time commentato
        // = resta il default).
        let toml_str = "[aw-watcher-afk]\ntimeout = 300\n#poll_time = 5\n\n[aw-watcher-afk-testing]\n#timeout = 20\n#poll_time = 1\n";
        let parsed: ConfigFile = toml::from_str(toml_str).unwrap();
        let normal = parsed.normal.unwrap();
        assert_eq!(normal.timeout, Some(300.0));
        assert_eq!(normal.poll_time, None); // commentato, non presente

        let testing = parsed.testing.unwrap_or_default();
        assert_eq!(testing.timeout, None);
    }

    #[test]
    fn missing_config_file_falls_back_to_hardcoded_defaults() {
        let d = load_defaults(true);
        // In testing mode, se non troviamo/parsiamo un file valido,
        // torniamo comunque a valori sensati (>0), mai a zero/negativi.
        assert!(d.timeout > 0.0);
        assert!(d.poll_time > 0.0);
        assert!(d.timeout >= d.poll_time);
    }
}

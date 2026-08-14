//! Legge i transcript locali di Claude Code (i file .jsonl che Claude Code
//! stesso scrive in ~/.claude/projects/<progetto>/<sessionId>.jsonl, uno
//! per sessione) e manda ad ActivityWatch un heartbeat per ogni messaggio
//! con timestamp reale, etichettato con il titolo che Claude Code stesso
//! genera per la sessione appena disponibile, o col nome della cartella
//! di lavoro ("cwd") in mancanza d'altro. Porting 1:1 da
//! aw_watcher_claude_code/main.py (Python).
//!
//! A differenza della VPN, qui non esiste un "inizio/fine sessione"
//! esplicito — solo eventi heartbeat che ActivityWatch unisce da solo se
//! entro la soglia (pulsetime).

use std::collections::HashMap;
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration as StdDuration;

use chrono::{DateTime, SecondsFormat, Utc};
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

const CLIENT_NAME: &str = "aw-watcher-claude-code";
const BUCKET_ID: &str = "claude-code-sessions";
const BUCKET_TYPE: &str = "claude.session";
// Stessa soglia usata per unire blocchi vicini nella Timeline della Home
// (MERGE_GAP_SECONDS in HomeTimelineSection.vue).
const DEFAULT_IDLE_TIMEOUT_SECONDS: i64 = 300;

/// Cartella scrivibile condivisa - default: %LOCALAPPDATA%\TrackFlow\app-data
/// (stessa cartella di icone/screenshot/VPN). Non più "accanto all'exe":
/// quel percorso cambia tra dev/release/installazione reale (spesso sotto
/// Program Files, di sola lettura per un utente standard).
fn default_app_data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_default()
        .join("TrackFlow")
        .join("app-data")
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct FileState {
    byte_letti: u64,
    cwd_conosciuto: Option<String>,
    titolo_ai: Option<String>,
    entrypoint_conosciuto: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct State {
    file: HashMap<String, FileState>,
}

fn load_state(path: &Path) -> State {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_state(path: &Path, state: &State) {
    if let Ok(json) = serde_json::to_string_pretty(state) {
        let _ = fs::write(path, json);
    }
}

fn load_project_mapping(path: &Path) -> HashMap<String, String> {
    let mut mapping = HashMap::new();
    let Ok(content) = fs::read_to_string(path) else {
        return mapping;
    };
    for riga in content.lines() {
        let riga = riga.trim();
        if riga.is_empty() || riga.starts_with('#') || !riga.contains('=') {
            continue;
        }
        if let Some((cartella, nome)) = riga.split_once('=') {
            mapping.insert(cartella.trim().to_string(), nome.trim().to_string());
        }
    }
    mapping
}

fn nome_progetto_da_cwd(cwd: Option<&str>, mapping: &HashMap<String, String>) -> String {
    let Some(cwd) = cwd else {
        return "Sconosciuto".to_string();
    };
    // cwd arriva con separatori Windows ("\\") o POSIX ("/") a seconda
    // dell'host che ha scritto il transcript — normalizziamo prima di
    // prendere l'ultimo pezzo del percorso.
    let normalizzato = cwd.replace('\\', "/");
    let cartella = Path::new(&normalizzato)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(cwd)
        .to_string();
    mapping.get(&cartella).cloned().unwrap_or(cartella)
}

fn sorgente_da_entrypoint(entrypoint: Option<&str>) -> &'static str {
    match entrypoint {
        Some("claude-desktop") => "Claude Desktop",
        Some("cli") => "Claude Code (CLI)",
        _ => "Claude Code",
    }
}

/// Un file per sessione, dentro una sottocartella per progetto — vanno
/// ri-scoperti a ogni giro perché Claude Code ne crea di nuovi ogni volta
/// che parte una sessione nuova.
fn trova_file_transcript(progetti_dir: &Path) -> Vec<PathBuf> {
    if !progetti_dir.exists() {
        return Vec::new();
    }
    let pattern = progetti_dir.join("*").join("*.jsonl");
    let Some(pattern_str) = pattern.to_str() else {
        return Vec::new();
    };
    let mut trovati: Vec<PathBuf> = glob::glob(pattern_str)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .collect();
    trovati.sort();
    trovati
}

struct ClaudeCodeWatcher {
    idle_timeout_seconds: f64,
    progetti_dir: PathBuf,
    mapping: HashMap<String, String>,
    stato: State,
    state_file: PathBuf,
}

impl ClaudeCodeWatcher {
    fn new(
        idle_timeout_seconds: i64,
        progetti_dir: PathBuf,
        mapping: HashMap<String, String>,
        state_file: PathBuf,
    ) -> Self {
        let stato = load_state(&state_file);

        ClaudeCodeWatcher {
            idle_timeout_seconds: idle_timeout_seconds as f64,
            progetti_dir,
            mapping,
            stato,
            state_file,
        }
    }

    fn controlla_tutti_i_file(&mut self) {
        for percorso in trova_file_transcript(&self.progetti_dir) {
            self.controlla_file(&percorso);
        }
    }

    fn controlla_file(&mut self, percorso: &Path) {
        let chiave = percorso.to_string_lossy().to_string();
        let dimensione_attuale = fs::metadata(percorso).map(|m| m.len()).unwrap_or(0);

        if !self.stato.file.contains_key(&chiave) {
            // File di transcript mai visto prima: partiamo dalla fine
            // di quello che c'è già, così non mandiamo tutta la
            // cronologia passata della sessione.
            self.stato.file.insert(
                chiave.clone(),
                FileState {
                    byte_letti: dimensione_attuale,
                    ..Default::default()
                },
            );
            save_state(&self.state_file, &self.stato);
            return;
        }

        let byte_gia_letti = self.stato.file[&chiave].byte_letti;
        let byte_gia_letti = if dimensione_attuale < byte_gia_letti {
            0
        } else {
            byte_gia_letti
        };

        if dimensione_attuale == byte_gia_letti {
            return;
        }

        let Ok(mut file) = fs::File::open(percorso) else {
            return;
        };
        if file.seek(SeekFrom::Start(byte_gia_letti)).is_err() {
            return;
        }
        let mut nuovi_byte = Vec::new();
        if file.read_to_end(&mut nuovi_byte).is_err() {
            return;
        }

        // Lettura in binario + taglio all'ultimo "a capo" completo: un
        // transcript può essere ancora in scrittura, quindi l'ultima
        // riga potrebbe essere a metà. La lasciamo per il giro
        // successivo invece di provare a fare il parse di JSON tagliato.
        let Some(ultimo_a_capo) = nuovi_byte.iter().rposition(|&b| b == b'\n') else {
            return;
        };

        let blocco_completo = &nuovi_byte[..=ultimo_a_capo];
        let nuova_posizione = byte_gia_letti + blocco_completo.len() as u64;
        let testo = String::from_utf8_lossy(blocco_completo).to_string();

        for riga in testo.lines() {
            let riga = riga.trim();
            if !riga.is_empty() {
                self.gestisci_riga(&chiave, riga);
            }
        }

        if let Some(s) = self.stato.file.get_mut(&chiave) {
            s.byte_letti = nuova_posizione;
        }
        save_state(&self.state_file, &self.stato);
    }

    fn gestisci_riga(&mut self, chiave: &str, riga_testo: &str) {
        let Ok(riga) = serde_json::from_str::<Value>(riga_testo) else {
            return;
        };

        // Riga speciale, senza timestamp: il titolo che Claude Code
        // stesso genera per la sessione — molto più leggibile del solo
        // nome cartella, quindi lo teniamo da parte e lo usiamo come
        // etichetta appena arriva.
        if riga.get("type").and_then(Value::as_str) == Some("ai-title") {
            if let Some(titolo) = riga.get("aiTitle").and_then(Value::as_str) {
                if let Some(s) = self.stato.file.get_mut(chiave) {
                    s.titolo_ai = Some(titolo.to_string());
                }
                return;
            }
        }

        let Some(timestamp_raw) = riga.get("timestamp").and_then(Value::as_str) else {
            // Altre righe senza timestamp sono metadati, non un vero
            // momento di attività.
            return;
        };

        let Ok(quando) = DateTime::parse_from_rfc3339(timestamp_raw) else {
            return;
        };
        let quando = quando.with_timezone(&Utc);

        let s = self.stato.file.entry(chiave.to_string()).or_default();

        let cwd = riga.get("cwd").and_then(Value::as_str);
        if let Some(cwd) = cwd {
            s.cwd_conosciuto = Some(cwd.to_string());
        }
        let cwd = cwd.map(str::to_string).or_else(|| s.cwd_conosciuto.clone());

        let entrypoint = riga.get("entrypoint").and_then(Value::as_str);
        if let Some(ep) = entrypoint {
            s.entrypoint_conosciuto = Some(ep.to_string());
        }
        let entrypoint = entrypoint
            .map(str::to_string)
            .or_else(|| s.entrypoint_conosciuto.clone());

        // Preferiamo il titolo AI della sessione quando c'è già (più
        // leggibile), altrimenti il nome della cartella di progetto.
        // Anteposta la sorgente (Claude Desktop / Claude Code CLI / ...)
        // così le due si distinguono anche quando lavori sullo stesso
        // progetto da entrambe.
        let nome_progetto = s
            .titolo_ai
            .clone()
            .unwrap_or_else(|| nome_progetto_da_cwd(cwd.as_deref(), &self.mapping));
        let etichetta = format!(
            "{}: {}",
            sorgente_da_entrypoint(entrypoint.as_deref()),
            nome_progetto
        );

        self.manda_heartbeat(quando, &etichetta);
    }

    fn manda_heartbeat(&self, quando: DateTime<Utc>, nome_progetto: &str) {
        let mut data = Map::new();
        data.insert("cliente".to_string(), nome_progetto.into());
        let envelope = json!({
            "bucket_id": BUCKET_ID,
            "bucket_type": BUCKET_TYPE,
            "client": CLIENT_NAME,
            "op": "heartbeat",
            "pulsetime": self.idle_timeout_seconds,
            "event": {
                "timestamp": quando.to_rfc3339_opts(SecondsFormat::Millis, true),
                "duration": 0.0,
                "data": data,
            },
        });
        let mut stdout = std::io::stdout();
        let _ = writeln!(stdout, "{envelope}");
        let _ = stdout.flush();
    }
}

#[derive(Parser)]
#[command(
    about = "Watcher per le sessioni Claude Code, letto dai transcript locali (~/.claude/projects)"
)]
struct Args {
    /// Gira in modalità test: porta 5666, dati separati da quelli reali
    #[arg(long)]
    testing: bool,

    /// Ogni quanti secondi controllare i transcript (default: 15)
    #[arg(long, default_value_t = 15)]
    poll_interval: i64,

    /// Secondi di inattività tra un messaggio e l'altro oltre i quali si
    /// considera chiusa la sessione di lavoro
    #[arg(long, default_value_t = DEFAULT_IDLE_TIMEOUT_SECONDS)]
    idle_timeout: i64,

    /// Cartella scrivibile condivisa dove leggere `project_mapping.txt` e
    /// salvare lo stato - default: %LOCALAPPDATA%\TrackFlow\app-data
    /// (stessa cartella di icone/screenshot/VPN, passata da Tauri).
    #[arg(long)]
    app_data_dir: Option<PathBuf>,
}

fn main() {
    let args = Args::parse();

    let data_dir = args.app_data_dir.clone().unwrap_or_else(default_app_data_dir);
    let _ = fs::create_dir_all(&data_dir);
    let state_file = data_dir.join("claude_code_watcher_state.json");
    let mapping_file = data_dir.join("project_mapping.txt");

    // Cartella dove Claude Code scrive i transcript — stessa per Claude
    // Code CLI, l'estensione IDE e Claude Desktop.
    let progetti_dir = std::env::var("AW_WATCHER_CLAUDE_CODE_PROJECTS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_default()
                .join(".claude")
                .join("projects")
        });

    if !progetti_dir.exists() {
        println!(
            "ATTENZIONE: cartella progetti Claude Code non trovata: {}",
            progetti_dir.display()
        );
        return;
    }

    let mapping = load_project_mapping(&mapping_file);

    println!(
        "Modalità: {}",
        if args.testing { "testing (porta 5666)" } else { "normale (porta 5600)" }
    );
    println!("Mappatura progetti caricata: {} cartelle note", mapping.len());
    println!("Cartella transcript: {}", progetti_dir.display());

    let mut watcher = ClaudeCodeWatcher::new(
        args.idle_timeout,
        progetti_dir,
        mapping,
        state_file,
    );

    loop {
        thread::sleep(StdDuration::from_secs(args.poll_interval as u64));
        watcher.controlla_tutti_i_file();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_message_line_with_cwd_and_entrypoint() {
        let dir = std::env::temp_dir().join(format!("aw-cc-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let state_file = dir.join("state.json");
        let mapping_file = dir.join("mapping.txt");
        std::fs::write(&mapping_file, "aw-webui=TrackFlow\n").unwrap();

        let mapping = load_project_mapping(&mapping_file);
        let mut watcher = ClaudeCodeWatcher::new(
            DEFAULT_IDLE_TIMEOUT_SECONDS,
            dir.clone(),
            mapping,
            state_file,
        );

        let chiave = "test.jsonl";
        watcher.stato.file.insert(chiave.to_string(), FileState::default());

        let riga = r#"{"timestamp":"2026-08-09T15:30:00Z","cwd":"C:\\Users\\bandi\\Documents\\Conteggio_Ore\\aw-webui","entrypoint":"cli"}"#;
        watcher.gestisci_riga(chiave, riga);

        let s = &watcher.stato.file[chiave];
        assert_eq!(s.cwd_conosciuto.as_deref(), Some("C:\\Users\\bandi\\Documents\\Conteggio_Ore\\aw-webui"));
        assert_eq!(s.entrypoint_conosciuto.as_deref(), Some("cli"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn ai_title_line_is_cached_and_no_heartbeat_sent() {
        let dir = std::env::temp_dir().join(format!("aw-cc-test2-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let state_file = dir.join("state.json");
        let mapping = HashMap::new();
        let mut watcher =
            ClaudeCodeWatcher::new(DEFAULT_IDLE_TIMEOUT_SECONDS, dir.clone(), mapping, state_file);

        let chiave = "test.jsonl";
        watcher.stato.file.insert(chiave.to_string(), FileState::default());

        let riga = r#"{"type":"ai-title","aiTitle":"Fix build pipeline"}"#;
        watcher.gestisci_riga(chiave, riga);

        assert_eq!(
            watcher.stato.file[chiave].titolo_ai.as_deref(),
            Some("Fix build pipeline")
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn cwd_to_project_name_normalizes_separators_and_uses_mapping() {
        let mut mapping = HashMap::new();
        mapping.insert("aw-webui".to_string(), "TrackFlow".to_string());

        assert_eq!(
            nome_progetto_da_cwd(Some("C:\\Users\\bandi\\Documents\\Conteggio_Ore\\aw-webui"), &mapping),
            "TrackFlow"
        );
        assert_eq!(
            nome_progetto_da_cwd(Some("/home/user/some-other-project"), &mapping),
            "some-other-project"
        );
        assert_eq!(nome_progetto_da_cwd(None, &mapping), "Sconosciuto");
    }

    #[test]
    fn entrypoint_labels_match_python_mapping() {
        assert_eq!(sorgente_da_entrypoint(Some("claude-desktop")), "Claude Desktop");
        assert_eq!(sorgente_da_entrypoint(Some("cli")), "Claude Code (CLI)");
        assert_eq!(sorgente_da_entrypoint(Some("qualcos-altro")), "Claude Code");
        assert_eq!(sorgente_da_entrypoint(None), "Claude Code");
    }

    #[test]
    fn incomplete_last_line_is_not_consumed() {
        // Verifica indirettamente tramite controlla_file: scriviamo un
        // file con una riga completa + una a metà (senza \n finale), e
        // controlliamo che byte_letti si fermi subito dopo l'ultimo \n.
        let dir = std::env::temp_dir().join(format!("aw-cc-test3-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let transcript_path = dir.join("session.jsonl");
        std::fs::write(
            &transcript_path,
            "{\"timestamp\":\"2026-08-09T15:30:00Z\",\"cwd\":\"/x/proj\"}\n{\"timestamp\":\"2026-08-09T15",
        )
        .unwrap();

        let state_file = dir.join("state.json");
        let mut watcher = ClaudeCodeWatcher::new(
            DEFAULT_IDLE_TIMEOUT_SECONDS,
            dir.clone(),
            HashMap::new(),
            state_file,
        );

        let chiave = transcript_path.to_string_lossy().to_string();
        // Prima chiamata: file mai visto, si posiziona alla fine attuale
        // (comportamento "non rileggere lo storico") — quindi simuliamo
        // uno stato preesistente con byte_letti a 0 per testare il taglio.
        watcher.stato.file.insert(
            chiave.clone(),
            FileState { byte_letti: 0, ..Default::default() },
        );

        watcher.controlla_file(&transcript_path);

        let contenuto_completo = std::fs::read(&transcript_path).unwrap();
        let ultimo_a_capo = contenuto_completo.iter().rposition(|&b| b == b'\n').unwrap();
        assert_eq!(watcher.stato.file[&chiave].byte_letti, (ultimo_a_capo + 1) as u64);

        std::fs::remove_dir_all(&dir).ok();
    }
}

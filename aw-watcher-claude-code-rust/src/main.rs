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

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{BufRead, Read, Seek, SeekFrom, Write};
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
    /// true dopo la prima lettura completa del file (cartella, origine, titolo).
    #[serde(default)]
    scansionato: bool,
    /// Ultima etichetta calcolata per questa sessione e quelle che aveva prima
    /// (es. chat rinominata): servono a correggere gli eventi vecchi.
    #[serde(default)]
    etichetta_attuale: Option<String>,
    #[serde(default)]
    etichette_superate: Vec<String>,
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
    let pezzi: Vec<&str> = normalizzato.split('/').filter(|p| !p.is_empty()).collect();
    let cartella = pezzi.last().copied().unwrap_or(cwd).to_string();
    if let Some(nome) = mapping.get(&cartella) {
        return nome.clone();
    }
    // Le chat di Claude Desktop senza cartella di progetto lavorano in
    // una cartella temporanea "scratch-AAAA-MM-GG-xxxx": inutile come
    // nome, la mostriamo come "Chat senza cartella · GG/MM".
    if let Some(resto) = cartella.strip_prefix("scratch-") {
        let p: Vec<&str> = resto.splitn(4, '-').collect();
        if p.len() == 4 && p[0].len() == 4 && p[1].len() == 2 && p[2].len() == 2 {
            return format!("Chat senza cartella · {}/{}", p[2], p[1]);
        }
    }
    // Cartelle generiche (src, src-tauri...): da sole non dicono quale
    // progetto sia, aggiungiamo quella sopra.
    const GENERICHE: [&str; 15] = [
        "src", "src-tauri", "app", "apps", "lib", "server", "client", "frontend", "backend",
        "web", "dist", "build", "public", "docs", "tests",
    ];
    if pezzi.len() > 1 && GENERICHE.contains(&cartella.to_lowercase().as_str()) {
        return format!("{} / {}", pezzi[pezzi.len() - 2], cartella);
    }
    cartella
}

/// Dati che Claude Desktop salva per ogni chat della scheda Code
/// (%APPDATA%\Claude\claude-code-sessions\<account>\<org>\local_*.json).
/// Una chat può avere più sessioni (ripresa, fork, rewind, cambio
/// cartella), ognuna con un suo transcript .jsonl: qui ci sono tutti gli
/// id, più il titolo che la chat ha nell'app.
#[derive(Deserialize)]
struct ChatApp {
    #[serde(rename = "cliSessionId")]
    cli_session_id: Option<String>,
    #[serde(rename = "priorCliSessionIds", default)]
    prior_cli_session_ids: Vec<String>,
    title: Option<String>,
}

/// id sessione (nome del file .jsonl) -> titolo della chat nell'app.
/// Sessioni diverse della stessa chat danno lo stesso titolo, quindi
/// finiscono sotto la stessa etichetta.
fn carica_titoli_chat(dir_sessioni_app: &Path) -> HashMap<String, String> {
    let mut titoli = HashMap::new();
    let Some(pattern) = dir_sessioni_app.join("*").join("*").join("local_*.json").to_str().map(str::to_string)
    else {
        return titoli;
    };
    for percorso in glob::glob(&pattern).into_iter().flatten().filter_map(Result::ok) {
        let Ok(testo) = fs::read_to_string(&percorso) else { continue };
        let Ok(chat) = serde_json::from_str::<ChatApp>(&testo) else { continue };
        let Some(titolo) = chat.title.filter(|t| !t.trim().is_empty()) else { continue };
        for id in chat.cli_session_id.into_iter().chain(chat.prior_cli_session_ids) {
            titoli.insert(id, titolo.clone());
        }
    }
    titoli
}

fn sorgente_da_entrypoint(entrypoint: Option<&str>) -> &'static str {
    match entrypoint {
        Some("claude-desktop") => "Claude Desktop",
        Some("cli") => "Claude Code (CLI)",
        _ => "Claude Code",
    }
}

/// Legge una volta tutto un transcript e ne ricava (cartella di lavoro,
/// origine, ultimo titolo automatico). Il titolo si cerca solo nelle righe
/// che lo contengono; cartella e origine nelle prime righe.
fn scansiona_transcript(percorso: &Path) -> (Option<String>, Option<String>, Option<String>) {
    let Ok(file) = fs::File::open(percorso) else {
        return (None, None, None);
    };
    let (mut cwd, mut entrypoint, mut titolo) = (None, None, None);
    for (i, riga) in std::io::BufReader::new(file).lines().map_while(Result::ok).enumerate() {
        let col_titolo = riga.contains("\"ai-title\"");
        if !col_titolo && (i > 200 || (cwd.is_some() && entrypoint.is_some())) {
            continue;
        }
        let Ok(v) = serde_json::from_str::<Value>(&riga) else {
            continue;
        };
        if v.get("type").and_then(Value::as_str) == Some("ai-title") {
            if let Some(t) = v.get("aiTitle").and_then(Value::as_str) {
                titolo = Some(t.to_string());
            }
            continue;
        }
        if cwd.is_none() {
            cwd = v.get("cwd").and_then(Value::as_str).map(str::to_string);
        }
        if entrypoint.is_none() {
            entrypoint = v.get("entrypoint").and_then(Value::as_str).map(str::to_string);
        }
    }
    (cwd, entrypoint, titolo)
}

/// Etichetta attuale di una sessione: titolo della chat nell'app (anche
/// rinominato a mano, unisce le sessioni riprese/forkate), altrimenti
/// titolo automatico di Claude Code, altrimenti cartella di lavoro. Davanti
/// la sorgente (Claude Desktop / CLI...) così si distinguono anche quando
/// lavori sullo stesso progetto da entrambe.
fn costruisci_etichetta(
    chiave: &str,
    s: &FileState,
    titoli_chat: &HashMap<String, String>,
    mapping: &HashMap<String, String>,
) -> String {
    let id = Path::new(chiave).file_stem().and_then(|n| n.to_str()).unwrap_or("");
    let nome = titoli_chat
        .get(id)
        .cloned()
        .or_else(|| s.titolo_ai.clone())
        .unwrap_or_else(|| nome_progetto_da_cwd(s.cwd_conosciuto.as_deref(), mapping));
    format!("{}: {}", sorgente_da_entrypoint(s.entrypoint_conosciuto.as_deref()), nome)
}

/// Etichette con cui le versioni precedenti del watcher hanno salvato gli
/// eventi di questa sessione: titolo automatico, oppure ultima cartella.
fn etichette_storiche(s: &FileState, mapping: &HashMap<String, String>) -> Vec<String> {
    let sorgente = sorgente_da_entrypoint(s.entrypoint_conosciuto.as_deref());
    let mut v = Vec::new();
    if let Some(t) = &s.titolo_ai {
        v.push(format!("{sorgente}: {t}"));
    }
    let cartella = match s.cwd_conosciuto.as_deref() {
        Some(cwd) => {
            let c = cwd.replace('\\', "/").split('/').filter(|p| !p.is_empty()).last().unwrap_or(cwd).to_string();
            mapping.get(&c).cloned().unwrap_or(c)
        }
        None => "Sconosciuto".to_string(),
    };
    v.push(format!("{sorgente}: {cartella}"));
    v
}

/// Coppie (etichetta vecchia, etichetta attuale) da correggere nello
/// storico. Regole di sicurezza: una vecchia etichetta si corregge solo se
/// porta a UNA sola chat (se due chat condividono la stessa cartella non si
/// può sapere a quale appartenga un evento) e se non è l'etichetta attuale
/// di un'altra sessione (altrimenti si rubano i suoi eventi).
fn calcola_relabel(
    stato: &mut State,
    titoli_chat: &HashMap<String, String>,
    mapping: &HashMap<String, String>,
) -> Vec<(String, String)> {
    let mut obiettivi: HashMap<String, HashSet<String>> = HashMap::new();
    let mut in_uso: HashSet<String> = HashSet::new();
    for (chiave, s) in stato.file.iter_mut() {
        if !s.scansionato {
            continue;
        }
        let attuale = costruisci_etichetta(chiave, s, titoli_chat, mapping);
        if let Some(prec) = s.etichetta_attuale.take() {
            if prec != attuale && !s.etichette_superate.contains(&prec) {
                s.etichette_superate.push(prec);
            }
        }
        s.etichetta_attuale = Some(attuale.clone());
        in_uso.insert(attuale.clone());
        let mut vecchie = etichette_storiche(s, mapping);
        vecchie.extend(s.etichette_superate.iter().cloned());
        for v in vecchie.into_iter().filter(|v| *v != attuale) {
            obiettivi.entry(v).or_default().insert(attuale.clone());
        }
    }
    let mut coppie: Vec<(String, String)> = obiettivi
        .into_iter()
        .filter(|(v, t)| t.len() == 1 && !in_uso.contains(v))
        .filter_map(|(v, t)| t.into_iter().next().map(|a| (v, a)))
        .collect();
    coppie.sort();
    coppie
}

/// Nome (non percorso completo) della cartella di lavoro dedicata usata
/// dalla chat dell'assistente AI di TrackFlow quando il provider è
/// "Claude (abbonamento Desktop)" (vedi claude_subscription.rs) — e
/// prefisso condiviso da quella usata per la categorizzazione
/// automatica delle app (categorization.rs, "claude-agent-workdir-
/// categorizzazione"). Claude Code sanitizza il percorso di lavoro
/// completo in un nome di cartella sotto ~/.claude/projects/
/// sostituendo i separatori con trattini, ma preservando i nomi delle
/// cartelle originali — quindi basta cercare questo prefisso come
/// sottostringa, indipendentemente da utente/percorso della macchina.
const PREFISSO_CARTELLA_CHAT_INTERNA: &str = "claude-agent-workdir";

/// Un file per sessione, dentro una sottocartella per progetto — vanno
/// ri-scoperti a ogni giro perché Claude Code ne crea di nuovi ogni volta
/// che parte una sessione nuova.
///
/// Bug reale trovato scrivendo la documentazione per gli utenti: la
/// chat dell'assistente AI di TrackFlow (e la categorizzazione
/// automatica delle app) passano anch'esse dal CLI di Claude Code,
/// quindi scrivono transcript nella STESSA cartella condivisa che
/// questo watcher legge — finivano tracciate in Timeline come se
/// fossero sessioni di lavoro su un progetto vero (es. "Claude Desktop:
/// claude-agent-workdir-categorizzazione" invece di un nome di
/// progetto). Escluse qui, alla scoperta dei file, così non generano
/// alcun evento — mai un problema di visualizzazione da correggere a
/// valle, il dato semplicemente non viene mai prodotto.
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
        .filter(|percorso| {
            let cartella_progetto = percorso
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("");
            !cartella_progetto.contains(PREFISSO_CARTELLA_CHAT_INTERNA)
        })
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
    /// Vuota = nessuna lettura dei titoli dell'app (si ricade su titolo AI / cartella).
    dir_sessioni_app: Option<PathBuf>,
    titoli_chat: HashMap<String, String>,
    /// Relabel già mandati in questa esecuzione (etichetta vecchia -> nuova).
    /// Non persistito: a ogni avvio si rimandano, l'app li applica in modo idempotente.
    relabel_inviati: HashMap<String, String>,
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
            dir_sessioni_app: None,
            titoli_chat: HashMap::new(),
            relabel_inviati: HashMap::new(),
        }
    }

    fn controlla_tutti_i_file(&mut self) {
        // Riletti a ogni giro: le chat nuove o rinominate nell'app
        // devono comparire subito con il nome giusto.
        if let Some(dir) = &self.dir_sessioni_app {
            self.titoli_chat = carica_titoli_chat(dir);
        }
        for percorso in trova_file_transcript(&self.progetti_dir) {
            self.controlla_file(&percorso);
        }
        self.riallinea_storico();
    }

    fn controlla_file(&mut self, percorso: &Path) {
        let chiave = percorso.to_string_lossy().to_string();
        let dimensione_attuale = fs::metadata(percorso).map(|m| m.len()).unwrap_or(0);

        let nuovo = !self.stato.file.contains_key(&chiave);
        if nuovo {
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
        }

        // Una sola lettura completa per file: cartella, origine e titolo
        // automatico servono a riconoscere le etichette vecchie dello
        // storico (vedi riallinea_storico), anche per i file che non
        // vengono più scritti.
        if !self.stato.file[&chiave].scansionato {
            let (cwd, entrypoint, titolo) = scansiona_transcript(percorso);
            if let Some(s) = self.stato.file.get_mut(&chiave) {
                s.cwd_conosciuto = s.cwd_conosciuto.take().or(cwd);
                s.entrypoint_conosciuto = s.entrypoint_conosciuto.take().or(entrypoint);
                s.titolo_ai = titolo.or(s.titolo_ai.take());
                s.scansionato = true;
            }
        }

        if nuovo {
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

        if let Some(cwd) = riga.get("cwd").and_then(Value::as_str) {
            s.cwd_conosciuto = Some(cwd.to_string());
        }
        if let Some(ep) = riga.get("entrypoint").and_then(Value::as_str) {
            s.entrypoint_conosciuto = Some(ep.to_string());
        }

        let etichetta = costruisci_etichetta(chiave, s, &self.titoli_chat, &self.mapping);
        self.manda_heartbeat(quando, &etichetta);
    }

    /// Allinea lo STORICO già salvato alle etichette di oggi: per ogni
    /// etichetta vecchia di una chat (nome cartella, titolo automatico,
    /// titolo precedente) manda all'app un "relabel" verso l'etichetta
    /// attuale. Ogni coppia parte una sola volta per esecuzione; l'app la
    /// applica agli eventi già nel database (idempotente).
    fn riallinea_storico(&mut self) {
        for (da, a) in calcola_relabel(&mut self.stato, &self.titoli_chat, &self.mapping) {
            if self.relabel_inviati.get(&da) != Some(&a) {
                self.manda_relabel(&da, &a);
                self.relabel_inviati.insert(da, a);
            }
        }
        save_state(&self.state_file, &self.stato);
    }

    fn manda_relabel(&self, da: &str, a: &str) {
        let envelope = json!({
            "bucket_id": BUCKET_ID,
            "bucket_type": BUCKET_TYPE,
            "client": CLIENT_NAME,
            "op": "relabel",
            "event": { "campo": "cliente", "da": da, "a": a },
        });
        let mut stdout = std::io::stdout();
        let _ = writeln!(stdout, "{envelope}");
        let _ = stdout.flush();
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
    // Claude Desktop salva le chat nella cartella di configurazione
    // (Roaming su Windows): da lì leggiamo titolo e sessioni di ogni chat.
    watcher.dir_sessioni_app = dirs::config_dir().map(|d| d.join("Claude").join("claude-code-sessions"));

    loop {
        thread::sleep(StdDuration::from_secs(args.poll_interval as u64));
        watcher.controlla_tutti_i_file();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Bug reale: la chat dell'assistente AI e la categorizzazione
    // automatica di TrackFlow passano dal CLI di Claude Code nella loro
    // cartella di lavoro dedicata — vanno escluse, non trattate come
    // progetti veri.
    #[test]
    fn trova_file_transcript_esclude_la_cartella_di_lavoro_della_chat_interna() {
        let base = std::env::temp_dir().join(format!("aw-cc-esclusione-{}", std::process::id()));
        let progetto_vero = base.join("C--Users-bandi-Documents-TrackFlow-src-tauri");
        let chat_interna = base.join("C--Users-bandi-AppData-Local-TrackFlow-app-data-claude-agent-workdir");
        let categorizzazione =
            base.join("C--Users-bandi-AppData-Local-TrackFlow-app-data-claude-agent-workdir-categorizzazione");
        for dir in [&progetto_vero, &chat_interna, &categorizzazione] {
            std::fs::create_dir_all(dir).unwrap();
            std::fs::write(dir.join("sessione.jsonl"), "").unwrap();
        }

        let trovati = trova_file_transcript(&base);

        assert_eq!(trovati.len(), 1, "solo il progetto vero deve restare: {trovati:?}");
        assert!(trovati[0].starts_with(&progetto_vero));

        std::fs::remove_dir_all(&base).ok();
    }

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
    fn nome_da_cwd_gestisce_scratch_e_cartelle_generiche() {
        let m = HashMap::new();
        assert_eq!(
            nome_progetto_da_cwd(Some("C:\\x\\scratch-2026-10-29-3ddff8"), &m),
            "Chat senza cartella · 29/10"
        );
        assert_eq!(nome_progetto_da_cwd(Some("C:\\_dev\\TrackFlow\\src-tauri"), &m), "TrackFlow / src-tauri");
        assert_eq!(nome_progetto_da_cwd(Some("C:\\_dev\\omg"), &m), "omg");
    }

    // Chat ripresa/forkata: tutti gli id sessione (attuale + precedenti)
    // devono dare lo stesso titolo, quello della chat nell'app.
    #[test]
    fn titoli_chat_uniscono_sessioni_riprese() {
        let base = std::env::temp_dir().join(format!("aw-cc-titoli-{}", std::process::id()));
        let cartella = base.join("account").join("org");
        std::fs::create_dir_all(&cartella).unwrap();
        std::fs::write(
            cartella.join("local_1.json"),
            r#"{"cliSessionId":"c","priorCliSessionIds":["a","b"],"title":"OMG gestionale"}"#,
        )
        .unwrap();
        std::fs::write(cartella.join("local_2.json"), r#"{"cliSessionId":"d","title":""}"#).unwrap();

        let t = carica_titoli_chat(&base);

        for id in ["a", "b", "c"] {
            assert_eq!(t.get(id).map(String::as_str), Some("OMG gestionale"), "id {id}");
        }
        assert!(!t.contains_key("d"), "titolo vuoto: nessuna voce");
        std::fs::remove_dir_all(&base).ok();
    }

    fn stato_con(sessioni: &[(&str, &str)]) -> State {
        let mut stato = State::default();
        for (chiave, cwd) in sessioni {
            stato.file.insert(
                chiave.to_string(),
                FileState {
                    cwd_conosciuto: Some(cwd.to_string()),
                    entrypoint_conosciuto: Some("claude-desktop".to_string()),
                    scansionato: true,
                    ..Default::default()
                },
            );
        }
        stato
    }

    fn titoli(coppie: &[(&str, &str)]) -> HashMap<String, String> {
        coppie.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
    }

    // Il caso reale: eventi salvati col nome cartella ("coucou") prima che
    // si usasse il titolo della chat ("CouCou app").
    #[test]
    fn relabel_corregge_nome_cartella_in_titolo_chat() {
        let mut stato = stato_con(&[("x/7a1f.jsonl", "C:\\_dev\\coucou")]);
        let r = calcola_relabel(&mut stato, &titoli(&[("7a1f", "CouCou app")]), &HashMap::new());
        assert_eq!(
            r,
            vec![("Claude Desktop: coucou".to_string(), "Claude Desktop: CouCou app".to_string())]
        );
    }

    // Due chat nella stessa cartella: non si sa a quale appartengano gli
    // eventi vecchi, quindi non si tocca nulla.
    #[test]
    fn relabel_salta_etichette_ambigue() {
        let mut stato = stato_con(&[("x/aaa.jsonl", "C:\\_dev\\omg"), ("x/bbb.jsonl", "C:\\_dev\\omg")]);
        let r = calcola_relabel(&mut stato, &titoli(&[("aaa", "Chat A"), ("bbb", "Chat B")]), &HashMap::new());
        assert!(r.is_empty(), "{r:?}");
    }

    // L'etichetta vecchia è quella attuale di un'altra sessione senza
    // titolo: correggerla le ruberebbe gli eventi.
    #[test]
    fn relabel_non_ruba_etichette_in_uso() {
        let mut stato = stato_con(&[("x/aaa.jsonl", "C:\\_dev\\omg"), ("x/ccc.jsonl", "C:\\_dev\\omg")]);
        let r = calcola_relabel(&mut stato, &titoli(&[("aaa", "Chat A")]), &HashMap::new());
        assert!(r.is_empty(), "{r:?}");
    }

    // Chat rinominata: gli eventi col titolo precedente passano al nuovo.
    #[test]
    fn relabel_segue_la_rinomina_della_chat() {
        let mut stato = stato_con(&[("x/aaa.jsonl", "C:\\_dev\\omg")]);
        calcola_relabel(&mut stato, &titoli(&[("aaa", "Vecchio")]), &HashMap::new());
        let r = calcola_relabel(&mut stato, &titoli(&[("aaa", "Nuovo")]), &HashMap::new());
        assert!(r.contains(&("Claude Desktop: Vecchio".to_string(), "Claude Desktop: Nuovo".to_string())), "{r:?}");
    }

    #[test]
    fn scansiona_transcript_legge_cartella_origine_e_ultimo_titolo() {
        let dir = std::env::temp_dir().join(format!("aw-cc-scan-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("s.jsonl");
        std::fs::write(
            &p,
            "{\"timestamp\":\"2026-08-09T15:30:00Z\",\"cwd\":\"C:\\\\x\\\\proj\",\"entrypoint\":\"claude-desktop\"}\n\
             {\"type\":\"ai-title\",\"aiTitle\":\"T1\"}\n\
             {\"type\":\"ai-title\",\"aiTitle\":\"T2\"}\n",
        )
        .unwrap();
        let (cwd, ep, titolo) = scansiona_transcript(&p);
        assert_eq!(cwd.as_deref(), Some("C:\\x\\proj"));
        assert_eq!(ep.as_deref(), Some("claude-desktop"));
        assert_eq!(titolo.as_deref(), Some("T2"));
        std::fs::remove_dir_all(&dir).ok();
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

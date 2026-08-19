//! Watcher personalizzati — sorgenti dati scritte dall'utente, senza
//! bisogno di compilarle come sidecar Tauri (vedi BLUEPRINT.md, sezione
//! sul nuovo sistema di moduli/watcher personalizzati).
//!
//! Ogni watcher vive in una propria cartella sotto
//! `app_data_dir/custom/watchers/<id>/`, con un manifest `watcher.json`.
//! Due modalità:
//! - "interval": l'app rilancia il programma dell'utente ogni
//!   `interval_seconds`, si aspetta UNA riga JSON con i soli dati
//!   sull'ultima riga di stdout, e la incapsula da sola in un
//!   `WatcherEnvelope` sintetico — pensata per chi non conosce il
//!   contratto interno.
//! - "raw": il programma dell'utente resta un processo persistente e
//!   scrive righe `WatcherEnvelope` complete su stdout, stesso identico
//!   contratto dei watcher integrati (vedi spawn_stdout_drain in lib.rs)
//!   — nessuna guida necessaria per chi lo conosce già.
//!
//! Lo spawn usa `tokio::process::Command` diretto (non il plugin shell di
//! Tauri, che accetta solo sidecar dichiarati a build-time): il Job
//! Object Windows già attivo (setup_kill_on_exit_job in lib.rs) copre
//! comunque anche questi processi, nessun rischio di orfani.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;

use tauri::Manager;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::{elabora_envelope_watcher, AppDataDirState, AppServer, WatcherEnvelope};

const NOME_FILE_MANIFEST: &str = "watcher.json";
const NOME_FILE_LOG: &str = "watcher.log";
// URL della guida "Creating one in the app"/"Writing the watcher script"
// sul sito — un file .url (formato "collegamento Internet" nativo di
// Windows: testo semplice, nessuna API speciale) creato accanto allo
// script di prova, così chi apre la cartella del watcher trova subito
// come scrivere il proprio script.
const URL_DOCUMENTAZIONE: &str = "https://codex-cpp.github.io/TrackFlow/documentation.html";
// Oltre questa soglia il log viene troncato tenendo solo le righe più
// recenti — un watcher a intervalli brevi attivo per mesi non deve poter
// far crescere il file all'infinito.
const LOG_MAX_BYTE: u64 = 1_000_000;
const LOG_RIGHE_DOPO_TRONCAMENTO: usize = 500;

/// Scrive una riga con timestamp nel log del singolo watcher (la sua
/// cartella, non il log globale dell'app) — richiesta esplicita: una
/// finestra di log "in tempo reale" dedicata a QUEL watcher, che copra
/// creazione, avvio, dati letti, chiusura, timeout. Best-effort: un
/// fallimento di scrittura del log non deve mai interrompere il watcher
/// vero e proprio.
fn scrivi_log_watcher(cartella: &Path, livello: &str, messaggio: &str) {
    let percorso = cartella.join(NOME_FILE_LOG);

    if let Ok(meta) = std::fs::metadata(&percorso) {
        if meta.len() > LOG_MAX_BYTE {
            if let Ok(contenuto) = std::fs::read_to_string(&percorso) {
                let ultime: Vec<&str> =
                    contenuto.lines().rev().take(LOG_RIGHE_DOPO_TRONCAMENTO).collect();
                let ridotto: String = ultime.into_iter().rev().collect::<Vec<_>>().join("\n");
                let _ = std::fs::write(&percorso, ridotto + "\n");
            }
        }
    }

    let riga = format!(
        "[{}] [{}] {}\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        livello,
        messaggio
    );
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&percorso) {
        use std::io::Write;
        let _ = file.write_all(riga.as_bytes());
    }
}

const RITENZIONE_LOG_GIORNI: i64 = 7;

/// Vero se la riga (formato scritto da `scrivi_log_watcher`, cioè
/// "[YYYY-MM-DD HH:MM:SS] [LIVELLO] messaggio") è più recente di
/// `cutoff`. Una riga che non rispetta il formato atteso viene tenuta
/// per sicurezza, invece di rischiare di perdere dati per un parsing
/// troppo rigido.
fn riga_log_recente(riga: &str, cutoff: chrono::DateTime<chrono::Local>) -> bool {
    let Some(dopo_parentesi) = riga.strip_prefix('[') else { return true };
    let Some((timestamp_str, _resto)) = dopo_parentesi.split_once(']') else { return true };
    match chrono::NaiveDateTime::parse_from_str(timestamp_str, "%Y-%m-%d %H:%M:%S") {
        Ok(naive) => match naive.and_local_timezone(chrono::Local).single() {
            Some(dt) => dt >= cutoff,
            None => true,
        },
        Err(_) => true,
    }
}

/// Elimina dai log di ogni watcher personalizzato le righe più vecchie di
/// `RITENZIONE_LOG_GIORNI` — richiesta esplicita, indipendente dal limite
/// per dimensione (`LOG_MAX_BYTE`): anche un log che resta piccolo non
/// deve accumulare voci vecchie all'infinito.
fn pulisci_log_vecchi(app_data_dir: &Path) {
    let cutoff = chrono::Local::now() - chrono::Duration::days(RITENZIONE_LOG_GIORNI);
    let base = cartella_custom_watchers(app_data_dir);
    let Ok(voci) = std::fs::read_dir(&base) else { return };
    for voce in voci.filter_map(Result::ok) {
        if !voce.path().is_dir() {
            continue;
        }
        let percorso_log = voce.path().join(NOME_FILE_LOG);
        let Ok(contenuto) = std::fs::read_to_string(&percorso_log) else { continue };
        let totale = contenuto.lines().count();
        let righe_recenti: Vec<&str> =
            contenuto.lines().filter(|riga| riga_log_recente(riga, cutoff)).collect();
        if righe_recenti.len() != totale {
            let mut nuovo_contenuto = righe_recenti.join("\n");
            if !nuovo_contenuto.is_empty() {
                nuovo_contenuto.push('\n');
            }
            let _ = std::fs::write(&percorso_log, nuovo_contenuto);
        }
    }
}

/// Avvia il controllo periodico di pulizia dei log — il primo giro
/// scatta subito all'avvio dell'app (comportamento di default di
/// `tokio::time::interval`), poi uno ogni 24 ore per tutta la durata del
/// processo.
pub(crate) fn avvia_pulizia_log_periodica(app_data_dir: PathBuf) {
    tauri::async_runtime::spawn(async move {
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(24 * 60 * 60));
        loop {
            ticker.tick().await;
            pulisci_log_vecchi(&app_data_dir);
        }
    });
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct WatcherManifest {
    name: String,
    mode: String,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    interval_seconds: Option<u64>,
    #[serde(default)]
    timeline_lane: bool,
    /// ID completo del bucket scritto da questo watcher — per modalità
    /// "interval" è sempre `custom-watcher-<id>` (convenzione fissa,
    /// calcolata alla creazione, senza suffisso hostname: import/export
    /// tra PC diversi dell'utente devono vedere lo stesso bucket, non uno
    /// per dispositivo); per modalità "raw" è quello che l'utente ha
    /// dichiarato nel proprio script (campo del form esperto). Permette al
    /// picker di Home (CustomModulePicker.vue) di collegare qualunque
    /// watcher a un modulo, non solo quelli in modalità semplificata.
    #[serde(default)]
    bucket_id: Option<String>,
    /// Larghezza (in colonne, 1-4) scelta per il modulo Home alla
    /// creazione — vive qui, non solo nelle props dell'elemento in
    /// `views.ts`, così sopravvive alla cancellazione/ricreazione del
    /// modulo: se l'utente lo toglie per sbaglio da Home, il selettore
    /// "Aggiungi modulo → Watcher personalizzato" la ritrova da qui
    /// invece di ripartire sempre da 1.
    #[serde(default)]
    grid_width: Option<u8>,
    /// Id del modello di visualizzazione scelto alla creazione (cartella
    /// dentro `watcher-templates/`, vedi elenca_modelli_visualizzazione_watcher)
    /// — `None` significa "tabella chiave/valore generica" (comportamento
    /// di sempre). Stessa ragione di grid_width: vive nel manifest, non
    /// solo nelle props del modulo Home, così sopravvive alla
    /// cancellazione/ricreazione del modulo.
    #[serde(default)]
    template_id: Option<String>,
}

#[derive(serde::Serialize)]
pub struct WatcherPersonalizzatoInfo {
    id: String,
    name: String,
    mode: String,
    timeline_lane: bool,
    running: bool,
    bucket_id: Option<String>,
    /// Campi da manifest esposti per la sezione "informazioni sviluppatore"
    /// nella pagina di dettaglio bucket (Bucket.vue) — nessuna logica in
    /// più, solo un passthrough di dati già letti dal manifest.
    interval_seconds: Option<u64>,
    command: Option<String>,
    args: Vec<String>,
    grid_width: Option<u8>,
    template_id: Option<String>,
}

/// Task in corso per ogni watcher personalizzato attivo, indicizzati per
/// id (nome cartella) — permette a `ricarica_watcher_personalizzati` di
/// capire quali fermare/avviare confrontando con una nuova scansione,
/// senza dover riavviare quelli già in esecuzione e invariati.
pub(crate) struct CustomWatcherProcesses(std::sync::Mutex<HashMap<String, tokio::task::AbortHandle>>);

impl CustomWatcherProcesses {
    pub(crate) fn new() -> Self {
        Self(std::sync::Mutex::new(HashMap::new()))
    }
}

pub(crate) fn cartella_custom_watchers(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("custom").join("watchers")
}

fn scansiona(app_data_dir: &Path) -> Vec<(String, WatcherManifest, PathBuf)> {
    let base = cartella_custom_watchers(app_data_dir);
    let Ok(voci) = std::fs::read_dir(&base) else {
        return Vec::new();
    };

    voci.filter_map(Result::ok)
        .filter(|v| v.path().is_dir())
        .filter_map(|v| {
            let id = v.file_name().to_string_lossy().to_string();
            let manifest_path = v.path().join(NOME_FILE_MANIFEST);
            let contenuto = std::fs::read_to_string(&manifest_path).ok()?;
            let manifest: WatcherManifest = match serde_json::from_str(&contenuto) {
                Ok(m) => m,
                Err(e) => {
                    log::warn!("Manifest non valido per il watcher personalizzato '{id}': {e}");
                    return None;
                }
            };
            Some((id, manifest, v.path()))
        })
        .collect()
}

/// Riga leggibile con il comando esatto che sta per essere lanciato — per
/// distinguere nel log se è TrackFlow a non riuscire ad avviare nulla
/// (interprete sbagliato/mancante) o se è lo script dell'utente a dare
/// problemi una volta partito.
fn descrivi_comando(manifest: &WatcherManifest) -> String {
    match &manifest.command {
        Some(c) => format!("{c} {}", manifest.args.join(" ")),
        None => manifest.args.join(" "),
    }
}

/// Costruisce il comando pronto per lo spawn: se `command` è assente nel
/// manifest, il primo elemento di `args` è trattato come eseguibile
/// diretto (l'utente porta il proprio binario, nessuna compilazione da
/// parte di TrackFlow in ogni caso).
fn costruisci_comando(manifest: &WatcherManifest, cartella: &Path) -> Option<Command> {
    let mut cmd = match &manifest.command {
        Some(c) => {
            let mut cmd = Command::new(c);
            cmd.args(&manifest.args);
            cmd
        }
        None => {
            let (eseguibile, resto) = manifest.args.split_first()?;
            let mut cmd = Command::new(eseguibile);
            cmd.args(resto);
            cmd
        }
    };
    cmd.current_dir(cartella)
        .kill_on_drop(true)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    nascondi_finestra_tokio(&mut cmd);
    Some(cmd)
}

/// Modalità "raw": processo persistente, righe JSON complete su stdout —
/// stesso ciclo di vita/contratto di spawn_stdout_drain per i sidecar
/// integrati, solo con un `Child` di tokio al posto di un `CommandChild`
/// di Tauri (qui non serve la distinzione stderr/evento strutturata a
/// eventi separati: entrambe le pipe sono lette riga per riga allo stesso
/// modo).
async fn esegui_raw(
    id: String,
    cartella: PathBuf,
    manifest: WatcherManifest,
    mut child: tokio::process::Child,
    server: Arc<AppServer>,
    hostname: String,
) {
    let pid = child.id();
    scrivi_log_watcher(
        &cartella,
        "INFO",
        &format!(
            "Processo avviato (modalità esperta, PID {}) — comando: {}",
            pid.map(|p| p.to_string()).unwrap_or_else(|| "?".to_string()),
            descrivi_comando(&manifest)
        ),
    );

    let stdout = match child.stdout.take() {
        Some(s) => s,
        None => {
            scrivi_log_watcher(&cartella, "ERRORE", "Impossibile agganciare lo stdout del processo");
            return;
        }
    };
    if let Some(stderr) = child.stderr.take() {
        let id_stderr = id.clone();
        let cartella_stderr = cartella.clone();
        tokio::spawn(async move {
            let mut righe = BufReader::new(stderr).lines();
            while let Ok(Some(riga)) = righe.next_line().await {
                let riga = riga.trim();
                if !riga.is_empty() {
                    log::error!(target: "watcher", "[custom:{id_stderr}] {riga}");
                    scrivi_log_watcher(&cartella_stderr, "ERRORE", riga);
                }
            }
        });
    }

    let mut righe = BufReader::new(stdout).lines();
    loop {
        match righe.next_line().await {
            Ok(Some(riga)) => {
                let riga = riga.trim();
                if riga.is_empty() {
                    continue;
                }
                match serde_json::from_str::<WatcherEnvelope>(riga) {
                    Ok(envelope) => {
                        scrivi_log_watcher(&cartella, "DATI", riga);
                        elabora_envelope_watcher(&envelope, &server, &hostname).await;
                    }
                    Err(_) => {
                        log::info!(target: "watcher", "[custom:{id}] {riga}");
                        scrivi_log_watcher(&cartella, "INFO", riga);
                    }
                }
            }
            Ok(None) => break,
            Err(e) => {
                log::error!(target: "watcher", "[custom:{id}] errore lettura stdout: {e}");
                scrivi_log_watcher(&cartella, "ERRORE", &format!("Errore lettura stdout: {e}"));
                break;
            }
        }
    }
    let _ = child.wait().await;
    log::warn!(target: "watcher", "[custom:{id}] processo terminato");
    scrivi_log_watcher(&cartella, "AVVISO", "Processo terminato");
}

/// Modalità "interval": rilancia il programma ogni `interval_seconds`,
/// legge l'ultima riga non vuota di stdout come dati grezzi (nessun
/// bucket/pulsetime richiesto all'utente) e la incapsula in un envelope
/// sintetico. Un errore in un singolo giro (JSON non valido, script che
/// fallisce, timeout) viene solo loggato — non deve mai fermare il ciclo
/// né far crashare l'app per uno script utente rotto.
async fn esegui_interval(
    id: String,
    manifest: WatcherManifest,
    cartella: PathBuf,
    server: Arc<AppServer>,
    hostname: String,
) {
    let intervallo = manifest.interval_seconds.unwrap_or(60).max(1);
    scrivi_log_watcher(
        &cartella,
        "INFO",
        &format!(
            "Avvio watcher (modalità semplificata, intervallo {intervallo}s) — comando: {}",
            descrivi_comando(&manifest)
        ),
    );
    let mut ticker = tokio::time::interval(std::time::Duration::from_secs(intervallo));
    loop {
        ticker.tick().await;

        let Some(mut cmd) = costruisci_comando(&manifest, &cartella) else {
            log::error!("[custom:{id}] manifest senza comando/eseguibile valido, salto il giro");
            scrivi_log_watcher(&cartella, "ERRORE", "Manifest senza comando/eseguibile valido, salto il giro");
            continue;
        };

        let esecuzione = tokio::time::timeout(
            std::time::Duration::from_secs(intervallo.max(10)),
            cmd.output(),
        )
        .await;

        let output = match esecuzione {
            Ok(Ok(o)) => o,
            Ok(Err(e)) => {
                log::warn!("[custom:{id}] impossibile avviare il programma: {e}");
                scrivi_log_watcher(
                    &cartella,
                    "ERRORE",
                    &format!("Impossibile avviare il programma (comando: {}): {e}", descrivi_comando(&manifest)),
                );
                continue;
            }
            Err(_) => {
                log::warn!("[custom:{id}] il programma non ha risposto entro il timeout, salto il giro");
                scrivi_log_watcher(
                    &cartella,
                    "AVVISO",
                    &format!("Timeout: il programma non ha risposto entro {}s, salto il giro", intervallo.max(10)),
                );
                continue;
            }
        };

        if !output.status.success() {
            log::warn!("[custom:{id}] il programma è uscito con errore (codice {:?})", output.status.code());
            scrivi_log_watcher(
                &cartella,
                "AVVISO",
                &format!("Il programma è uscito con errore (codice {:?})", output.status.code()),
            );
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let Some(ultima_riga) = stdout.lines().rev().find(|r| !r.trim().is_empty()) else {
            log::warn!("[custom:{id}] nessun output ricevuto in questo giro");
            scrivi_log_watcher(&cartella, "AVVISO", "Nessun output ricevuto in questo giro");
            continue;
        };

        let dati: serde_json::Value = match serde_json::from_str(ultima_riga.trim()) {
            Ok(v) => v,
            Err(e) => {
                log::warn!("[custom:{id}] output non è JSON valido ({e}): {ultima_riga}");
                scrivi_log_watcher(&cartella, "ERRORE", &format!("Output non è JSON valido ({e}): {ultima_riga}"));
                continue;
            }
        };

        scrivi_log_watcher(&cartella, "DATI", ultima_riga.trim());

        let envelope = WatcherEnvelope {
            bucket_id: format!("custom-watcher-{id}"),
            bucket_type: "custom-watcher".to_string(),
            client: id.clone(),
            op: "heartbeat".to_string(),
            pulsetime: Some((intervallo as f64) * 2.0),
            event: serde_json::json!({
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "duration": 0,
                "data": dati,
            }),
        };
        elabora_envelope_watcher(&envelope, &server, &hostname).await;
    }
}

fn avvia_uno(
    id: String,
    manifest: WatcherManifest,
    cartella: PathBuf,
    server: Arc<AppServer>,
    hostname: String,
) -> Option<tokio::task::AbortHandle> {
    match manifest.mode.as_str() {
        "raw" => {
            let mut cmd = costruisci_comando(&manifest, &cartella)?;
            let child = match cmd.spawn() {
                Ok(c) => c,
                Err(e) => {
                    log::error!("[custom:{id}] impossibile avviare il watcher (modalità raw): {e}");
                    scrivi_log_watcher(
                        &cartella,
                        "ERRORE",
                        &format!("Impossibile avviare il watcher (comando: {}): {e}", descrivi_comando(&manifest)),
                    );
                    return None;
                }
            };
            // tauri::async_runtime::spawn, non tokio::spawn: questa
            // funzione è chiamata anche da comandi Tauri sincroni (es.
            // ricarica_watcher_personalizzati), eseguiti sul thread della
            // finestra — senza un runtime Tokio "corrente" su quel
            // thread, tokio::spawn panica e manda in crash l'intera app
            // (bug reale riscontrato creando un watcher dal wizard).
            let handle = tauri::async_runtime::spawn(esegui_raw(id, cartella, manifest, child, server, hostname));
            Some(handle.inner().abort_handle())
        }
        "interval" => {
            let handle = tauri::async_runtime::spawn(esegui_interval(id, manifest, cartella, server, hostname));
            Some(handle.inner().abort_handle())
        }
        altro => {
            log::warn!("[custom:{id}] modalità sconosciuta '{altro}' nel manifest, ignorato");
            None
        }
    }
}

/// Avvia tutti i watcher personalizzati trovati su disco che non sono già
/// in esecuzione — chiamata sia all'avvio dell'app sia da
/// `ricarica_watcher_personalizzati` (rilevamento "a caldo").
pub(crate) fn spawn_custom_watchers(
    server: Arc<AppServer>,
    hostname: String,
    app_data_dir: &Path,
    tracked: &CustomWatcherProcesses,
) {
    let trovati = scansiona(app_data_dir);
    let mut guard = tracked.0.lock().unwrap();

    // Rimuove (interrompendo il task, che uccide il child grazie a
    // kill_on_drop) i watcher la cui cartella non esiste più.
    let id_trovati: std::collections::HashSet<&String> = trovati.iter().map(|(id, _, _)| id).collect();
    guard.retain(|id, handle| {
        if id_trovati.contains(id) {
            true
        } else {
            handle.abort();
            log::info!("Watcher personalizzato '{id}' rimosso (cartella non più presente)");
            false
        }
    });

    for (id, manifest, cartella) in trovati {
        if guard.contains_key(&id) {
            continue;
        }
        if let Some(handle) = avvia_uno(id.clone(), manifest, cartella, server.clone(), hostname.clone()) {
            log::info!("Watcher personalizzato '{id}' avviato");
            guard.insert(id, handle);
        }
    }
}

#[tauri::command]
pub fn elenca_watcher_personalizzati(app_handle: tauri::AppHandle) -> Vec<WatcherPersonalizzatoInfo> {
    let Some(dir) = app_handle.try_state::<AppDataDirState>() else {
        return Vec::new();
    };
    let tracked = app_handle.state::<CustomWatcherProcesses>();
    let guard = tracked.0.lock().unwrap();
    scansiona(&dir.0)
        .into_iter()
        .map(|(id, manifest, _)| WatcherPersonalizzatoInfo {
            running: guard.contains_key(&id),
            bucket_id: manifest.bucket_id.clone(),
            interval_seconds: manifest.interval_seconds,
            command: manifest.command.clone(),
            args: manifest.args.clone(),
            grid_width: manifest.grid_width,
            template_id: manifest.template_id.clone(),
            id,
            name: manifest.name,
            mode: manifest.mode,
            timeline_lane: manifest.timeline_lane,
        })
        .collect()
}

#[tauri::command]
pub fn ricarica_watcher_personalizzati(app_handle: tauri::AppHandle) -> Result<(), String> {
    let dir = app_handle
        .try_state::<AppDataDirState>()
        .ok_or_else(|| "Cartella dati non ancora pronta, riprova tra poco".to_string())?;
    let server = app_handle
        .try_state::<Arc<AppServer>>()
        .ok_or_else(|| "Server non ancora pronto, riprova tra poco".to_string())?;
    let hostname = gethostname::gethostname().to_string_lossy().to_string();
    let tracked = app_handle.state::<CustomWatcherProcesses>();
    spawn_custom_watchers(server.inner().clone(), hostname, &dir.0, &tracked);
    Ok(())
}

fn slug(nome: &str) -> String {
    let s: String = nome
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let s = s.trim_matches('-').to_string();
    if s.is_empty() {
        "watcher".to_string()
    } else {
        s
    }
}

/// Imposta l'attributo "nascosto" di Windows su `watcher.json` — i suoi
/// parametri (intervallo, riga Timeline, bucket_id...) vanno modificati
/// solo dall'app stessa, mai a mano dall'utente finale: nasconderlo
/// evita che chi apre la cartella per scrivere il proprio script si
/// ritrovi a modificare anche il manifest per sbaglio. Resta comunque
/// un file normale (leggibile/scrivibile da qualunque processo, incluso
/// questo stesso programma) — solo non compare in Esplora risorse a
/// meno che l'utente non attivi esplicitamente "Elementi nascosti".
fn nascondi_manifest(path: &Path) {
    let mut cmd = std::process::Command::new("attrib");
    cmd.args(["+h", &path.to_string_lossy()]);
    nascondi_finestra(&mut cmd);
    let _ = cmd.status();
}

/// Su Windows, impedisce al processo lanciato di aprire la sua finestra
/// console — senza questo, ogni riga di `attrib`/PowerShell/Python/ecc.
/// lanciata per i watcher personalizzati fa lampeggiare brevemente una
/// finestra nera sullo schermo (bug reale segnalato da un utente dopo
/// aver installato v0.1.12: creare un nuovo watcher apriva "diverse
/// finestre cmd" prima della conferma). No-op su altre piattaforme.
#[cfg(windows)]
fn nascondi_finestra(cmd: &mut std::process::Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    cmd.creation_flags(CREATE_NO_WINDOW);
}
#[cfg(windows)]
fn nascondi_finestra_tokio(cmd: &mut Command) {
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    cmd.creation_flags(CREATE_NO_WINDOW);
}
#[cfg(not(windows))]
fn nascondi_finestra(_cmd: &mut std::process::Command) {}
#[cfg(not(windows))]
fn nascondi_finestra_tokio(_cmd: &mut Command) {}

fn crea_cartella_univoca(base: &Path, nome: &str) -> Result<(String, PathBuf), String> {
    std::fs::create_dir_all(base).map_err(|e| e.to_string())?;
    let radice = slug(nome);
    let mut candidato = radice.clone();
    let mut n = 2;
    while base.join(&candidato).exists() {
        candidato = format!("{radice}-{n}");
        n += 1;
    }
    let percorso = base.join(&candidato);
    std::fs::create_dir_all(&percorso).map_err(|e| e.to_string())?;
    Ok((candidato, percorso))
}

const SCRIPT_INIZIALE_SEMPLIFICATA: &str = r#"# Questo script viene eseguito periodicamente da TrackFlow.
# Ad ogni esecuzione, stampa UNA riga JSON con i tuoi dati ed esce -
# TrackFlow pensa a tutto il resto (salvataggio, visualizzazione).
#
# Sostituisci i valori qui sotto con quelli che vuoi tracciare.
$dati = @{
    valore = 42
    etichetta = "esempio"
}
$dati | ConvertTo-Json -Compress
"#;

/// Crea la cartella + manifest per la modalità semplificata (wizard "a
/// prova di scemo") — l'utente scrive SOLO lo script indicato, in
/// qualunque linguaggio preferisca (basta cambiare `command`/`args` nel
/// manifest se non usa PowerShell). Ritorna il percorso della cartella
/// creata, mostrato nella schermata di conferma del wizard.
#[tauri::command]
pub fn crea_watcher_personalizzato_semplice(
    app_handle: tauri::AppHandle,
    nome: String,
    intervallo_secondi: u64,
    mostra_timeline: bool,
    larghezza_griglia: u8,
    modello_id: Option<String>,
) -> Result<String, String> {
    let dir = app_handle
        .try_state::<AppDataDirState>()
        .ok_or_else(|| "Cartella dati non ancora pronta, riprova tra poco".to_string())?;
    let base = cartella_custom_watchers(&dir.0);
    let (id, cartella) = crea_cartella_univoca(&base, &nome)?;

    std::fs::write(cartella.join("watcher.ps1"), SCRIPT_INIZIALE_SEMPLIFICATA).map_err(|e| e.to_string())?;

    // Best-effort: un collegamento cliccabile alla documentazione non è
    // essenziale al funzionamento del watcher, non deve mai far fallire
    // la creazione se la scrittura fallisse per qualche motivo.
    let _ = std::fs::write(
        cartella.join("documentation.url"),
        format!("[InternetShortcut]\r\nURL={URL_DOCUMENTAZIONE}\r\n"),
    );

    let manifest = WatcherManifest {
        name: nome,
        mode: "interval".to_string(),
        command: Some("powershell".to_string()),
        args: vec!["-NoProfile".to_string(), "-File".to_string(), "watcher.ps1".to_string()],
        interval_seconds: Some(intervallo_secondi.max(5)),
        timeline_lane: mostra_timeline,
        bucket_id: Some(format!("custom-watcher-{id}")),
        grid_width: Some(larghezza_griglia.clamp(1, 4)),
        template_id: modello_id.filter(|m| !m.trim().is_empty()),
    };
    let percorso_manifest = cartella.join(NOME_FILE_MANIFEST);
    std::fs::write(
        &percorso_manifest,
        serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    nascondi_manifest(&percorso_manifest);
    scrivi_log_watcher(
        &cartella,
        "INFO",
        &format!("Watcher creato (modalità semplificata, intervallo {}s)", intervallo_secondi.max(5)),
    );

    Ok(cartella.to_string_lossy().to_string())
}

/// Modalità esperta: l'utente fornisce direttamente il comando da
/// eseguire (un proprio eseguibile, o un interprete + script già scritti
/// altrove) — avviato tramite `cmd /C` per accettare la stessa sintassi
/// che digiterebbe in un terminale, senza dover fare parsing manuale di
/// virgolette/argomenti.
#[tauri::command]
pub fn crea_watcher_personalizzato_esperto(
    app_handle: tauri::AppHandle,
    nome: String,
    comando: String,
    bucket_id: Option<String>,
) -> Result<String, String> {
    if comando.trim().is_empty() {
        return Err("Il comando non può essere vuoto".to_string());
    }
    let dir = app_handle
        .try_state::<AppDataDirState>()
        .ok_or_else(|| "Cartella dati non ancora pronta, riprova tra poco".to_string())?;
    let base = cartella_custom_watchers(&dir.0);
    let (id, cartella) = crea_cartella_univoca(&base, &nome)?;

    let manifest = WatcherManifest {
        name: nome,
        mode: "raw".to_string(),
        command: Some("cmd".to_string()),
        args: vec!["/C".to_string(), comando],
        interval_seconds: None,
        timeline_lane: false,
        bucket_id: bucket_id.filter(|b| !b.trim().is_empty()),
        // La modalità esperta non passa dagli step di scelta
        // dimensione/modello del wizard — nessun valore a priori, il
        // selettore userà i default.
        grid_width: None,
        template_id: None,
    };
    let percorso_manifest = cartella.join(NOME_FILE_MANIFEST);
    std::fs::write(
        &percorso_manifest,
        serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    nascondi_manifest(&percorso_manifest);
    scrivi_log_watcher(&cartella, "INFO", "Watcher creato (modalità esperta)");

    let _ = id;
    Ok(cartella.to_string_lossy().to_string())
}

#[tauri::command]
pub fn apri_cartella_custom_watcher(app_handle: tauri::AppHandle, id: String) -> Result<(), String> {
    let dir = app_handle
        .try_state::<AppDataDirState>()
        .ok_or_else(|| "Cartella dati non ancora pronta, riprova tra poco".to_string())?;
    crate::folder_shortcuts::apri_in_esplora_risorse(&cartella_custom_watchers(&dir.0).join(id))
}

/// Legge il log dedicato di un watcher personalizzato (creazione, avvio,
/// dati letti, timeout, chiusura...) — il frontend lo richiama a
/// intervalli regolari per mostrare la finestra di log "in tempo reale"
/// nella pagina di dettaglio del bucket. Nessun file ancora presente non
/// è un errore: il watcher potrebbe non aver ancora scritto nulla.
#[tauri::command]
pub fn leggi_log_watcher(app_handle: tauri::AppHandle, id: String) -> Result<String, String> {
    let dir = app_handle
        .try_state::<AppDataDirState>()
        .ok_or_else(|| "Cartella dati non ancora pronta, riprova tra poco".to_string())?;
    let percorso = cartella_custom_watchers(&dir.0).join(id).join(NOME_FILE_LOG);
    match std::fs::read_to_string(&percorso) {
        Ok(contenuto) => Ok(contenuto),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(e.to_string()),
    }
}

/// Sceglie l'interprete giusto in base all'estensione del file scelto
/// dall'utente per la modalità semplificata — richiesta esplicita: non
/// solo `.ps1`, anche `.py`/`.js`/`.cmd`/`.bat` (e in generale qualunque
/// estensione riconosciuta), semplicemente rinominando il campo nei
/// dettagli. Un'estensione non riconosciuta viene comunque eseguita
/// direttamente (utile per un `.exe` già compilato): se l'interprete o
/// il runtime necessario non è installato sul PC dell'utente, il
/// fallimento finisce nel log del watcher come qualunque altro errore di
/// avvio — non è compito dell'app garantirne la presenza.
fn comando_per_file(nome_file: &str) -> (Option<String>, Vec<String>) {
    let estensione = Path::new(nome_file)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase());
    match estensione.as_deref() {
        Some("ps1") => (
            Some("powershell".to_string()),
            vec!["-NoProfile".to_string(), "-File".to_string(), nome_file.to_string()],
        ),
        Some("py") | Some("pyw") => (Some("python".to_string()), vec![nome_file.to_string()]),
        Some("js") => (Some("node".to_string()), vec![nome_file.to_string()]),
        Some("cmd") | Some("bat") => (Some("cmd".to_string()), vec!["/C".to_string(), nome_file.to_string()]),
        _ => (None, vec![nome_file.to_string()]),
    }
}

/// Cambia il file che un watcher in modalità semplificata lancia ad ogni
/// giro (per default "watcher.ps1") — richiesta esplicita: campo
/// modificabile in tabella, purché il file resti dentro la cartella del
/// watcher (nessun percorso, solo un nome file: niente `/`, `\` o `..`).
/// L'interprete usato per lanciarlo viene ricalcolato dall'estensione
/// (vedi comando_per_file). Riavvia subito il watcher con il nuovo
/// manifest, invece di aspettare il prossimo riavvio dell'app.
#[tauri::command]
pub fn imposta_file_watcher(app_handle: tauri::AppHandle, id: String, nome_file: String) -> Result<(), String> {
    let nome_file = nome_file.trim().to_string();
    if nome_file.is_empty() {
        return Err("Il nome del file non può essere vuoto".to_string());
    }
    if nome_file.contains('/') || nome_file.contains('\\') || nome_file.contains("..") {
        return Err(
            "Il file deve trovarsi nella cartella del watcher: nessun percorso, solo il nome del file".to_string(),
        );
    }

    let dir = app_handle
        .try_state::<AppDataDirState>()
        .ok_or_else(|| "Cartella dati non ancora pronta, riprova tra poco".to_string())?;
    let cartella = cartella_custom_watchers(&dir.0).join(&id);
    let percorso_manifest = cartella.join(NOME_FILE_MANIFEST);
    let contenuto = std::fs::read_to_string(&percorso_manifest).map_err(|e| e.to_string())?;
    let mut manifest: WatcherManifest = serde_json::from_str(&contenuto).map_err(|e| e.to_string())?;

    if manifest.mode != "interval" {
        return Err("Il file associato si può modificare solo per i watcher in modalità semplificata".to_string());
    }

    let (comando, args) = comando_per_file(&nome_file);
    let descrizione_interprete = comando.clone().unwrap_or_else(|| "eseguito direttamente".to_string());
    manifest.command = comando;
    manifest.args = args;

    std::fs::write(
        &percorso_manifest,
        serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    nascondi_manifest(&percorso_manifest);
    scrivi_log_watcher(
        &cartella,
        "INFO",
        &format!("File associato aggiornato: {nome_file} (interprete: {descrizione_interprete})"),
    );

    // Riavvia subito il task con il manifest aggiornato — esegui_interval
    // tiene in memoria la propria copia del manifest per tutta la vita
    // del task, quindi senza un riavvio il cambiamento resterebbe
    // invisibile finché l'app non viene chiusa e riaperta.
    let server = app_handle
        .try_state::<Arc<AppServer>>()
        .ok_or_else(|| "Server non ancora pronto, riprova tra poco".to_string())?;
    let hostname = gethostname::gethostname().to_string_lossy().to_string();
    let tracked = app_handle.state::<CustomWatcherProcesses>();
    {
        let mut guard = tracked.0.lock().unwrap();
        if let Some(handle) = guard.remove(&id) {
            handle.abort();
        }
    }
    if let Some(handle) = avvia_uno(id.clone(), manifest, cartella, server.inner().clone(), hostname) {
        let mut guard = tracked.0.lock().unwrap();
        guard.insert(id, handle);
    }

    Ok(())
}

/// Elimina un watcher personalizzato per intero: ferma il processo (se in
/// esecuzione) e cancella la sua cartella da disco. Chiamata separatamente
/// dall'eliminazione del bucket (endpoint HTTP esistente, invariato) — se
/// non si fermasse anche il processo, questo continuerebbe a mandare
/// heartbeat e a ricreare da solo il bucket appena cancellato.
#[tauri::command]
pub fn elimina_watcher_personalizzato(app_handle: tauri::AppHandle, id: String) -> Result<(), String> {
    let dir = app_handle
        .try_state::<AppDataDirState>()
        .ok_or_else(|| "Cartella dati non ancora pronta, riprova tra poco".to_string())?;

    let tracked = app_handle.state::<CustomWatcherProcesses>();
    {
        let mut guard = tracked.0.lock().unwrap();
        if let Some(handle) = guard.remove(&id) {
            handle.abort();
        }
    }

    let cartella = cartella_custom_watchers(&dir.0).join(&id);
    if cartella.exists() {
        std::fs::remove_dir_all(&cartella).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ModelloVisualizzazioneInfo {
    id: String,
    nome_it: String,
    nome_en: String,
}

/// Elenca i modelli di visualizzazione disponibili per i watcher
/// personalizzati — letti da `watcher-templates/templates.json`, una
/// risorsa bundlata con l'app (non in app_data_dir): aggiungere un nuovo
/// modello è solo una nuova sottocartella html/css/js + una riga in
/// questo json, nessun cambiamento di codice. Nessun file/json ancora
/// presente non è un errore, semplicemente nessun modello disponibile.
#[tauri::command]
pub fn elenca_modelli_visualizzazione_watcher(
    app_handle: tauri::AppHandle,
) -> Vec<ModelloVisualizzazioneInfo> {
    let Ok(resource_dir) = app_handle.path().resource_dir() else {
        return Vec::new();
    };
    let percorso = resource_dir.join("watcher-templates").join("templates.json");
    let Ok(contenuto) = std::fs::read_to_string(&percorso) else {
        return Vec::new();
    };
    serde_json::from_str(&contenuto).unwrap_or_default()
}

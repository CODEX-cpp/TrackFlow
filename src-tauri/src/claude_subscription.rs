//! Provider alternativo per la chat AI: l'abbonamento Claude (Pro/Max)
//! già attivo dell'utente, invece di una chiave API Anthropic a
//! consumo — richiesta esplicita dell'utente, vedi BLUEPRINT.md sezione
//! 44. Il login diretto a claude.ai (reverse-engineering della sessione
//! web) è stato scartato: violerebbe i Termini di Servizio di
//! Anthropic. La via legittima trovata: Claude Desktop installa un CLI
//! **Claude Code** completo e funzionante in
//! `%APPDATA%\Claude\claude-code\<versione>\claude.exe`, che riusa da
//! solo (via OAuth/keychain di sistema) il login/abbonamento già attivo
//! — nessuna password o token gestito o salvato da TrackFlow.
//!
//! Il CLI però è pensato per essere un agente di programmazione
//! generico (accesso a Bash, file, web, altri connettori MCP
//! configurati sull'account...), non un'API di chat pulita. Per
//! restringerlo agli stessi strumenti di sola lettura/scrittura
//! controllata già definiti in `agent.rs` (mai Bash/file/web):
//! - gli viene passato un server MCP minimale, ospitato IN QUESTO
//!   STESSO PROCESSO su una porta locale effimera con un token
//!   generato ad ogni chiamata (vedi `avvia` più sotto);
//! - `--strict-mcp-config` impedisce di caricare qualunque altro server
//!   MCP configurato sulla macchina (es. i connettori Gmail/Calendar/
//!   Drive visti nell'account Claude dell'utente durante l'indagine);
//! - `--allowedTools` con l'elenco ESATTO (non un pattern jolly) dei
//!   nomi `mcp__trackflow__<tool>` impedisce l'uso di qualunque
//!   strumento nativo del CLI.
//! - `--disallowedTools` nega ESPLICITAMENTE i più rischiosi (Bash,
//!   Read, Edit, Write, WebFetch, WebSearch...) come seconda rete di
//!   sicurezza — verificato dal vivo (build di prova con un server MCP
//!   finto) che senza questo il CLI in modalità `-p` nega comunque
//!   Bash/Read/Edit/Write da solo (nessun terminale a cui chiedere
//!   conferma interattiva), MA lascia passare un paio di strumenti a
//!   basso rischio non presenti nell'allowlist (`Glob`, che elenca solo
//!   NOMI di file nella cartella di lavoro corrente, mai il contenuto;
//!   `ToolSearch`, usato dal CLI stesso per risolvere lo schema di uno
//!   strumento MCP prima di chiamarlo) — innocuo dato che la cartella
//!   di lavoro dedicata (vedi sotto) non contiene mai nulla di
//!   sensibile, ma il denylist esplicito toglie ogni dubbio per il
//!   resto.
//!
//! Altra richiesta esplicita dell'utente: questa chat non deve MAI
//! comparire come sessione visibile dentro Claude Desktop o un normale
//! `claude --resume`. Le sessioni del CLI sono legate alla cartella da
//! cui partono — usando sempre una cartella di lavoro dedicata
//! (`<app-data-dir>/claude-agent-workdir`), mai aperta manualmente
//! dall'utente in Claude Code, e mai `--cloud` (che le sincronizzerebbe
//! altrove), queste sessioni restano isolate dalle sessioni normali
//! dell'utente.
//!
//! Il processo `claude.exe` stesso resta VIVO tra un messaggio e
//! l'altro della stessa conversazione (vedi `SessioneAttiva`/
//! `ClaudeDesktopState`) invece di essere riavviato ogni volta — bug
//! reale segnalato dall'utente al primo test dal vivo: un processo
//! nuovo ad ogni messaggio pagava ogni volta il costo di avvio a
//! freddo del CLI (~10s), indipendentemente da modello o livello di
//! ragionamento. Un solo processo per conversazione, pilotato via
//! `--input-format stream-json`, elimina questo costo su ogni messaggio
//! tranne il primo — e il sistema/le istruzioni/gli strumenti restano
//! caricati una volta sola, non rimandati ad ogni turno.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde_json::{json, Value};
use tauri::{AppHandle, Manager};
use tokio::io::{AsyncBufReadExt, BufReader as TokioBufReader};

use crate::agent::{self, ModelloDisponibile, RispostaAgente};
use crate::diagnostics;
use crate::AppServer;

/// Valore di `AiAgentConfig.provider` per questa opzione — confrontato
/// da `agent.rs` (send_message/list_models) per scegliere questo
/// percorso invece della chiamata HTTP diretta ad Anthropic.
pub const PROVIDER_ID: &str = "claude_desktop";

/// Nessuna chiave API per questo provider — l'elenco è statico, gli
/// alias accettati dal CLI (`claude --help`, opzione `--model`).
pub fn modelli_disponibili() -> Vec<ModelloDisponibile> {
    vec![
        ModelloDisponibile { id: "sonnet".to_string(), nome: "Claude Sonnet (consigliato)".to_string() },
        ModelloDisponibile { id: "opus".to_string(), nome: "Claude Opus".to_string() },
        ModelloDisponibile {
            id: "haiku".to_string(),
            nome: "Claude Haiku (più veloce/economico)".to_string(),
        },
    ]
}

/// Cerca `claude.exe` tra le versioni di Claude Code installate insieme
/// a Claude Desktop — la cartella può contenere più versioni insieme
/// (l'app si aggiorna da sola in background), quindi sceglie sempre la
/// più recente presente al momento invece di un percorso fisso.
pub fn trova_claude_exe() -> Option<PathBuf> {
    let appdata = std::env::var_os("APPDATA")?;
    let base = PathBuf::from(appdata).join("Claude").join("claude-code");
    let mut migliore: Option<(Vec<u64>, PathBuf)> = None;
    for voce in std::fs::read_dir(&base).ok()?.flatten() {
        let percorso = voce.path();
        if !percorso.is_dir() {
            continue;
        }
        let Some(nome) = percorso.file_name().and_then(|n| n.to_str()) else { continue };
        let versione: Vec<u64> = nome.split('.').filter_map(|p| p.parse().ok()).collect();
        if versione.is_empty() {
            continue;
        }
        let ese = percorso.join("claude.exe");
        if !ese.is_file() {
            continue;
        }
        if migliore.as_ref().map(|(v, _)| versione > *v).unwrap_or(true) {
            migliore = Some((versione, ese));
        }
    }
    migliore.map(|(_, ese)| ese)
}

/// Comando invocato dalle Impostazioni per mostrare subito (senza dover
/// prima provare a mandare un messaggio) se Claude Desktop è installata
/// e utilizzabile su questo PC.
#[tauri::command]
pub fn claude_desktop_disponibile() -> bool {
    trova_claude_exe().is_some()
}

/// Vero se il CLI Claude Code ha già completato il proprio login — bug
/// reale segnalato da un utente (portatile di lavoro, issue interna
/// 2026-09-03): avere Claude Desktop (l'app grafica) collegata al
/// proprio abbonamento NON significa che il CLI bundlato (un processo
/// separato, con una propria autenticazione OAuth) l'abbia già fatto —
/// il CLI richiede un primo avvio interattivo una tantum (scelta tema,
/// conferma di fiducia della cartella, login) prima di poter essere
/// usato in modalità automatica (`-p`) come fa questo modulo. Verifica
/// solo la presenza del file di credenziali che quel primo avvio scrive
/// — non garantisce che siano ancora valide (potrebbero essere scadute),
/// ma è lo stesso controllo che l'utente può fare da sé aprendo quella
/// cartella, e costa zero (nessun processo da avviare).
fn credenziali_presenti() -> bool {
    std::env::var_os("USERPROFILE")
        .map(|h| PathBuf::from(h).join(".claude").join(".credentials.json"))
        .map(|p| p.is_file())
        .unwrap_or(false)
}

/// Stato completo per le Impostazioni — sostituisce `claude_desktop_disponibile`
/// (lasciata per compatibilità) con le due informazioni che servono per
/// distinguere "non installata" da "installata ma non ancora autenticata"
/// e per costruire il comando pronto da copiare (vedi `AiAgentSettings.vue`).
#[derive(serde::Serialize)]
pub struct StatoClaudeDesktop {
    pub trovato: bool,
    pub autenticato: bool,
    pub percorso_exe: Option<String>,
}

#[tauri::command]
pub fn claude_desktop_stato() -> StatoClaudeDesktop {
    let percorso = trova_claude_exe();
    StatoClaudeDesktop {
        trovato: percorso.is_some(),
        autenticato: credenziali_presenti(),
        percorso_exe: percorso.map(|p| p.display().to_string()),
    }
}

/// Genera un token casuale per proteggere il server MCP locale (vedi
/// `avvia`) — nessuna dipendenza nuova solo per questo: ogni
/// `RandomState` di libreria standard è seminato dal sistema operativo
/// in modo imprevedibile, qui mescolato anche con orario/PID per
/// sicurezza. Il token vive solo per la durata di una singola chiamata
/// a Claude Code (pochi secondi/minuti), non serve altro.
fn genera_token() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut token = String::new();
    for _ in 0..4 {
        let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
        let adesso = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        hasher.write_u128(adesso);
        hasher.write_u32(std::process::id());
        token.push_str(&format!("{:016x}", hasher.finish()));
    }
    token
}

/// Server MCP minimale (JSON-RPC 2.0 su HTTP, un solo endpoint) che
/// espone ESATTAMENTE gli stessi strumenti già definiti in `agent.rs`
/// (`definisci_strumenti`/`esegui_strumento`) — nessuna logica
/// duplicata, solo un adattamento del formato (Anthropic `input_schema`
/// → MCP `inputSchema`, e viceversa per il risultato). Bind solo su
/// `127.0.0.1`, porta effimera assegnata dal sistema operativo, protetto
/// da un token per-chiamata: vive solo per la durata di UNA invocazione
/// di `claude.exe`, poi viene chiuso (vedi `Drop`).
pub struct BridgeMcp {
    porta: u16,
    token: String,
    esecuzione: Arc<AtomicBool>,
    join: Option<std::thread::JoinHandle<()>>,
}

impl BridgeMcp {
    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}/mcp", self.porta)
    }

    pub fn token(&self) -> &str {
        &self.token
    }
}

impl Drop for BridgeMcp {
    fn drop(&mut self) {
        self.esecuzione.store(false, Ordering::SeqCst);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

pub fn avvia(
    server: Arc<AppServer>,
    app_data_dir: PathBuf,
    hostname: String,
) -> std::io::Result<BridgeMcp> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    listener.set_nonblocking(true)?;
    let porta = listener.local_addr()?.port();
    let token = genera_token();
    let esecuzione = Arc::new(AtomicBool::new(true));

    let token_thread = token.clone();
    let esecuzione_thread = esecuzione.clone();
    let join = std::thread::spawn(move || {
        while esecuzione_thread.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, _)) => {
                    let _ = stream.set_nonblocking(false);
                    gestisci_connessione(stream, &server, &app_data_dir, &hostname, &token_thread);
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
                Err(_) => {
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
            }
        }
    });

    Ok(BridgeMcp { porta, token, esecuzione, join: Some(join) })
}

/// Legge una richiesta HTTP/1.1 minimale (riga di richiesta ignorata a
/// parte il corpo — un solo endpoint, non serve instradare per path) da
/// una connessione grezza: intestazioni fino alla riga vuota, poi
/// esattamente `Content-Length` byte di corpo.
fn leggi_richiesta_http(stream: &TcpStream) -> Option<(HashMap<String, String>, Vec<u8>)> {
    let mut reader = BufReader::new(stream.try_clone().ok()?);
    let mut prima_riga = String::new();
    if reader.read_line(&mut prima_riga).ok()? == 0 {
        return None;
    }
    let mut intestazioni = HashMap::new();
    loop {
        let mut riga = String::new();
        let n = reader.read_line(&mut riga).ok()?;
        if n == 0 || riga.trim().is_empty() {
            break;
        }
        if let Some((chiave, valore)) = riga.split_once(':') {
            intestazioni.insert(chiave.trim().to_lowercase(), valore.trim().to_string());
        }
    }
    let lunghezza: usize = intestazioni.get("content-length").and_then(|v| v.parse().ok()).unwrap_or(0);
    let mut corpo = vec![0u8; lunghezza];
    if lunghezza > 0 {
        reader.read_exact(&mut corpo).ok()?;
    }
    Some((intestazioni, corpo))
}

fn scrivi_risposta_http(stream: &mut TcpStream, codice: u16, corpo: &[u8]) {
    let stato = match codice {
        200 => "200 OK",
        202 => "202 Accepted",
        400 => "400 Bad Request",
        401 => "401 Unauthorized",
        _ => "500 Internal Server Error",
    };
    let intestazione = format!(
        "HTTP/1.1 {stato}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        corpo.len()
    );
    let _ = stream.write_all(intestazione.as_bytes());
    let _ = stream.write_all(corpo);
    let _ = stream.flush();
}

fn gestisci_connessione(
    mut stream: TcpStream,
    server: &Arc<AppServer>,
    app_data_dir: &Path,
    hostname: &str,
    token_atteso: &str,
) {
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(10)));
    let Some((intestazioni, corpo)) = leggi_richiesta_http(&stream) else { return };

    let autorizzato = intestazioni
        .get("authorization")
        .map(|v| v == &format!("Bearer {token_atteso}"))
        .unwrap_or(false);
    if !autorizzato {
        scrivi_risposta_http(&mut stream, 401, b"{}");
        return;
    }

    let Ok(richiesta) = serde_json::from_slice::<Value>(&corpo) else {
        scrivi_risposta_http(&mut stream, 400, b"{}");
        return;
    };

    // Notifica JSON-RPC (nessun "id", es. "notifications/initialized") —
    // nessuna risposta prevista dal protocollo, solo un 202 per chiudere
    // la richiesta HTTP in modo pulito.
    let Some(id) = richiesta.get("id").cloned() else {
        scrivi_risposta_http(&mut stream, 202, b"");
        return;
    };
    let metodo = richiesta["method"].as_str().unwrap_or("").to_string();
    let parametri = richiesta["params"].clone();

    let inizio = std::time::Instant::now();
    let risultato =
        tauri::async_runtime::block_on(gestisci_metodo(&metodo, &parametri, server, app_data_dir, hostname));
    diagnostics::scrivi(
        "claude_desktop_mcp_richiesta",
        json!({
            "metodo": metodo,
            "nome_strumento": parametri["name"].as_str(),
            "esito": if risultato.is_ok() { "ok" } else { "errore" },
            "durata_ms": inizio.elapsed().as_millis(),
        }),
    );

    let risposta_jsonrpc = match risultato {
        Ok(v) => json!({ "jsonrpc": "2.0", "id": id, "result": v }),
        Err((codice, messaggio)) => {
            json!({ "jsonrpc": "2.0", "id": id, "error": { "code": codice, "message": messaggio } })
        }
    };
    let corpo_risposta = serde_json::to_vec(&risposta_jsonrpc).unwrap_or_default();
    scrivi_risposta_http(&mut stream, 200, &corpo_risposta);
}

async fn gestisci_metodo(
    metodo: &str,
    parametri: &Value,
    server: &Arc<AppServer>,
    app_data_dir: &Path,
    hostname: &str,
) -> Result<Value, (i64, String)> {
    match metodo {
        "initialize" => Ok(json!({
            "protocolVersion": "2025-06-18",
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "trackflow", "version": "1.0.0" },
        })),
        "tools/list" => {
            let strumenti: Vec<Value> = agent::definisci_strumenti()
                .into_iter()
                .map(|t| {
                    json!({
                        "name": t["name"],
                        "description": t["description"],
                        "inputSchema": t["input_schema"],
                    })
                })
                .collect();
            Ok(json!({ "tools": strumenti }))
        }
        "tools/call" => {
            let nome = parametri["name"].as_str().unwrap_or("").to_string();
            let argomenti = parametri.get("arguments").cloned().unwrap_or_else(|| json!({}));
            let risultato = agent::esegui_strumento(server, app_data_dir, hostname, &nome, &argomenti).await;
            Ok(json!({ "content": [ { "type": "text", "text": risultato.to_string() } ] }))
        }
        "ping" => Ok(json!({})),
        altro => Err((-32601, format!("metodo sconosciuto: {altro}"))),
    }
}

/// Variante generica di `avvia`/`gestisci_connessione`, per chi non è la
/// chat (agent.rs, dati letti dal database vero) ma comunque ha bisogno
/// di esporre a Claude Code un piccolo set di strumenti su misura —
/// usata dalla categorizzazione automatica (vedi
/// `esegui_task_una_tantum` più sotto): il chiamante fornisce il
/// proprio elenco di strumenti (stesso formato Anthropic `input_schema`
/// di `agent::definisci_strumenti`) e una funzione SINCRONA per
/// eseguirli — nessun `Arc<AppServer>` o accesso a dati esterni
/// impliciti qui, solo quello che il chiamante cattura nella closure
/// (es. uno stato condiviso dietro `Arc<Mutex<..>>`).
fn avvia_generico(
    strumenti: Vec<Value>,
    gestore: impl Fn(&str, &Value) -> Value + Send + Sync + 'static,
) -> std::io::Result<BridgeMcp> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    listener.set_nonblocking(true)?;
    let porta = listener.local_addr()?.port();
    let token = genera_token();
    let esecuzione = Arc::new(AtomicBool::new(true));
    let strumenti = Arc::new(strumenti);
    let gestore = Arc::new(gestore);

    let token_thread = token.clone();
    let esecuzione_thread = esecuzione.clone();
    let join = std::thread::spawn(move || {
        while esecuzione_thread.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, _)) => {
                    let _ = stream.set_nonblocking(false);
                    gestisci_connessione_generica(stream, &strumenti, gestore.as_ref(), &token_thread);
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
                Err(_) => {
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
            }
        }
    });

    Ok(BridgeMcp { porta, token, esecuzione, join: Some(join) })
}

fn gestisci_connessione_generica(
    mut stream: TcpStream,
    strumenti: &[Value],
    gestore: &(dyn Fn(&str, &Value) -> Value + Send + Sync),
    token_atteso: &str,
) {
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(10)));
    let Some((intestazioni, corpo)) = leggi_richiesta_http(&stream) else { return };

    let autorizzato = intestazioni
        .get("authorization")
        .map(|v| v == &format!("Bearer {token_atteso}"))
        .unwrap_or(false);
    if !autorizzato {
        scrivi_risposta_http(&mut stream, 401, b"{}");
        return;
    }

    let Ok(richiesta) = serde_json::from_slice::<Value>(&corpo) else {
        scrivi_risposta_http(&mut stream, 400, b"{}");
        return;
    };
    let Some(id) = richiesta.get("id").cloned() else {
        scrivi_risposta_http(&mut stream, 202, b"");
        return;
    };
    let metodo = richiesta["method"].as_str().unwrap_or("");
    let parametri = &richiesta["params"];

    let risultato: Result<Value, (i64, String)> = match metodo {
        "initialize" => Ok(json!({
            "protocolVersion": "2025-06-18",
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "trackflow", "version": "1.0.0" },
        })),
        "tools/list" => {
            let elenco: Vec<Value> = strumenti
                .iter()
                .map(|t| {
                    json!({
                        "name": t["name"],
                        "description": t["description"],
                        "inputSchema": t["input_schema"],
                    })
                })
                .collect();
            Ok(json!({ "tools": elenco }))
        }
        "tools/call" => {
            let nome = parametri["name"].as_str().unwrap_or("");
            let argomenti = parametri.get("arguments").cloned().unwrap_or_else(|| json!({}));
            let risultato = gestore(nome, &argomenti);
            Ok(json!({ "content": [ { "type": "text", "text": risultato.to_string() } ] }))
        }
        "ping" => Ok(json!({})),
        altro => Err((-32601, format!("metodo sconosciuto: {altro}"))),
    };

    let risposta_jsonrpc = match risultato {
        Ok(v) => json!({ "jsonrpc": "2.0", "id": id, "result": v }),
        Err((codice, messaggio)) => {
            json!({ "jsonrpc": "2.0", "id": id, "error": { "code": codice, "message": messaggio } })
        }
    };
    let corpo_risposta = serde_json::to_vec(&risposta_jsonrpc).unwrap_or_default();
    scrivi_risposta_http(&mut stream, 200, &corpo_risposta);
}

// Seconda rete di sicurezza esplicita accanto ad --allowedTools — vedi
// il commento in cima al file per la verifica dal vivo che ha motivato
// questa scelta. "ToolSearch" è qui per un motivo diverso dagli altri
// (non di sicurezza, di prestazioni): richiesta esplicita dell'utente
// dopo aver letto il log — di sua natura Claude Code tratta strumenti
// MCP come "da scoprire", facendo un giro di ricerca (ToolSearch, un
// intero round-trip di rete) PRIMA di chiamarli, anche se lo schema
// completo è già stato mandato per intero in tools/list. Verificato dal
// vivo (server MCP di prova): negando esplicitamente ToolSearch, il
// modello chiama lo strumento direttamente usando lo schema già
// ricevuto — stessa risposta corretta, un giro di rete in meno (da 3-4
// a 2 turni, tempo dentro l'API sceso da ~18s a meno di 4s in quel
// test). Non è un compromesso sulla sicurezza: ToolSearch è solo un
// meccanismo di scoperta, non un modo per accedere a dati/file oltre a
// quelli già esposti dai nostri strumenti MCP.
const STRUMENTI_NATIVI_VIETATI: &str =
    "Bash,Read,Edit,Write,NotebookEdit,WebFetch,WebSearch,PowerShell,Task,SendMessage,PushNotification,RemoteTrigger,CronCreate,CronDelete,CronList,EnterWorktree,ExitWorktree,DesignSync,ToolSearch";

/// Un processo Claude Code TENUTO VIVO tra un messaggio e l'altro della
/// stessa conversazione — bug reale segnalato dall'utente al primo test
/// dal vivo: aprire un processo NUOVO ad ogni messaggio (avvio a
/// freddo del CLI, ogni volta) costava ~10s anche per un semplice
/// scambio, indipendentemente dal modello o dal livello di ragionamento.
/// Misurato dal vivo con un giro di prova: 1° turno di un processo
/// persistente ~3s (avvio compreso), turni successivi sullo STESSO
/// processo ~2s — la differenza è quasi tutta il costo di avvio del
/// CLI, pagato una volta sola invece che ad ogni messaggio.
///
/// Il bridge MCP (`_bridge`) resta vivo per tutta la sessione, non solo
/// per un turno — stessa porta/token per l'intera conversazione.
struct SessioneAttiva {
    figlio: tokio::process::Child,
    stdin: tokio::process::ChildStdin,
    righe_stdout: tokio::io::Lines<TokioBufReader<tokio::process::ChildStdout>>,
    _bridge: BridgeMcp,
}

impl Drop for SessioneAttiva {
    fn drop(&mut self) {
        let _ = self.figlio.start_kill();
    }
}

/// Stato Tauri dedicato (gestito con `app.manage(...)` in lib.rs, come
/// `AiAgentState`) — tenuto separato invece di infilarlo dentro
/// `AiAgentState` per non far dipendere `agent.rs` da questo modulo.
/// `std::sync::Mutex` (non async) apposta: la sessione viene SEMPRE
/// tolta dal Mutex prima di qualunque `.await` (vedi `invia_messaggio`),
/// mai tenuta a cavallo di un punto di sospensione asincrono.
pub struct ClaudeDesktopState {
    sessione: std::sync::Mutex<Option<SessioneAttiva>>,
}

impl ClaudeDesktopState {
    pub fn new() -> Self {
        Self { sessione: std::sync::Mutex::new(None) }
    }
}

impl Default for ClaudeDesktopState {
    fn default() -> Self {
        Self::new()
    }
}

/// Chiude (termina) la sessione persistente, se ce n'è una — chiamata
/// da `agent::ai_agent_new_conversation` (pulsante "nuova conversazione"
/// / reset dopo inattività): il prossimo messaggio ne aprirà una nuova,
/// da zero, invece di continuare quella vecchia.
pub fn termina_sessione(stato: &ClaudeDesktopState) {
    *stato.sessione.lock().unwrap() = None;
}

/// Prepara la cartella di lavoro dedicata e il file di configurazione
/// MCP per il bridge appena avviato — fattorizzato fuori da
/// `avvia_sessione` solo per leggibilità.
fn prepara_cartella_e_config(
    app_data_dir: &Path,
    nome_cartella: &str,
    bridge: &BridgeMcp,
) -> Result<PathBuf, String> {
    // Cartella dedicata — vedi il commento in cima al file: MAI la
    // cartella vera di un progetto dell'utente, così questa
    // conversazione non compare mai tra le sue sessioni normali. Il
    // nome è parametrizzato (da "claude-agent-workdir" in poi) perché
    // riusato anche dalla categorizzazione automatica (vedi
    // `esegui_task_una_tantum`), che deve restare in una cartella
    // separata dalla chat: le due potrebbero girare in concomitanza,
    // sovrascriversi a vicenda lo stesso mcp-config.json altrimenti.
    let cartella_lavoro = app_data_dir.join(nome_cartella);
    std::fs::create_dir_all(&cartella_lavoro)
        .map_err(|e| format!("Impossibile preparare la cartella di lavoro: {e}"))?;

    let percorso_config = cartella_lavoro.join("mcp-config.json");
    let config_mcp = json!({
        "mcpServers": {
            "trackflow": {
                "type": "http",
                "url": bridge.url(),
                "headers": { "Authorization": format!("Bearer {}", bridge.token()) },
            }
        }
    });
    std::fs::write(&percorso_config, serde_json::to_vec(&config_mcp).map_err(|e| e.to_string())?)
        .map_err(|e| format!("Impossibile scrivere la configurazione MCP: {e}"))?;

    Ok(cartella_lavoro)
}

/// Avvia un nuovo processo Claude Code persistente (`--input-format
/// stream-json --output-format stream-json`, vedi il commento su
/// `SessioneAttiva`) — un solo turno viene mandato subito dopo tramite
/// `manda_turno`, i successivi riusano lo stesso processo.
async fn avvia_sessione(
    server: Arc<AppServer>,
    app_data_dir: PathBuf,
    hostname: String,
    claude_exe: &Path,
    model: &str,
) -> Result<SessioneAttiva, String> {
    let inizio_totale = std::time::Instant::now();
    diagnostics::scrivi(
        "claude_desktop_avvio_sessione_iniziato",
        json!({ "claude_exe": claude_exe.display().to_string(), "modello": model }),
    );

    let bridge = avvia(server, app_data_dir.clone(), hostname)
        .map_err(|e| format!("Impossibile avviare il collegamento locale per gli strumenti: {e}"))?;
    diagnostics::scrivi("claude_desktop_bridge_mcp_avviato", json!({ "url": bridge.url() }));
    let cartella_lavoro = prepara_cartella_e_config(&app_data_dir, "claude-agent-workdir", &bridge)?;

    // Elenco ESATTO (non un pattern jolly tipo "mcp__trackflow__*",
    // vedi il commento in cima al file) — fail-closed: solo questi nomi
    // sono utilizzabili, qualunque strumento nativo del CLI resta negato.
    let strumenti_consentiti: Vec<String> = agent::definisci_strumenti()
        .iter()
        .filter_map(|t| t["name"].as_str())
        .map(|n| format!("mcp__trackflow__{n}"))
        .collect();

    let mut comando = tokio::process::Command::new(claude_exe);
    comando
        .current_dir(&cartella_lavoro)
        .arg("-p")
        .arg("--input-format")
        .arg("stream-json")
        .arg("--output-format")
        .arg("stream-json")
        .arg("--verbose")
        .arg("--system-prompt")
        .arg(agent::system_prompt())
        .arg("--mcp-config")
        .arg(cartella_lavoro.join("mcp-config.json"))
        .arg("--strict-mcp-config")
        .arg("--allowedTools")
        .arg(strumenti_consentiti.join(","))
        .arg("--disallowedTools")
        .arg(STRUMENTI_NATIVI_VIETATI)
        // "low" — richieste come le nostre (interrogare dati già
        // strutturati con strumenti dal significato chiaro) non
        // beneficiano del ragionamento esteso di un livello più alto,
        // che nei test dal vivo si è visto pesare parecchio sui tempi
        // di risposta.
        .arg("--effort")
        .arg("low")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    // claude.exe è un binario Node — su Windows, senza questo flag,
    // avviarlo apre per un istante una finestra console visibile (bug
    // reale segnalato dall'utente al primo test dal vivo). Stesso flag
    // già usato altrove in questo codice per gli stessi motivi (vedi
    // CREATE_NO_WINDOW nei watcher sidecar).
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        comando.creation_flags(CREATE_NO_WINDOW);
    }
    // Difesa in profondità oltre al fix lato UI (AiAgentSettings.vue):
    // gli id modello di "anthropic" (es. "claude-sonnet-4-5-20250929",
    // con data) e di questo provider (alias corti, vedi
    // `modelli_disponibili`) vivono in spazi diversi — un valore
    // rimasto da un cambio di provider precedente (bug reale trovato
    // dal log diagnostico: passava comunque al CLI senza errore, solo
    // con un modello diverso da quello inteso) viene qui ignorato invece
    // che passato alla cieca.
    let modello_valido = modelli_disponibili().iter().any(|m| m.id == model.trim());
    if modello_valido {
        comando.arg("--model").arg(model.trim());
    } else if !model.trim().is_empty() {
        diagnostics::scrivi(
            "claude_desktop_modello_non_valido_ignorato",
            json!({ "modello_ricevuto": model.trim() }),
        );
    }

    diagnostics::scrivi(
        "claude_desktop_avvio_processo",
        json!({
            "cartella_lavoro": cartella_lavoro.display().to_string(),
            "strumenti_consentiti": strumenti_consentiti,
        }),
    );
    let inizio_spawn = std::time::Instant::now();
    let mut figlio = comando.spawn().map_err(|e| format!("Impossibile avviare Claude Code: {e}"))?;
    let pid = figlio.id();
    let stdin = figlio.stdin.take().ok_or_else(|| "nessun stdin verso Claude Code".to_string())?;
    let stdout = figlio.stdout.take().ok_or_else(|| "nessun output da Claude Code".to_string())?;
    let righe_stdout = TokioBufReader::new(stdout).lines();

    // Lo stderr va SVUOTATO continuamente in background, non solo
    // aperto — bug potenziale reale: una pipe "piped" mai letta si
    // riempie (il buffer del sistema operativo è di poche decine di KB)
    // e a quel punto il processo figlio si blocca al primo tentativo di
    // scriverci sopra, finché qualcuno non la svuota. Ogni riga finisce
    // comunque nel log diagnostico (utile per capire cosa logga
    // internamente il CLI), non solo scartata.
    if let Some(stderr) = figlio.stderr.take() {
        tauri::async_runtime::spawn(async move {
            let mut righe_stderr = TokioBufReader::new(stderr).lines();
            while let Ok(Some(riga)) = righe_stderr.next_line().await {
                diagnostics::scrivi("claude_desktop_stderr", json!({ "riga": riga }));
            }
        });
    }

    diagnostics::scrivi(
        "claude_desktop_processo_avviato",
        json!({
            "pid": pid,
            "durata_spawn_ms": inizio_spawn.elapsed().as_millis(),
            "durata_totale_avvio_sessione_ms": inizio_totale.elapsed().as_millis(),
        }),
    );

    Ok(SessioneAttiva { figlio, stdin, righe_stdout, _bridge: bridge })
}

/// Manda un singolo turno (un messaggio utente) sulla sessione già
/// aperta e legge gli eventi di risposta finché non arriva il "result"
/// di QUESTO turno — il processo resta vivo dopo, pronto per il turno
/// successivo (non viene chiuso qui).
async fn manda_turno(sessione: &mut SessioneAttiva, testo: &str) -> Result<RispostaAgente, String> {
    use tokio::io::AsyncWriteExt;

    let inizio_turno = std::time::Instant::now();
    diagnostics::scrivi("claude_desktop_messaggio_inviato", json!({ "testo": testo }));

    let messaggio = json!({ "type": "user", "message": { "role": "user", "content": testo } });
    let mut riga = serde_json::to_string(&messaggio).map_err(|e| e.to_string())?;
    riga.push('\n');
    sessione
        .stdin
        .write_all(riga.as_bytes())
        .await
        .map_err(|e| format!("Impossibile mandare il messaggio a Claude Code: {e}"))?;
    sessione.stdin.flush().await.map_err(|e| format!("Impossibile mandare il messaggio a Claude Code: {e}"))?;
    diagnostics::scrivi(
        "claude_desktop_messaggio_scritto_su_stdin",
        json!({ "durata_ms": inizio_turno.elapsed().as_millis() }),
    );

    let mut strumenti_usati = Vec::new();
    let (testo_finale, errore_finale) = loop {
        // Tetto di sicurezza: se Claude Code smette di rispondere del
        // tutto (processo bloccato, bridge irraggiungibile) non deve
        // restare in attesa per sempre — la sessione viene comunque
        // scartata dal chiamante in caso di errore, così il messaggio
        // successivo ne apre una pulita.
        let inizio_attesa_riga = std::time::Instant::now();
        let prossima_riga = tokio::time::timeout(std::time::Duration::from_secs(120), sessione.righe_stdout.next_line())
            .await
            .map_err(|_| {
                diagnostics::scrivi(
                    "claude_desktop_timeout",
                    json!({ "atteso_da_ms": inizio_turno.elapsed().as_millis() }),
                );
                "Claude Code non ha risposto in tempo.".to_string()
            })?
            .map_err(|e| format!("Errore leggendo la risposta di Claude Code: {e}"))?;
        let Some(riga) = prossima_riga else {
            diagnostics::scrivi(
                "claude_desktop_stdout_chiuso",
                json!({ "dopo_ms": inizio_turno.elapsed().as_millis() }),
            );
            return Err("La sessione con Claude Code si è interrotta inaspettatamente.".to_string());
        };
        let Ok(evento) = serde_json::from_str::<Value>(&riga) else {
            diagnostics::scrivi("claude_desktop_riga_non_json", json!({ "riga": riga }));
            continue;
        };
        let tipo_evento = evento["type"].as_str().unwrap_or("?").to_string();
        diagnostics::scrivi(
            "claude_desktop_evento_ricevuto",
            json!({
                "tipo": tipo_evento,
                "attesa_ms": inizio_attesa_riga.elapsed().as_millis(),
                "trascorso_dal_messaggio_ms": inizio_turno.elapsed().as_millis(),
            }),
        );
        match evento["type"].as_str() {
            Some("assistant") => {
                if let Some(blocchi) = evento["message"]["content"].as_array() {
                    for blocco in blocchi {
                        if blocco["type"] == "tool_use" {
                            if let Some(nome) = blocco["name"].as_str() {
                                let nome_pulito = nome.strip_prefix("mcp__trackflow__").unwrap_or(nome);
                                strumenti_usati.push(nome_pulito.to_string());
                                diagnostics::scrivi(
                                    "claude_desktop_strumento_chiamato",
                                    json!({ "nome": nome_pulito, "input": blocco["input"] }),
                                );
                            }
                        } else if blocco["type"] == "thinking" {
                            diagnostics::scrivi("claude_desktop_thinking", json!({}));
                        } else if blocco["type"] == "text" {
                            diagnostics::scrivi(
                                "claude_desktop_testo_parziale",
                                json!({ "testo": blocco["text"] }),
                            );
                        }
                    }
                }
            }
            Some("result") => {
                let testo = evento["result"].as_str().map(str::to_string);
                let errore = evento["is_error"].as_bool().unwrap_or(false);
                diagnostics::scrivi(
                    "claude_desktop_risultato_finale",
                    json!({
                        "testo": testo,
                        "errore": errore,
                        "num_turns": evento["num_turns"],
                        "duration_api_ms": evento["duration_api_ms"],
                        "duration_ms": evento["duration_ms"],
                        "durata_totale_lato_trackflow_ms": inizio_turno.elapsed().as_millis(),
                        "strumenti_usati": strumenti_usati,
                    }),
                );
                break (testo, errore);
            }
            _ => {}
        }
    };

    let testo_finale = testo_finale.ok_or_else(|| {
        "Nessuna risposta da Claude Code — verifica che Claude Desktop sia installata e connessa.".to_string()
    })?;
    if errore_finale {
        return Err(traduci_errore_cli(&testo_finale));
    }
    Ok(RispostaAgente { testo: testo_finale, strumenti_usati })
}

/// Marcatore stabile (non un messaggio in italiano) per il caso "CLI non
/// autenticato" — bug reale segnalato da un utente (portatile di
/// lavoro): Claude Desktop era collegata regolarmente, ma il CLI
/// bundlato non aveva mai completato il proprio primo avvio (vedi
/// `credenziali_presenti`), e il messaggio grezzo che restituisce ("Not
/// logged in · Please run /login") non significa nulla per chi non sa
/// cos'è un CLI o un comando "/login". Un CODICE invece di una frase
/// già in italiano: il testo vero e proprio (con azione cliccabile verso
/// Impostazioni) lo mostra il frontend, che sa già gestire lingua IT/EN
/// — vedi `AiChatWidget.vue`.
pub const ERRORE_NON_AUTENTICATO: &str = "CLAUDE_DESKTOP_NON_AUTENTICATO";

/// Traduce l'errore grezzo del CLI — solo il pattern "non autenticato"
/// viene riconosciuto e sostituito col marcatore sopra; qualunque altro
/// errore del CLI passa invariato (meglio il testo originale, utile per
/// una segnalazione, che nasconderlo dietro un messaggio generico
/// sbagliato).
fn traduci_errore_cli(testo: &str) -> String {
    if testo.to_lowercase().contains("not logged in") {
        ERRORE_NON_AUTENTICATO.to_string()
    } else {
        testo.to_string()
    }
}

/// Punto di ingresso da `agent::ai_agent_send_message` quando il
/// provider configurato è questo — vedi il commento in cima al file per
/// il design completo e su `SessioneAttiva` per il perché di un
/// processo persistente invece di uno per messaggio.
pub async fn invia_messaggio(
    app_handle: &AppHandle,
    server: Arc<AppServer>,
    app_data_dir: PathBuf,
    hostname: String,
    testo: String,
    model: String,
) -> Result<RispostaAgente, String> {
    let claude_exe = trova_claude_exe().ok_or_else(|| {
        "Claude Desktop non trovata (o senza Claude Code aggiornato) su questo PC — installala, accedi col tuo abbonamento e riprova.".to_string()
    })?;
    let stato = app_handle.state::<Arc<ClaudeDesktopState>>();

    // Tolta dal Mutex PRIMA di qualunque `.await` qui sotto — un
    // MutexGuard std non è Send tra punti di sospensione asincroni.
    let mut sessione_esistente = stato.sessione.lock().unwrap().take();

    // Riusa la sessione già aperta SOLO se il processo risulta ancora
    // vivo — se nel frattempo è morto da solo (crash, chiusura
    // imprevista), se ne apre una pulita invece di riusare una pipe
    // ormai rotta.
    let viva = matches!(sessione_esistente.as_mut().map(|s| s.figlio.try_wait()), Some(Ok(None)));
    diagnostics::scrivi(
        "claude_desktop_invia_messaggio",
        json!({
            "sessione_esistente": sessione_esistente.is_some(),
            "sessione_viva": viva,
        }),
    );
    let mut sessione = if viva {
        sessione_esistente.unwrap()
    } else {
        avvia_sessione(server, app_data_dir, hostname, &claude_exe, &model).await?
    };

    let inizio_giro = std::time::Instant::now();
    let esito = manda_turno(&mut sessione, &testo).await;
    diagnostics::scrivi(
        "claude_desktop_giro_completato",
        json!({
            "esito": if esito.is_ok() { "ok" } else { "errore" },
            "durata_totale_ms": inizio_giro.elapsed().as_millis(),
        }),
    );
    match esito {
        Ok(risposta) => {
            // Rimessa a disposizione per il prossimo messaggio SOLO se
            // questo turno è andato a buon fine.
            *stato.sessione.lock().unwrap() = Some(sessione);
            Ok(risposta)
        }
        Err(e) => Err(e),
    }
}

/// Task una-tantum (non conversazionale, mai tenuto vivo tra una
/// chiamata e l'altra) verso Claude Code — usata dalla categorizzazione
/// automatica (categorization.rs) per funzionare anche quando il
/// provider configurato è questo invece di "anthropic": a differenza
/// della chat (`invia_messaggio`, sessione persistente, strumenti fissi
/// di agent.rs), qui il chiamante fornisce il proprio elenco di
/// strumenti e la propria funzione sincrona per eseguirli (vedi
/// `avvia_generico`) — un bridge MCP e un processo claude.exe nuovi di
/// zecca vengono creati per QUESTA chiamata e chiusi subito dopo l'unica
/// risposta (nessun riuso, a differenza di `ClaudeDesktopState`): un
/// task di categorizzazione è raro (solo quando compare un'app nuova)
/// e non ha bisogno del vantaggio di un processo caldo che invece conta
/// per la chat, dove i messaggi si susseguono rapidamente.
pub async fn esegui_task_una_tantum(
    model: &str,
    system_prompt: &str,
    messaggio: &str,
    strumenti: Vec<Value>,
    gestore: impl Fn(&str, &Value) -> Value + Send + Sync + 'static,
    nome_sottocartella: &str,
    app_data_dir: &Path,
) -> Result<RispostaAgente, String> {
    let claude_exe = trova_claude_exe().ok_or_else(|| {
        "Claude Desktop non trovata (o senza Claude Code aggiornato) su questo PC.".to_string()
    })?;

    let bridge = avvia_generico(strumenti.clone(), gestore)
        .map_err(|e| format!("Impossibile avviare il collegamento locale per gli strumenti: {e}"))?;
    let cartella_lavoro = prepara_cartella_e_config(app_data_dir, nome_sottocartella, &bridge)?;

    // Elenco ESATTO — stessa ragione fail-closed di avvia_sessione.
    let strumenti_consentiti: Vec<String> = strumenti
        .iter()
        .filter_map(|t| t["name"].as_str())
        .map(|n| format!("mcp__trackflow__{n}"))
        .collect();

    let mut comando = tokio::process::Command::new(&claude_exe);
    comando
        .current_dir(&cartella_lavoro)
        .arg("-p")
        .arg("--input-format")
        .arg("stream-json")
        .arg("--output-format")
        .arg("stream-json")
        .arg("--verbose")
        .arg("--system-prompt")
        .arg(system_prompt)
        .arg("--mcp-config")
        .arg(cartella_lavoro.join("mcp-config.json"))
        .arg("--strict-mcp-config")
        .arg("--allowedTools")
        .arg(strumenti_consentiti.join(","))
        .arg("--disallowedTools")
        .arg(STRUMENTI_NATIVI_VIETATI)
        .arg("--effort")
        .arg("low")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        comando.creation_flags(CREATE_NO_WINDOW);
    }
    let modello_valido = modelli_disponibili().iter().any(|m| m.id == model.trim());
    if modello_valido {
        comando.arg("--model").arg(model.trim());
    }

    diagnostics::scrivi(
        "claude_desktop_task_una_tantum_avvio",
        json!({ "cartella_lavoro": cartella_lavoro.display().to_string(), "strumenti_consentiti": strumenti_consentiti }),
    );

    let mut figlio = comando.spawn().map_err(|e| format!("Impossibile avviare Claude Code: {e}"))?;
    let stdin = figlio.stdin.take().ok_or_else(|| "nessun stdin verso Claude Code".to_string())?;
    let stdout = figlio.stdout.take().ok_or_else(|| "nessun output da Claude Code".to_string())?;
    let righe_stdout = TokioBufReader::new(stdout).lines();

    // Stessa ragione di avvia_sessione: lo stderr va svuotato in
    // background, mai lasciato pieno (rischio di bloccare il processo
    // figlio al primo tentativo di scriverci sopra).
    if let Some(stderr) = figlio.stderr.take() {
        tauri::async_runtime::spawn(async move {
            let mut righe_stderr = TokioBufReader::new(stderr).lines();
            while let Ok(Some(riga)) = righe_stderr.next_line().await {
                diagnostics::scrivi("claude_desktop_task_una_tantum_stderr", json!({ "riga": riga }));
            }
        });
    }

    // `sessione` esce di scope alla fine di questa funzione — il suo
    // `Drop` termina il processo (e con esso il bridge MCP), qualunque
    // sia l'esito: nessun riuso, coerente col resto del commento sopra.
    let mut sessione = SessioneAttiva { figlio, stdin, righe_stdout, _bridge: bridge };
    manda_turno(&mut sessione, messaggio).await
}

/// Avvia il processo Claude Code in anticipo, subito all'avvio
/// dell'app — richiesta esplicita dell'utente per non pagare il costo
/// di avvio a freddo (~3s, vedi il commento su `SessioneAttiva`) anche
/// sul PRIMO messaggio di ogni sessione. Non riduce il resto del tempo
/// di risposta (quello è vera latenza di rete/ragionamento del
/// modello, uguale a processo caldo o no) — solo l'avvio del processo.
/// Non fa nulla se il provider configurato non è questo, se Claude
/// Desktop non è installata, o se una sessione risulta già viva
/// (evita un doppio avvio se richiamata più di una volta).
pub async fn prewarm_se_configurato(app_handle: &AppHandle, app_data_dir: PathBuf) {
    let Some(config) = agent::load_config(&app_data_dir) else { return };
    if config.provider != PROVIDER_ID {
        return;
    }
    let Some(claude_exe) = trova_claude_exe() else { return };

    let stato = app_handle.state::<Arc<ClaudeDesktopState>>();
    if stato.sessione.lock().unwrap().is_some() {
        return;
    }

    let server = app_handle.state::<Arc<AppServer>>().inner().clone();
    let hostname = gethostname::gethostname().to_string_lossy().to_string();

    diagnostics::scrivi("claude_desktop_prewarm_iniziato", json!({}));
    match avvia_sessione(server, app_data_dir, hostname, &claude_exe, &config.model).await {
        Ok(sessione) => {
            *stato.sessione.lock().unwrap() = Some(sessione);
            diagnostics::scrivi("claude_desktop_prewarm_completato", json!({}));
        }
        Err(errore) => {
            // Non bloccante: se qualcosa va storto (es. Claude Desktop
            // disconnessa) il primo messaggio vero riproverà da solo
            // (stessa logica "sessione morta → riapri" già in
            // invia_messaggio), l'utente non vede nessun errore ora.
            diagnostics::scrivi("claude_desktop_prewarm_fallito", json!({ "errore": errore }));
        }
    }
}

#[cfg(test)]
mod test_stato {
    //! A differenza di `test_manuale` sotto, questo NON spawna alcun
    //! processo né consuma l'abbonamento — solo controlli sul filesystem
    //! locale (percorso claude.exe, file di credenziali). Non `#[ignore]`
    //! apposta: economico, gira in ogni `cargo test` normale. I valori
    //! esatti dipendono dalla macchina (qui solo stampati per verifica
    //! visiva con --nocapture), l'unica cosa asserita è che la funzione
    //! non va in panico e la forma del risultato è quella attesa.
    use super::*;

    #[test]
    fn claude_desktop_stato_non_va_in_panico() {
        let stato = claude_desktop_stato();
        println!(
            "trovato={} autenticato={} percorso_exe={:?}",
            stato.trovato, stato.autenticato, stato.percorso_exe
        );
        // Coerenza minima: se non è stato trovato nessun claude.exe, il
        // percorso deve essere None (mai un trovato=false con un percorso
        // valorizzato, o viceversa).
        assert_eq!(stato.trovato, stato.percorso_exe.is_some());
    }
}

#[cfg(test)]
mod test_manuale {
    //! Test manuale, NON eseguito da un normale `cargo test` (`#[ignore]`)
    //! — spawna un vero `claude.exe` e consuma davvero l'abbonamento
    //! dell'utente. Usato una tantum per verificare dal vivo il percorso
    //! completo (bridge MCP + processo persistente + strumenti REALI sul
    //! database vero) fuori dall'app grafica, per leggere subito
    //! l'output invece di dover cliccare nella UI. Va lanciato con
    //! `app.exe` GIÀ CHIUSO (stesso database SQLite, evitare accessi
    //! concorrenti) — vedi BLUEPRINT.md.
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_reale_end_to_end() {
        crate::diagnostics::avvia(&std::path::PathBuf::from(
            std::env::var("USERPROFILE").unwrap() + "\\Desktop",
        ))
        .ok();

        let app_data_dir = PathBuf::from(std::env::var("LOCALAPPDATA").unwrap())
            .join("TrackFlow")
            .join("app-data");
        let cartella_neutra = std::env::temp_dir();

        let server = Arc::new(crate::build_app_server(&app_data_dir, &cartella_neutra, &cartella_neutra).await);
        let hostname = gethostname::gethostname().to_string_lossy().to_string();

        let claude_exe = trova_claude_exe().expect("Claude Desktop non trovata");
        println!("claude.exe: {}", claude_exe.display());

        let mut sessione = avvia_sessione(server.clone(), app_data_dir.clone(), hostname.clone(), &claude_exe, "haiku")
            .await
            .expect("avvio sessione fallito");

        let inizio = std::time::Instant::now();
        let risposta = manda_turno(&mut sessione, "Ho lavorato con qualche cliente oggi?").await;
        println!("Turno 1 completato in {:?}: {:?}", inizio.elapsed(), risposta);

        let inizio2 = std::time::Instant::now();
        let risposta2 = manda_turno(&mut sessione, "E ieri, invece?").await;
        println!("Turno 2 completato in {:?}: {:?}", inizio2.elapsed(), risposta2);
    }
}

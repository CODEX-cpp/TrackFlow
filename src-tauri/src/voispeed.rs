//! Integrazione VoiSpeed (TeamSystem VoiSpeed / Cluana) — registro chiamate.
//! Vedi BLUEPRINT.md sezione 6 per l'indagine completa che ha portato a
//! questo design: VoiSpeed desktop è Electron puro attorno a una web app
//! (`https://app.cluana.com`), senza nessuna API/COM di automazione.
//!
//! Nessuna password viene mai salvata da questo modulo. L'utente fa login
//! nella pagina VERA di VoiSpeed dentro una finestra webview di Tauri
//! (motore browser vero, non un client HTTP nostro) — da quel momento la
//! webview mantiene il cookie "ricordami" come farebbe un vero browser, e
//! la si ricarica in background quando serve un token fresco per
//! interrogare l'API delle chiamate.
//!
//! NOTA IMPORTANTE: costruito senza un account VoiSpeed disponibile su
//! questa macchina — l'estrazione del token (regex su `new Controller(...)`,
//! vedi `estrai_identita_da_html`) è la migliore ricostruzione possibile
//! dalla sola riga di esempio trovata durante l'indagine (con "..." per
//! argomenti intermedi sconosciuti). Da verificare e correggere con un
//! login reale sul PC di lavoro prima di considerarlo definitivo — stesso
//! trattamento già riservato al parsing titolo di `aw-watcher-excel-rust`.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::AppServer;

const LOGIN_URL: &str = "https://app.cluana.com/";
const CALLS_API_HOST: &str = "sadr.cluana.com:15644";
const CONNECTION_FILE: &str = "voispeed-connection.json";
const BUCKET_TYPE: &str = "app.voispeed.calls";
const CLIENT_NAME: &str = "trackflow-voispeed";

/// Quanto aspettare, controllando ripetutamente il contenuto della
/// finestra di login, prima di rinunciare (l'utente potrebbe metterci un
/// po' a digitare le credenziali la prima volta).
const LOGIN_POLL_INTERVAL_SECONDS: u64 = 2;
const LOGIN_POLL_TIMEOUT_SECONDS: u64 = 600;

/// Ogni quanto interrogare di nuovo l'elenco chiamate una volta connessi.
/// Alzato da 5 a 30 minuti — richiesta esplicita dell'utente, bug reale:
/// ogni giro riesegue per pochi secondi la pagina VERA di VoiSpeed (vedi
/// il commento su `ottieni_token_fresco`/`estrai_identita_da_html`), che
/// il server tratta come una sessione concorrente a quella dell'app
/// desktop — a 5 minuti capitava troppo spesso perché fosse solo un
/// fastidio occasionale. Il registro chiamate non ha comunque bisogno di
/// essere quasi in tempo reale per uno strumento di time-tracking.
const CALLS_POLL_INTERVAL_SECONDS: u64 = 1800;

/// Identità del contratto VoiSpeed dell'utente — NON un segreto (stesso
/// livello di sensibilità di un nome utente), persistita su disco per
/// sapere che siamo "collegati" tra un riavvio e l'altro. Il token vero
/// (quello sì sensibile e a scadenza breve) non viene mai scritto su
/// disco: si rigenera da zero ogni volta dalla webview.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiSpeedIdentity {
    pub ext: String,
    pub domain: String,
    pub user_id: u64,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct VoiSpeedStatus {
    pub connected: bool,
    pub connecting: bool,
    pub last_error: Option<String>,
    pub last_sync: Option<String>,
}

pub struct VoiSpeedState {
    pub identity: std::sync::Mutex<Option<VoiSpeedIdentity>>,
    pub status: std::sync::Mutex<VoiSpeedStatus>,
    seen_call_ids: std::sync::Mutex<HashSet<String>>,
    /// Spento dal sottomenu Moduli della tray — indipendente dal login:
    /// l'account resta collegato (identity intatta), si ferma solo
    /// l'aggiornamento periodico. Controllato dal ciclo di polling ad
    /// ogni giro.
    polling_enabled: AtomicBool,
    /// Evita di avviare due cicli di polling in parallelo se il modulo
    /// viene riacceso mentre uno precedente non si è ancora fermato del
    /// tutto.
    polling_running: AtomicBool,
    /// Sveglia subito il ciclo di polling mentre dorme tra un giro e
    /// l'altro (fino a 5 minuti) quando arriva uno stop/riavvio dal menu
    /// Moduli, invece di aspettare che il sleep scada da solo.
    polling_wake: tokio::sync::Notify,
    /// true da quando l'utente chiude la finestra di login senza
    /// completarlo — fa uscire subito ottieni_token_fresco() invece di
    /// aspettare il timeout di 10 minuti.
    login_window_closed: AtomicBool,
}

impl VoiSpeedState {
    pub fn new() -> Self {
        Self {
            identity: std::sync::Mutex::new(None),
            status: std::sync::Mutex::new(VoiSpeedStatus::default()),
            seen_call_ids: std::sync::Mutex::new(HashSet::new()),
            polling_enabled: AtomicBool::new(true),
            polling_running: AtomicBool::new(false),
            polling_wake: tokio::sync::Notify::new(),
            login_window_closed: AtomicBool::new(false),
        }
    }
}

fn connection_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(CONNECTION_FILE)
}

pub fn load_identity(app_data_dir: &Path) -> Option<VoiSpeedIdentity> {
    let content = std::fs::read_to_string(connection_path(app_data_dir)).ok()?;
    serde_json::from_str(&content).ok()
}

fn save_identity(app_data_dir: &Path, identity: &VoiSpeedIdentity) {
    if let Ok(json) = serde_json::to_string_pretty(identity) {
        let _ = std::fs::write(connection_path(app_data_dir), json);
    }
}

fn clear_identity(app_data_dir: &Path) {
    let _ = std::fs::remove_file(connection_path(app_data_dir));
}

/// Estrae ext/token/domain/user_id dalla riga `new Controller(...)` che
/// VoiSpeed incorpora nella pagina dopo un login riuscito (vedi
/// BLUEPRINT.md sezione 6 per la riga di esempio trovata). Posizioni
/// note: 1° argomento = ext, 2° = token, 4° = domain, ultimo argomento
/// (numerico, senza virgolette) = user_id — gli argomenti intermedi
/// (3°, e qualunque altro prima dell'ultimo) sono ignorati apposta,
/// perché il loro numero/contenuto esatto non è stato verificato su
/// dati reali.
fn estrai_identita_da_html(html: &str) -> Option<(String, VoiSpeedIdentity)> {
    let pattern = Regex::new(
        r#"new Controller\(\s*"([^"]*)"\s*,\s*"([^"]*)"\s*,\s*"[^"]*"\s*,\s*"([^"]*)"\s*,[\s\S]*?,\s*(\d+)\s*\)"#,
    )
    .ok()?;
    let cap = pattern.captures(html)?;
    let ext = cap.get(1)?.as_str().to_string();
    let token = cap.get(2)?.as_str().to_string();
    let domain = cap.get(3)?.as_str().to_string();
    let user_id: u64 = cap.get(4)?.as_str().parse().ok()?;
    if ext.is_empty() || token.is_empty() || domain.is_empty() {
        return None;
    }
    Some((token, VoiSpeedIdentity { ext, domain, user_id }))
}

/// JS iniettata ripetutamente nella finestra di login: cerca la riga
/// `new Controller(...)` nell'HTML della pagina corrente ed espone il
/// risultato tramite `document.title` — un modo semplice per far
/// arrivare un valore dalla webview a Rust senza impostare un vero
/// canale IPC dedicato in questa finestra separata (che carica un sito
/// esterno, non la nostra webui).
const EXTRACT_MARKER: &str = "VOISPEED_HTML_READY::";
const EXTRACT_JS: &str = r#"
(function() {
    try {
        var html = document.documentElement.outerHTML;
        if (html.indexOf('Controller(') !== -1) {
            document.title = "VOISPEED_HTML_READY::" + encodeURIComponent(html);
        }
    } catch (e) {}
})();
"#;

/// Apre (o riusa se già esiste) la finestra di login VoiSpeed, ci inietta
/// ripetutamente `EXTRACT_JS` finché non trova la riga Controller(...)
/// nel contenuto della pagina, o scade il timeout. `visible` controlla
/// se mostrarla (primo collegamento, l'utente deve poter digitare le
/// credenziali) o tenerla nascosta (rinnovo automatico del token,
/// affidandosi al cookie "ricordami" già presente da un login precedente).
async fn ottieni_token_fresco(app_handle: &AppHandle, visible: bool) -> Option<(String, VoiSpeedIdentity)> {
    // Azzerata prima di (ri)aprire la finestra: se un tentativo
    // precedente era stato chiuso dall'utente, questo nuovo tentativo
    // riparte pulito invece di uscire subito credendo che sia già stata
    // chiusa anche questa volta.
    app_handle
        .state::<Arc<VoiSpeedState>>()
        .login_window_closed
        .store(false, Ordering::SeqCst);

    let app_handle_for_window = app_handle.clone();
    let window_result = app_handle.run_on_main_thread(move || {
        if let Some(existing) = app_handle_for_window.get_webview_window("voispeed_auth") {
            let _ = existing.navigate(LOGIN_URL.parse().expect("URL di login VoiSpeed non valido"));
            if visible {
                let _ = existing.show();
                let _ = existing.set_focus();
            }
        } else {
            let builder = WebviewWindowBuilder::new(
                &app_handle_for_window,
                "voispeed_auth",
                WebviewUrl::External(LOGIN_URL.parse().expect("URL di login VoiSpeed non valido")),
            )
            .title("Collega VoiSpeed")
            .inner_size(480.0, 640.0)
            .visible(visible);
            match builder.build() {
                Ok(window) => {
                    // Rilevato subito (invece di aspettare fino a 10
                    // minuti di timeout) quando l'utente chiude la
                    // finestra senza completare il login — richiesta
                    // esplicita: il pulsante "Collega VoiSpeed" deve
                    // tornare cliccabile appena la finestra si chiude,
                    // non dopo un lungo timeout silenzioso.
                    let state_for_close = app_handle_for_window.state::<Arc<VoiSpeedState>>().inner().clone();
                    window.on_window_event(move |event| {
                        if let tauri::WindowEvent::CloseRequested { .. } = event {
                            state_for_close.login_window_closed.store(true, Ordering::SeqCst);
                        }
                    });
                }
                Err(e) => log::error!("Impossibile creare la finestra di login VoiSpeed: {e}"),
            }
        }
    });
    if window_result.is_err() {
        return None;
    }

    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(LOGIN_POLL_TIMEOUT_SECONDS);
    while start.elapsed() < timeout {
        tokio::time::sleep(std::time::Duration::from_secs(LOGIN_POLL_INTERVAL_SECONDS)).await;

        if app_handle.state::<Arc<VoiSpeedState>>().login_window_closed.load(Ordering::SeqCst) {
            return None;
        }

        let app_handle_for_poll = app_handle.clone();
        let (tx, rx) = tokio::sync::oneshot::channel::<Option<String>>();
        let sent = app_handle.run_on_main_thread(move || {
            let title = if let Some(window) = app_handle_for_poll.get_webview_window("voispeed_auth") {
                let _ = window.eval(EXTRACT_JS);
                // eval() è fire-and-forget: diamo alla pagina un istante per
                // eseguire lo script prima di rileggere il titolo.
                window.title().ok()
            } else {
                None
            };
            let _ = tx.send(title);
        });
        if sent.is_err() {
            continue;
        }
        let Ok(title) = rx.await else { continue };
        let Some(title) = title else { continue };
        let Some(encoded) = title.strip_prefix(EXTRACT_MARKER) else {
            continue;
        };
        let Ok(html) = urlencoding_decode(encoded) else {
            continue;
        };
        if let Some((token, identity)) = estrai_identita_da_html(&html) {
            // CHIUDE la finestra (non solo `.hide()`) appena estratto il
            // token — bug reale segnalato dall'utente: "quando faccio il
            // login mi disconnette dall'app VoiSpeed desktop". VoiSpeed
            // applica lato server una regola "una sola sessione live per
            // account": nascondere la finestra lascia comunque la pagina
            // vera in esecuzione in background (stessa connessione SSE
            // aperta di un browser reale), quindi per TUTTO il tempo in
            // cui TrackFlow resta collegato quella sessione nascosta
            // competeva continuamente con la sessione dell'app desktop,
            // non solo per un istante durante il rinnovo. Chiudendo la
            // finestra invece di nasconderla, la pagina reale (e la sua
            // connessione SSE) esiste solo per i pochi secondi necessari
            // a leggere il token ad ogni giro di rinnovo — molto meno
            // occasioni di conflitto, non zero (l'unica soluzione a zero
            // conflitti richiederebbe di non eseguire mai la pagina vera,
            // che è esattamente il meccanismo di bypass scartato per
            // motivi di autorizzazione, vedi BLUEPRINT.md).
            if !visible {
                if let Some(window) = app_handle.get_webview_window("voispeed_auth") {
                    let _ = window.close();
                }
            }
            return Some((token, identity));
        }
    }
    None
}

/// Piccolo decoder percent-encoding senza aggiungere una dipendenza
/// nuova solo per questo — l'HTML della pagina può contenere caratteri
/// non ASCII che `document.title` (una stringa DOM normale) gestirebbe
/// comunque, ma li codifichiamo lato JS per evitare problemi con
/// eventuali newline/caratteri di controllo nel titolo della finestra.
fn urlencoding_decode(input: &str) -> Result<String, ()> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).map_err(|_| ())?;
            let byte = u8::from_str_radix(hex, 16).map_err(|_| ())?;
            out.push(byte);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| ())
}

fn oggi_range_locale() -> (String, String) {
    let oggi = chrono::Local::now().date_naive();
    (
        format!("{} 00:00:00", oggi.format("%Y-%m-%d")),
        format!("{} 23:59:59", oggi.format("%Y-%m-%d")),
    )
}

/// Interroga l'endpoint chiamate di VoiSpeed per la giornata corrente.
/// Vedi BLUEPRINT.md sezione 6 per come è stato trovato questo endpoint
/// (non documentato pubblicamente).
fn interroga_chiamate(token: &str, identity: &VoiSpeedIdentity) -> Result<Value, String> {
    let (date_from, date_to) = oggi_range_locale();
    let url = format!(
        "https://{host}/restapi/v1/entities/users/{user_id}/calls\
         ?date_from={date_from}&date_to={date_to}\
         &offset=0&limit=100&audio_calls=-1&video_calls=-1&recorded=-1&group_calls=-1\
         &token={token}&ext={ext}&domain={domain}",
        host = CALLS_API_HOST,
        user_id = identity.user_id,
        date_from = urlencode_query(&date_from),
        date_to = urlencode_query(&date_to),
        token = urlencode_query(token),
        ext = urlencode_query(&identity.ext),
        domain = urlencode_query(&identity.domain),
    );
    ureq::get(&url)
        .call()
        .map_err(|e| format!("richiesta fallita: {e}"))?
        .into_json::<Value>()
        .map_err(|e| format!("risposta non valida: {e}"))
}

fn urlencode_query(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for b in input.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Converte una chiamata VoiSpeed grezza in un evento ActivityWatch.
/// `outcome`: 1 = risposta, 5 = persa/non risposta (riconoscibile anche
/// da `answer_time` col placeholder "1899-12-30", vedi indagine).
fn chiamata_a_evento(chiamata: &Value) -> Option<Value> {
    let start_time = chiamata.get("start_time")?.as_str()?;
    let duration = chiamata.get("duration").and_then(Value::as_f64).unwrap_or(0.0);
    let direction = chiamata.get("direction").and_then(Value::as_i64).unwrap_or(0);
    let outcome = chiamata.get("outcome").and_then(Value::as_i64).unwrap_or(0);
    let remote_number = chiamata.get("remote_number").and_then(Value::as_str).unwrap_or("");
    let identity_nome = chiamata.get("identity").and_then(Value::as_str).unwrap_or("");

    let timestamp = chrono::DateTime::parse_from_str(
        &format!("{start_time} +0000"),
        "%Y-%m-%d %H:%M:%S %z",
    )
    .ok()?
    .to_rfc3339();

    let nome_contatto = if identity_nome.is_empty() { remote_number } else { identity_nome };
    let data = json!({
        "cliente": nome_contatto,
        "numero": remote_number,
        "direzione": if direction == 1 { "uscente" } else { "entrante" },
        "risposta": outcome == 1,
    });

    Some(json!({ "timestamp": timestamp, "duration": duration, "data": data }))
}

/// Un singolo giro: rinnova il token (riusando la finestra nascosta, che
/// fa da solo il login col cookie "ricordami" già presente — questo È la
/// verifica che siamo ancora davvero loggati, non c'è un endpoint più
/// leggero apposta solo per "controllare" senza rinnovare) e interroga
/// le chiamate di oggi, inserendo solo quelle mai viste prima. Riusato
/// sia dal ciclo periodico automatico sia dal pulsante "Aggiorna" manuale
/// nelle Impostazioni (`voispeed_refresh_check`) — stessa identica
/// logica, unica differenza è chi la richiama e quando.
///
/// Se il rinnovo del token fallisce, la sessione VoiSpeed è scaduta (o
/// l'account è stato scollegato lato TeamSystem): `status.connected`
/// torna `false` così il pulsante "Collega VoiSpeed" ricompare da solo
/// nelle Impostazioni (il template lo mostra con `v-if="!status.connected"`)
/// — richiesta esplicita dell'utente, 2026-08-12. L'identità salvata
/// (ext/domain/user_id) NON viene cancellata: se il problema era
/// transitorio, il prossimo giro può riconnettersi da solo senza un
/// nuovo login manuale.
async fn esegui_un_giro(app_handle: &AppHandle, server: &Arc<AppServer>, hostname: &str) {
    let esito = match ottieni_token_fresco(app_handle, false).await {
        Some((token, identity_fresca)) => {
            interroga_chiamate(&token, &identity_fresca).map(|risposta| (identity_fresca, risposta))
        }
        None => {
            Err("Sessione VoiSpeed scaduta — rifai il login.".to_string())
        }
    };

    let state = app_handle.state::<Arc<VoiSpeedState>>();
    match esito {
        Ok((identity_fresca, risposta)) => {
            save_identity_from_state(app_handle, &identity_fresca);

            // Niente più suffisso "_<hostname>" — vedi il commento in
            // aw-watcher-afk-rust/src/main.rs.
            let bucket_id = "voispeed-calls".to_string();
            server.ensure_bucket(&bucket_id, BUCKET_TYPE, CLIENT_NAME, hostname).await;

            if let Some(chiamate) = risposta.get("calls").and_then(Value::as_array) {
                // Prima si decide COSA è nuovo (lock breve, mai tenuto
                // durante un .await — un MutexGuard std non è Send tra
                // punti di sospensione asincroni), poi si inseriscono
                // gli eventi nuovi, poi si segna il tutto come "visto"
                // con un secondo lock breve.
                let da_inserire: Vec<(String, Value)> = {
                    let seen = state.seen_call_ids.lock().unwrap();
                    chiamate
                        .iter()
                        .filter_map(|chiamata| {
                            let id = chiamata.get("id").and_then(|v| {
                                v.as_str().map(str::to_string).or_else(|| v.as_i64().map(|n| n.to_string()))
                            })?;
                            if seen.contains(&id) {
                                return None;
                            }
                            let evento = chiamata_a_evento(chiamata)?;
                            Some((id, evento))
                        })
                        .collect()
                };

                let mut nuovi_id = Vec::with_capacity(da_inserire.len());
                for (id, evento) in da_inserire {
                    server.insert_event(&bucket_id, &evento).await;
                    nuovi_id.push(id);
                }

                let mut seen = state.seen_call_ids.lock().unwrap();
                seen.extend(nuovi_id);
            }

            let mut status = state.status.lock().unwrap();
            status.connected = true;
            status.connecting = false;
            status.last_error = None;
            status.last_sync = Some(chrono::Utc::now().to_rfc3339());
        }
        Err(errore) => {
            log::warn!("VoiSpeed: giro di aggiornamento fallito: {errore}");
            // Notifica solo sulla TRANSIZIONE connesso→disconnesso, non ad
            // ogni giro fallito mentre resta giù (il polling ci riprova
            // ogni 5 minuti — richiesta esplicita: "deve arrivare solo nel
            // momento in cui prima ero effettivamente loggato", altrimenti
            // un server VoiSpeed giù per ore manderebbe un toast ogni
            // giro). era_connesso letto PRIMA di sovrascrivere connected,
            // così cattura lo stato di un attimo fa, non quello appena
            // scritto qui sotto.
            let era_connesso = {
                let mut status = state.status.lock().unwrap();
                let era_connesso = status.connected;
                status.connected = false;
                status.last_error = Some(errore);
                era_connesso
            };
            if era_connesso {
                crate::notifications::invia_toast(
                    app_handle,
                    "VoiSpeed disconnesso",
                    "La sessione si è interrotta (server VoiSpeed non raggiungibile o account scollegato). Apri Impostazioni → Integrazioni per ricollegarti.",
                );
            }
        }
    }
}

/// Ciclo periodico automatico — un `esegui_un_giro` ogni
/// `CALLS_POLL_INTERVAL_SECONDS` (30 minuti, vedi il commento sulla
/// costante per il perché non più breve): non serve un timer separato,
/// il rinnovo del token che questo ciclo fa comunque ad ogni giro (per
/// interrogare le chiamate) È la stessa identica verifica,
/// semplicemente già eseguita più spesso del minimo richiesto.
async fn ciclo_di_polling(app_handle: AppHandle, server: Arc<AppServer>, hostname: String) {
    loop {
        let (identity, abilitato) = {
            let state = app_handle.state::<Arc<VoiSpeedState>>();
            let guard = state.identity.lock().unwrap();
            (guard.clone(), state.polling_enabled.load(Ordering::SeqCst))
        };
        if identity.is_none() {
            // Disconnesso nel frattempo (l'utente ha premuto "Scollega") —
            // il ciclo si ferma da solo, va riavviato da voispeed_connect.
            return;
        }
        if !abilitato {
            // Spento dal menu Moduli della tray — l'account resta
            // collegato (identity intatta), il ciclo si ferma finché non
            // viene riacceso (vedi avvia_polling, richiamato dal toggle).
            return;
        }

        esegui_un_giro(&app_handle, &server, &hostname).await;

        // select! invece di un sleep semplice: se ferma_polling() arriva
        // mentre il ciclo sta dormendo, si sveglia subito (polling_wake)
        // invece di aspettare fino a 5 minuti che il sleep scada da solo
        // — il toggle nel menu Moduli deve avere effetto quasi immediato.
        let state = app_handle.state::<Arc<VoiSpeedState>>();
        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_secs(CALLS_POLL_INTERVAL_SECONDS)) => {}
            _ = state.polling_wake.notified() => {}
        }
    }
}

fn save_identity_from_state(app_handle: &AppHandle, identity: &VoiSpeedIdentity) {
    if let Some(dir) = app_handle.try_state::<crate::AppDataDirState>() {
        save_identity(&dir.0, identity);
    }
    let state = app_handle.state::<Arc<VoiSpeedState>>();
    *state.identity.lock().unwrap() = Some(identity.clone());
}

/// Avvia il ciclo di polling periodico — chiamata all'avvio dell'app (se
/// risultava già collegata), subito dopo un primo collegamento riuscito,
/// e quando il modulo VoiSpeed viene riacceso dal menu Moduli della
/// tray. Sicura da richiamare anche se un ciclo è già in corso (non ne
/// avvia un secondo in parallelo) — sempre riabilita il polling
/// (`polling_enabled`), utile quando si riparte dopo uno stop dal menu.
pub fn avvia_polling(app_handle: AppHandle, server: Arc<AppServer>, hostname: String) {
    let state = app_handle.state::<Arc<VoiSpeedState>>();
    state.polling_enabled.store(true, Ordering::SeqCst);
    if state.polling_running.swap(true, Ordering::SeqCst) {
        // Già in corso — sveglialo nel caso stesse dormendo dopo un
        // vecchio "spegni" appena annullato da questo stesso riavvio.
        state.polling_wake.notify_one();
        return;
    }
    tauri::async_runtime::spawn(async move {
        ciclo_di_polling(app_handle.clone(), server, hostname).await;
        app_handle.state::<Arc<VoiSpeedState>>().polling_running.store(false, Ordering::SeqCst);
    });
}

/// Ferma il ciclo di polling (menu Moduli della tray) senza scollegare
/// l'account — l'identità resta salvata, riaccendere il modulo riprende
/// da dove aveva lasciato senza bisogno di un nuovo login.
pub fn ferma_polling(app_handle: &AppHandle) {
    let state = app_handle.state::<Arc<VoiSpeedState>>();
    state.polling_enabled.store(false, Ordering::SeqCst);
    state.polling_wake.notify_one();
}

/// Usato dal pannello "Stato watcher" (Impostazioni → Sviluppatore) per
/// sapere se il ciclo di polling VoiSpeed è davvero in corso in questo
/// momento — non essendo un sidecar separato (gira in-process), non
/// compare in `SidecarProcesses` come gli altri.
pub fn is_polling_running(app_handle: &AppHandle) -> bool {
    app_handle
        .try_state::<Arc<VoiSpeedState>>()
        .map(|s| s.polling_running.load(Ordering::SeqCst))
        .unwrap_or(false)
}

/// Comando Tauri: pulsante "Collega VoiSpeed" nelle Impostazioni. Apre la
/// finestra di login VISIBILE, aspetta che l'utente completi il login
/// nella pagina vera, poi avvia il ciclo di polling.
#[tauri::command]
pub async fn voispeed_connect(app_handle: AppHandle) -> Result<(), String> {
    {
        let state = app_handle.state::<Arc<VoiSpeedState>>();
        let mut status = state.status.lock().unwrap();
        status.connecting = true;
        status.last_error = None;
    }

    match ottieni_token_fresco(&app_handle, true).await {
        Some((_token, identity)) => {
            save_identity_from_state(&app_handle, &identity);
            {
                let state = app_handle.state::<Arc<VoiSpeedState>>();
                let mut status = state.status.lock().unwrap();
                status.connecting = false;
                status.connected = true;
            }
            // Stesso motivo del caso "non visibile" in ottieni_token_fresco:
            // chiudere invece di nascondere evita di lasciare una sessione
            // VoiSpeed reale (e la sua connessione SSE) viva in background
            // per tutta la durata del collegamento con TrackFlow.
            if let Some(window) = app_handle.get_webview_window("voispeed_auth") {
                let _ = window.close();
            }

            let server = app_handle.state::<Arc<AppServer>>().inner().clone();
            let hostname = gethostname::gethostname().to_string_lossy().to_string();
            avvia_polling(app_handle.clone(), server, hostname);
            Ok(())
        }
        None => {
            let state = app_handle.state::<Arc<VoiSpeedState>>();
            // Distingue "l'utente ha chiuso la finestra" (nessun errore
            // vero, torna solo cliccabile il pulsante) da un timeout
            // reale/pagina non riconosciuta (quello sì un errore da
            // mostrare) — richiesta esplicita dell'utente.
            let chiusa_dall_utente = state.login_window_closed.load(Ordering::SeqCst);
            let mut status = state.status.lock().unwrap();
            status.connecting = false;
            if chiusa_dall_utente {
                status.last_error = None;
                Ok(())
            } else {
                status.last_error = Some(
                    "Login non completato entro 10 minuti, o pagina VoiSpeed non riconosciuta — riprova."
                        .to_string(),
                );
                Err(status.last_error.clone().unwrap_or_default())
            }
        }
    }
}

/// Comando Tauri: pulsante "Aggiorna" nelle Impostazioni — forza subito
/// un giro (rinnovo token + verifica login + interrogazione chiamate)
/// invece di aspettare il prossimo giro automatico. Richiesta esplicita
/// dell'utente, 2026-08-12. Riusa `esegui_un_giro`, stessa logica del
/// ciclo periodico — nessun accesso diretto al datastore o ai comandi da
/// duplicare qui.
#[tauri::command]
pub async fn voispeed_refresh_check(app_handle: AppHandle) -> Result<(), String> {
    let identity = {
        let state = app_handle.state::<Arc<VoiSpeedState>>();
        let guard = state.identity.lock().unwrap();
        guard.clone()
    };
    if identity.is_none() {
        return Err("Nessun account VoiSpeed collegato.".to_string());
    }

    let server = app_handle.state::<Arc<AppServer>>().inner().clone();
    let hostname = gethostname::gethostname().to_string_lossy().to_string();
    esegui_un_giro(&app_handle, &server, &hostname).await;

    let state = app_handle.state::<Arc<VoiSpeedState>>();
    let status = state.status.lock().unwrap();
    match &status.last_error {
        Some(errore) => Err(errore.clone()),
        None => Ok(()),
    }
}

#[tauri::command]
pub fn voispeed_status(app_handle: AppHandle) -> VoiSpeedStatus {
    let state = app_handle.state::<Arc<VoiSpeedState>>();
    let guard = state.status.lock().unwrap();
    guard.clone()
}

#[tauri::command]
pub fn voispeed_disconnect(app_handle: AppHandle) -> Result<(), String> {
    if let Some(dir) = app_handle.try_state::<crate::AppDataDirState>() {
        clear_identity(&dir.0);
    }
    let state = app_handle.state::<Arc<VoiSpeedState>>();
    *state.identity.lock().unwrap() = None;
    let mut status = state.status.lock().unwrap();
    *status = VoiSpeedStatus::default();
    drop(status);

    if let Some(window) = app_handle.get_webview_window("voispeed_auth") {
        let _ = window.close();
    }
    Ok(())
}

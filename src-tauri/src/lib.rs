use std::collections::HashMap;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tauri::menu::{CheckMenuItem, IsMenuItem, Menu, MenuItem, Submenu};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::{http, AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;
use tokio::sync::Mutex as AsyncMutex;

mod about;
mod agent;
mod autostart;
mod categorization;
mod custom_modules;
mod custom_watchers;
mod devtools;
mod dpapi;
mod folder_shortcuts;
mod notifications;
mod voispeed;
mod vpn_mapping;
mod vpn_notify;
mod updater;
mod watcher_status;
mod window_state;
use voispeed::VoiSpeedState;

/// Handle condiviso per il processo `aw-watcher-app-icons`: `None`
/// quando il modulo è spento, `Some` quando gira. Condiviso (stesso
/// `Arc`) sia da chi lo avvia/ferma (spawn_watcher/stop_watcher, il
/// menu Moduli della tray) sia dal drenaggio dello stdout del watcher
/// finestra, che ci scrive dentro i nomi delle app scoperte — vedi
/// spawn_stdout_drain.
type IconsHandle = Arc<std::sync::Mutex<Option<CommandChild>>>;

/// Crea un Job Object di Windows con KILL_ON_JOB_CLOSE e ci assegna il
/// processo corrente. Tutti i processi figli avviati DA QUESTO MOMENTO
/// in poi (i sidecar watcher) entrano automaticamente nello stesso job
/// (comportamento di default di CreateProcess su Windows). Se il processo
/// di questa app termina per QUALSIASI motivo — crash, chiuso da Task
/// Manager, kill esterno, non solo un'uscita pulita dal menu della tray —
/// Windows chiude l'handle del job come parte della pulizia del processo,
/// il che (grazie a KILL_ON_JOB_CLOSE) termina automaticamente anche tutti
/// i sidecar. Senza questo, un kill diretto del processo principale lascia
/// i watcher orfani in esecuzione all'infinito — verificato empiricamente
/// prima di aggiungere questo fix (Fase 4), e riverificato dopo il
/// passaggio al server in-process (Fase 5).
#[cfg(windows)]
fn setup_kill_on_exit_job() {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, SetInformationJobObject,
        JobObjectExtendedLimitInformation, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_BREAKAWAY_OK, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    use windows::Win32::System::Threading::GetCurrentProcess;

    unsafe {
        let Ok(job) = CreateJobObjectW(None, None) else {
            log::error!("Impossibile creare il Job Object per la gestione dei sidecar");
            return;
        };

        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        // BREAKAWAY_OK oltre a KILL_ON_JOB_CLOSE — senza, QUALUNQUE
        // processo figlio (compresi i sidecar watcher, comportamento
        // voluto) entra nel job e viene ucciso col padre. Serve
        // un'eccezione esplicita per launcher.exe quando lo lanciamo per
        // il riavvio dopo un aggiornamento (vedi updater.rs,
        // CREATE_BREAKAWAY_FROM_JOB): senza questo flag sul job, quello
        // spawn fallirebbe, e CON questo flag ma senza chiederlo
        // esplicitamente al momento dello spawn i sidecar continuerebbero
        // comunque a restare nel job come prima — bug reale trovato
        // dall'utente: il riavvio automatico non appariva mai, perché
        // launcher.exe veniva ucciso insieme al processo che lo aveva
        // appena lanciato, un istante prima di poter avviare la versione
        // nuova.
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_BREAKAWAY_OK;

        let set_ok = SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &info as *const _ as *const std::ffi::c_void,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        );
        if set_ok.is_err() {
            log::error!("Impossibile configurare KILL_ON_JOB_CLOSE sul Job Object");
            let _ = CloseHandle(job);
            return;
        }

        let current: HANDLE = GetCurrentProcess();
        if AssignProcessToJobObject(job, current).is_err() {
            log::error!("Impossibile assegnare il processo corrente al Job Object");
            let _ = CloseHandle(job);
            return;
        }

        // Deliberato: NON chiudiamo l'handle — deve restare aperto per
        // tutta la vita del processo, altrimenti KILL_ON_JOB_CLOSE
        // scatterebbe subito. `HANDLE` è Copy e non implementa Drop,
        // quindi lasciarlo semplicemente uscire di scope qui è
        // sufficiente: Windows non lo chiude finché il processo non
        // termina.
    }
}

#[cfg(not(windows))]
fn setup_kill_on_exit_job() {}

/// Elenco dei watcher "generici" (nome sidecar, senza suffisso
/// target-triple — Tauri lo risolve da solo) — quelli gestiti dalla
/// mappa in `SidecarProcesses`. "aw-watcher-app-icons" NON è in questa
/// lista: ha un ciclo di vita a parte (IconsHandle) perché il suo stdin
/// deve restare raggiungibile per inoltrargli i nomi delle app scoperte
/// dal watcher finestra (vedi spawn_stdout_drain, BLUEPRINT.md Fase 5).
const WATCHERS: &[&str] = &[
    "aw-watcher-vpn",
    "aw-watcher-claude-code",
    "aw-watcher-screenshot",
    "aw-watcher-afk",
    "aw-watcher-window",
    "aw-watcher-tray",
    "aw-watcher-vscode",
    "aw-watcher-excel",
];

/// Tutti e 9 i moduli, incluso app-icons, con l'etichetta mostrata nel
/// sottomenu "Moduli" della tray — un modulo alla volta, ordine fisso.
/// Un modulo assente da modules-config.json (es. appena aggiunto qui)
/// parte sempre attivo di default — vedi load_modules_config() sotto.
const ALL_MODULES: &[(&str, &str)] = &[
    ("aw-watcher-afk", "Rilevamento inattività"),
    ("aw-watcher-window", "Finestra attiva"),
    ("aw-watcher-vpn", "Sessioni VPN"),
    ("aw-watcher-claude-code", "Sessioni Claude Code"),
    ("aw-watcher-screenshot", "Screenshot"),
    ("aw-watcher-app-icons", "Icone app"),
    ("aw-watcher-tray", "App in background (system tray)"),
    ("aw-watcher-vscode", "Attività VS Code"),
    ("aw-watcher-excel", "Attività Excel"),
    // Non è un sidecar (gira in-process nel binario principale, vedi
    // voispeed.rs) — assente da WATCHERS sopra apposta, ma ha comunque
    // una voce qui per poter essere acceso/spento dal menu Moduli come
    // gli altri. Spegnerlo ferma solo il polling periodico, non
    // scollega l'account (l'identità resta salvata).
    ("aw-watcher-voispeed", "VoiSpeed"),
    // Anche questo virtuale, come VoiSpeed sopra — nessun processo da
    // avviare/fermare (gira agganciato agli eventi del watcher finestra,
    // vedi categorization.rs), ma spegnibile dal menu Moduli per chi non
    // vuole che le app scoperte vengano mandate a Claude per la
    // categorizzazione automatica, pur tenendo attiva la chiave AI per
    // la chat. Il controllo vero e proprio è letto dal vivo ad ogni
    // evento (vedi spawn_stdout_drain), non da uno start/stop qui.
    ("ai-categorization", "Categorizzazione app (AI)"),
];

/// Nome del file, dentro la cartella dati scrivibile, dove si ricorda
/// quali moduli l'utente ha acceso/spento dal menu della tray — riletto
/// all'avvio così la scelta resta valida anche dopo un riavvio (richiesta
/// esplicita: "il programma quando parte non deve più attivare i moduli
/// senza la spunta").
const MODULES_CONFIG_FILE: &str = "modules-config.json";

fn load_modules_config(app_data_dir: &Path) -> HashMap<String, bool> {
    let path = app_data_dir.join(MODULES_CONFIG_FILE);
    let mut map: HashMap<String, bool> = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    // Qualunque modulo assente dal file (prima esecuzione, o un modulo
    // nuovo aggiunto dopo che il file esisteva già) parte attivo di
    // default — lo stesso comportamento di sempre finché l'utente non
    // lo spegne esplicitamente.
    //
    // Eccezioni, spenti di default:
    // - aw-watcher-tray: le icone nascoste dietro "Mostra icone nascoste"
    //   non sono ancora lette in modo affidabile (vedi BLUEPRINT.md
    //   sezione 7).
    // - aw-watcher-vpn, aw-watcher-claude-code, aw-watcher-excel,
    //   aw-watcher-voispeed: richiesta esplicita — moduli pensati per chi
    //   usa VPN aziendali/Claude Code/Excel/VoiSpeed in un certo modo,
    //   non rilevanti per la maggior parte di chi installa l'app per la
    //   prima volta.
    // Tutti restano comunque riattivabili a mano dal sottomenu "Moduli"
    // della tray.
    for (name, _) in ALL_MODULES {
        let default_attivo = !matches!(
            *name,
            "aw-watcher-tray"
                | "aw-watcher-vpn"
                | "aw-watcher-claude-code"
                | "aw-watcher-excel"
                | "aw-watcher-voispeed"
        );
        map.entry((*name).to_string()).or_insert(default_attivo);
    }
    map
}

fn save_modules_config(app_data_dir: &Path, config: &HashMap<String, bool>) {
    if let Ok(json) = serde_json::to_string_pretty(config) {
        let _ = std::fs::write(app_data_dir.join(MODULES_CONFIG_FILE), json);
    }
}

/// Tiene vivi i handle dei processi figlio "generici" (tutti tranne
/// app-icons, vedi IconsHandle), indicizzati per nome così il menu
/// Moduli della tray possa fermarne uno specifico senza toccare gli
/// altri. Se droppati, `CommandChild` non uccide automaticamente il
/// processo, ma teniamo comunque i riferimenti per poterli terminare
/// esplicitamente al bisogno (Esci, o spegnimento di un singolo modulo).
struct SidecarProcesses(std::sync::Mutex<HashMap<String, CommandChild>>);

/// Cartella dati scrivibile (icone, screenshot, config moduli, override
/// impostazioni) — calcolata una volta in setup() e tenuta come stato
/// gestito così spawn/stop dei moduli innescati dal menu della tray non
/// devono ricalcolarla.
struct AppDataDirState(PathBuf);

/// Scelta attiva/spento per ciascun modulo, tenuta in memoria (oltre che
/// su file) per rispondere rapidamente ai click sul menu Moduli senza
/// rileggere il file ad ogni volta.
struct ModulesConfigState(std::sync::Mutex<HashMap<String, bool>>);

/// Handle ai `CheckMenuItem` del sottomenu Moduli, per aggiornarne la
/// spunta a video quando lo stato cambia (avvio/arresto di un modulo).
struct ModuleMenuItems(std::sync::Mutex<HashMap<String, CheckMenuItem<tauri::Wry>>>);

/// Il server `aw-server-rust` incorporato nel processo di questa app
/// (Fase 5 — vedi BLUEPRINT.md): non apre più nessuna porta TCP, ogni
/// richiesta (sia della webui che dei watcher) viene dispatchata in
/// memoria tramite `rocket::local::asynchronous::Client`, che fa girare
/// l'intero Rocket dell'app senza mai passare dalla rete. Per questo
/// motivo un browser esterno non può più raggiungere il server in nessun
/// modo — non esiste letteralmente nulla in ascolto.
struct AppServer {
    client: rocket::local::asynchronous::Client,
    /// Bucket già creati in questa sessione (cache per evitare una POST
    /// ridondante di creazione ad ogni singolo evento — i bucket restano
    /// comunque idempotenti anche senza questa cache, la INSERT fallisce
    /// silenziosamente se il bucket esiste già).
    known_buckets: AsyncMutex<HashSet<String>>,
    /// Stessa cartella scrivibile passata a --custom-static app-data —
    /// usata anche per scrivere il file di override dell'intervallo
    /// screenshot quando cambia dalle impostazioni (vedi dispatch()).
    app_data_dir: PathBuf,
    /// Registro nome->cartella dei moduli personalizzati, condiviso con
    /// la route Rocket `/pages/custom/<nome>/` — vedi custom_modules.rs.
    pub(crate) custom_pages_registry: Arc<aw_server::endpoints::CustomPagesRegistry>,
}

impl AppServer {
    /// Header 'Host' che soddisfa il fairing HostCheck di aw-server-rust
    /// (protezione anti DNS-rebinding — non più necessaria dato che non
    /// c'è più nessuna rete, ma il codice del server non è stato toccato:
    /// più semplice soddisfare il controllo esistente che disattivarlo).
    fn host_header() -> rocket::http::Header<'static> {
        rocket::http::Header::new("Host", "127.0.0.1")
    }

    async fn ensure_bucket(&self, bucket_id: &str, bucket_type: &str, client_name: &str, hostname: &str) {
        {
            let known = self.known_buckets.lock().await;
            if known.contains(bucket_id) {
                return;
            }
        }
        let body = serde_json::json!({
            "id": bucket_id,
            "type": bucket_type,
            "client": client_name,
            "hostname": hostname,
        })
        .to_string();
        let _ = self
            .client
            .post(format!("/api/0/buckets/{bucket_id}"))
            .header(rocket::http::ContentType::JSON)
            .header(Self::host_header())
            .body(body)
            .dispatch()
            .await;
        self.known_buckets.lock().await.insert(bucket_id.to_string());
    }

    async fn heartbeat(&self, bucket_id: &str, pulsetime: f64, event: &serde_json::Value) {
        let _ = self
            .client
            .post(format!("/api/0/buckets/{bucket_id}/heartbeat?pulsetime={pulsetime}"))
            .header(rocket::http::ContentType::JSON)
            .header(Self::host_header())
            .body(event.to_string())
            .dispatch()
            .await;
    }

    async fn insert_event(&self, bucket_id: &str, event: &serde_json::Value) {
        let body = serde_json::json!([event]).to_string();
        let _ = self
            .client
            .post(format!("/api/0/buckets/{bucket_id}/events"))
            .header(rocket::http::ContentType::JSON)
            .header(Self::host_header())
            .body(body)
            .dispatch()
            .await;
    }

    /// Esegue una query AQL sul motore già usato da tutta la webui
    /// (`/api/0/query`, vedi `aw-server-rust-src/aw-query`) — a differenza
    /// degli altri metodi sopra, questo LEGGE davvero il body della
    /// risposta invece di scartarlo, perché chi lo chiama (i tool
    /// dell'agente AI, vedi agent.rs) ha bisogno del risultato, non solo
    /// di sapere che la richiesta è partita.
    async fn query(
        &self,
        timeperiods: Vec<String>,
        query_lines: Vec<String>,
    ) -> Result<serde_json::Value, String> {
        let body = serde_json::json!({
            "timeperiods": timeperiods,
            "query": query_lines,
        })
        .to_string();
        let local_resp = self
            .client
            .post("/api/0/query")
            .header(rocket::http::ContentType::JSON)
            .header(Self::host_header())
            .body(body)
            .dispatch()
            .await;
        let status = local_resp.status().code;
        let bytes = local_resp.into_bytes().await.unwrap_or_default();
        if !(200..300).contains(&status) {
            return Err(format!(
                "query AQL fallita (status {status}): {}",
                String::from_utf8_lossy(&bytes)
            ));
        }
        serde_json::from_slice(&bytes).map_err(|e| format!("risposta query non valida: {e}"))
    }

    /// Elenca tutti i bucket noti (`/api/0/buckets`) — usato dal tool
    /// `elenca_bucket` dell'agente AI per sapere cosa esiste prima di
    /// interrogare un periodo. Risposta un oggetto `{id_bucket: {...}}`
    /// (vedi `aw-server-rust-src/aw-server/src/endpoints/bucket.rs`).
    async fn list_buckets(&self) -> Result<serde_json::Value, String> {
        let local_resp =
            self.client.get("/api/0/buckets").header(Self::host_header()).dispatch().await;
        let status = local_resp.status().code;
        let bytes = local_resp.into_bytes().await.unwrap_or_default();
        if !(200..300).contains(&status) {
            return Err(format!("elenco bucket fallito (status {status})"));
        }
        serde_json::from_slice(&bytes).map_err(|e| format!("risposta bucket non valida: {e}"))
    }

    /// Legge un'impostazione tramite lo stesso endpoint già usato dalla
    /// webui (`/api/0/settings/<key>`, vedi
    /// `aw-server-rust-src/aw-server/src/endpoints/settings.rs`) — questo
    /// endpoint ritorna sempre 200 con `null` se la chiave non esiste
    /// ancora, mai un errore, quindi `None` qui significa specificamente
    /// "chiave assente o valore letteralmente null", non un errore di
    /// rete. Usato dalla categorizzazione automatica (`categorization.rs`)
    /// per leggere/scrivere `appCategories` — la stessa chiave che in
    /// futuro leggerà anche l'interfaccia Impostazioni per le categorie.
    async fn get_setting(&self, key: &str) -> Option<serde_json::Value> {
        let local_resp =
            self.client.get(format!("/api/0/settings/{key}")).header(Self::host_header()).dispatch().await;
        if local_resp.status().code != 200 {
            return None;
        }
        let bytes = local_resp.into_bytes().await.unwrap_or_default();
        let valore: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
        if valore.is_null() {
            None
        } else {
            Some(valore)
        }
    }

    /// Scrive un'impostazione (stesso endpoint di `get_setting`, corpo =
    /// il valore JSON grezzo, non incapsulato — stesso contratto già
    /// usato dalla webui in `stores/settings.ts`).
    async fn set_setting(&self, key: &str, value: &serde_json::Value) -> Result<(), String> {
        let local_resp = self
            .client
            .post(format!("/api/0/settings/{key}"))
            .header(rocket::http::ContentType::JSON)
            .header(Self::host_header())
            .body(value.to_string())
            .dispatch()
            .await;
        let status = local_resp.status().code;
        if !(200..300).contains(&status) {
            let bytes = local_resp.into_bytes().await.unwrap_or_default();
            return Err(format!(
                "scrittura impostazione '{key}' fallita (status {status}): {}",
                String::from_utf8_lossy(&bytes)
            ));
        }
        Ok(())
    }

    /// Inoltra una richiesta HTTP arbitraria (dalla webview, tramite il
    /// protocollo personalizzato) al Rocket in-process, senza toccare mai
    /// la rete. Copre sia le pagine statiche della webui sia le API.
    async fn dispatch(&self, req: http::Request<Vec<u8>>) -> http::Response<Vec<u8>> {
        let method = match req.method().as_str() {
            "GET" => rocket::http::Method::Get,
            "POST" => rocket::http::Method::Post,
            "PUT" => rocket::http::Method::Put,
            "DELETE" => rocket::http::Method::Delete,
            "OPTIONS" => rocket::http::Method::Options,
            "HEAD" => rocket::http::Method::Head,
            "PATCH" => rocket::http::Method::Patch,
            _ => rocket::http::Method::Get,
        };
        let path_and_query = req
            .uri()
            .path_and_query()
            .map(|p| p.as_str().to_string())
            .unwrap_or_else(|| "/".to_string());

        // Intercetta il cambio di alcune impostazioni lette dal watcher
        // screenshot (intervallo di cattura, giorni di conservazione):
        // il watcher non parla più in rete (Fase 5), quindi non può
        // rileggerle da qui — le riscriviamo in piccoli file locali che
        // il watcher rilegge ad ogni giro (vedi il suo main.rs). Catturato
        // PRIMA di consumare `req` con `.into_body()` più sotto.
        let override_file = if req.method() == http::Method::POST {
            match req.uri().path() {
                "/api/0/settings/screenshotIntervalSeconds" => Some("screenshot-interval-override.txt"),
                "/api/0/settings/screenshotRetentionDays" => Some("screenshot-retention-days-override.txt"),
                _ => None,
            }
        } else {
            None
        };
        let body_for_override = override_file.map(|_| req.body().clone());

        let mut local_req = self.client.req(method, path_and_query);
        for (name, value) in req.headers() {
            // L'header Origin che WebView2 mette sulle richieste verso uno
            // schema personalizzato ("http://trackflow.localhost") non è
            // nella lista consentita dal fairing CORS del server (pensato
            // per una vera origine di rete) — non c'è comunque nessuna
            // vera richiesta cross-origin qui (l'unico chiamante possibile
            // è questa stessa webview), quindi lo scartiamo invece di
            // toccare la configurazione CORS del server.
            if name.as_str().eq_ignore_ascii_case("origin") {
                continue;
            }
            if let Ok(v) = value.to_str() {
                local_req = local_req.header(rocket::http::Header::new(name.to_string(), v.to_string()));
            }
        }
        // Sovrascrive/forza sempre un Host valido: la webview manda un
        // Host legato al nostro schema personalizzato (es.
        // "trackflow.localhost"), che il fairing HostCheck del server
        // rifiuterebbe altrimenti.
        local_req = local_req.header(Self::host_header());
        local_req = local_req.body(req.into_body());

        let local_resp = local_req.dispatch().await;
        let status = local_resp.status().code;
        let headers: Vec<(String, String)> = local_resp
            .headers()
            .iter()
            .map(|h| (h.name().to_string(), h.value().to_string()))
            .collect();
        let body = local_resp.into_bytes().await.unwrap_or_default();

        if (200..300).contains(&status) {
            if let (Some(filename), Some(raw_value)) = (override_file, body_for_override) {
                // Il valore JSON grezzo può essere un numero o una
                // stringa numerica (come già gestito lato watcher in
                // leggi_intervallo) — scriviamo esattamente i byte
                // inviati, senza reinterpretarli qui.
                let testo = String::from_utf8_lossy(&raw_value);
                let testo = testo.trim().trim_matches('"');
                let path = self.app_data_dir.join(filename);
                let _ = std::fs::write(path, testo);
            }
        }

        let mut builder = http::Response::builder().status(status);
        for (name, value) in headers {
            builder = builder.header(name, value);
        }
        // Senza questo, WebView2 mette in cache le risposte del
        // protocollo personalizzato (pagine, JS, CSS) di sua iniziativa
        // — nessun header di per sé lo dice esplicitamente, ma il
        // comportamento di default per uno schema personalizzato è
        // comunque cache-friendly. Risultato concreto osservato: dopo
        // una modifica alla webui e un rebuild, la finestra continuava a
        // mostrare l'interfaccia VECCHIA finché non si cancellava a mano
        // la cache di WebView2 — capitato con la barra del titolo
        // personalizzata durante lo sviluppo di questa stessa funzione.
        builder = builder.header("Cache-Control", "no-store");
        builder.body(body).unwrap_or_else(|_| {
            http::Response::builder()
                .status(500)
                .body(Vec::new())
                .expect("static response is always valid")
        })
    }
}

/// Aspetta che il server in-process risponda su /api/0/info prima di
/// mostrare la finestra — evita di mostrare una pagina di errore di
/// connessione al primo istante, anche se ora il dispatch è in memoria
/// e quindi quasi istantaneo (il datastore/le migrazioni SQLite alla
/// primissima apertura possono comunque richiedere qualche istante).
async fn wait_for_server_ready(server: &AppServer, timeout: std::time::Duration) -> bool {
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        let resp = server
            .client
            .get("/api/0/info")
            .header(AppServer::host_header())
            .dispatch()
            .await;
        if resp.status().code == 200 {
            return true;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    false
}

/// Rete di sicurezza contro un bug segnalato dall'utente: a volte, subito
/// dopo l'avvio, la finestra mostra per qualche secondo la pagina di
/// errore nativa di WebView2 ("Impossibile raggiungere la pagina") prima
/// di caricare TrackFlow correttamente da sola. Non è il nostro server —
/// `wait_for_server_ready` sopra garantisce che sia già pronto PRIMA che
/// la finestra venga creata — ma una corsa interna alla primissima
/// inizializzazione dell'ambiente WebView2 stesso sulla prima
/// navigazione verso lo schema personalizzato "trackflow://" (nota
/// occasionale di WebView2 su macchine/avvi più lenti). Qui agganciamo
/// l'evento nativo `NavigationCompleted`: se una navigazione fallisce, la
/// ricarichiamo subito invece di lasciare che l'utente veda l'errore per
/// qualche secondo finché WebView2 non si riprende da solo.
fn attach_navigation_retry(window: &tauri::WebviewWindow) {
    let window_for_retry = window.clone();
    let result = window.with_webview(move |webview| {
        let core_webview = match unsafe { webview.controller().CoreWebView2() } {
            Ok(cw) => cw,
            Err(e) => {
                log::error!("Impossibile agganciare il retry di navigazione WebView2: {e}");
                return;
            }
        };
        let handler = webview2_com::NavigationCompletedEventHandler::create(Box::new(
            move |_sender, args| {
                if let Some(args) = args {
                    let mut success = windows_core::BOOL(0);
                    if unsafe { args.IsSuccess(&mut success) }.is_ok() && !success.as_bool() {
                        log::warn!("Navigazione iniziale della finestra fallita, ricarico...");
                        let _ = window_for_retry.reload();
                    }
                }
                Ok(())
            },
        ));
        let mut token: i64 = 0;
        if let Err(e) = unsafe { core_webview.add_NavigationCompleted(&handler, &mut token) } {
            log::error!("Impossibile registrare NavigationCompleted: {e}");
        }
    });
    if let Err(e) = result {
        log::error!("with_webview fallita per il retry di navigazione: {e}");
    }
}

/// Costruisce il server `aw-server-rust` in-process: stesso identico
/// codice del binario originale (`aw_server::endpoints::build_rocket`),
/// solo dispatchato in memoria invece che su una vera porta TCP — vedi
/// BLUEPRINT.md, Fase 5, per il perché e la verifica di fattibilità.
async fn build_app_server(app_data_dir: &Path, webui_dir: &Path, watcher_templates_dir: &Path) -> AppServer {
    let testing = false;
    let mut config = aw_server::config::create_config(testing, None);
    config
        .custom_static
        .insert("app-data".to_string(), app_data_dir.to_string_lossy().to_string());

    // Il database vive dentro app_data_dir (stessa cartella di
    // screenshot/icone/config, quella che l'installer NSIS cancella
    // già alla disinstallazione) invece che nel percorso di default
    // di aw-server-rust vendorizzato (aw_server::dirs::db_path,
    // %APPDATA%\activitywatch\aw-server-rust — ancora col nome del
    // progetto originale, mai aggiornato al rebrand). Bug reale
    // segnalato dall'utente: nessuna disinstallazione toccava mai
    // quel percorso, quindi la cronologia della Timeline sopravviveva
    // sempre, anche a "dati cancellati" confermato dall'utente.
    let db_path = app_data_dir
        .join("sqlite.db")
        .to_str()
        .expect("Percorso del database non valido UTF-8")
        .to_string();
    let legacy_import = true;
    let datastore = aw_datastore::Datastore::new(db_path, legacy_import);

    let server_state = aw_server::endpoints::ServerState {
        datastore,
        asset_resolver: aw_server::endpoints::AssetResolver::new(Some(webui_dir.to_path_buf())),
        device_id: aw_server::device_id::get_device_id(),
    };

    // Registro nome->cartella dei moduli personalizzati (vedi
    // custom_modules.rs) — un solo Arc condiviso tra questo Rocket (che
    // lo consulta ad ogni richiesta a /pages/custom/<nome>/) e i comandi
    // Tauri che lo ripopolano quando l'utente apre il selettore "Aggiungi
    // modulo" o preme "Aggiorna", permettendo di scoprire nuove cartelle
    // senza mai dover riavviare il server.
    let custom_pages_registry: Arc<aw_server::endpoints::CustomPagesRegistry> =
        Arc::new(std::sync::RwLock::new(HashMap::new()));

    let watcher_templates_dir = Arc::new(aw_server::endpoints::WatcherTemplatesDir(
        watcher_templates_dir.to_path_buf(),
    ));
    let rocket = aw_server::endpoints::build_rocket(
        server_state,
        config,
        custom_pages_registry.clone(),
        watcher_templates_dir,
    );
    let client = rocket::local::asynchronous::Client::tracked(rocket)
        .await
        .expect("Impossibile avviare il server aw-server-rust in-process");

    AppServer {
        client,
        known_buckets: AsyncMutex::new(HashSet::new()),
        app_data_dir: app_data_dir.to_path_buf(),
        custom_pages_registry,
    }
}

/// Un evento ricevuto dallo stdout di un watcher (una riga JSON — vedi
/// BLUEPRINT.md, Fase 5, per il contratto completo, documentato anche
/// per watcher di terze parti). Pubblico nel crate: è lo stesso contratto
/// riusato dai watcher personalizzati in modalità "raw" (custom_watchers.rs).
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct WatcherEnvelope {
    pub(crate) bucket_id: String,
    pub(crate) bucket_type: String,
    pub(crate) client: String,
    pub(crate) op: String,
    #[serde(default)]
    pub(crate) pulsetime: Option<f64>,
    pub(crate) event: serde_json::Value,
}

/// Inoltra un envelope già interpretato al server in-process — crea il
/// bucket se serve (idempotente) poi dispatcha heartbeat/event. Punto di
/// ingresso condiviso tra il drenaggio stdout dei sidecar integrati
/// (spawn_stdout_drain) e i watcher personalizzati (custom_watchers.rs,
/// sia modalità "raw" che l'envelope sintetico della modalità "interval")
/// — un solo posto che parla con AppServer, invece di duplicare la
/// stessa logica ensure_bucket+dispatch in più punti.
pub(crate) async fn elabora_envelope_watcher(
    envelope: &WatcherEnvelope,
    server: &Arc<AppServer>,
    hostname: &str,
) {
    server
        .ensure_bucket(&envelope.bucket_id, &envelope.bucket_type, &envelope.client, hostname)
        .await;

    match envelope.op.as_str() {
        "heartbeat" => {
            let pulsetime = envelope.pulsetime.unwrap_or(60.0);
            server.heartbeat(&envelope.bucket_id, pulsetime, &envelope.event).await;
        }
        "event" => {
            server.insert_event(&envelope.bucket_id, &envelope.event).await;
        }
        other => {
            log::warn!("{}: op sconosciuta '{other}' ignorata", envelope.client);
        }
    }
}

/// Drena in continuo lo stdout di un watcher già avviato, interpretando
/// ogni riga come un `WatcherEnvelope` e inoltrandola al server
/// in-process. Il watcher non tocca mai la rete: è questo loop (che gira
/// nel processo Tauri, lo stesso che possiede il datastore) a "tirare" i
/// dati leggendoli dalla pipe privata stdout del figlio. Termina da solo
/// quando il processo watcher muore (canale chiuso) — sia per un vero
/// crash, sia perché il menu Moduli l'ha spento apposta.
///
/// `app_icons_stdin`, quando presente (solo per il watcher finestra),
/// riceve anche il nome dell'app della finestra attiva ad ogni riga —
/// stesso principio: aw-watcher-app-icons non parla più in rete (vedi
/// il suo main.rs), quindi è questo stesso loop a "spingergli" i nomi
/// scoperti sulla sua pipe stdin privata, invece di lasciarlo interrogare
/// il bucket finestra via HTTP com'era prima della Fase 5.
fn spawn_stdout_drain(
    watcher_name: String,
    mut rx: tauri::async_runtime::Receiver<CommandEvent>,
    server: Arc<AppServer>,
    hostname: String,
    app_icons_stdin: Option<IconsHandle>,
    categorization_ctx: Option<(AppHandle, Arc<categorization::CategorizationState>, PathBuf)>,
) {
    tauri::async_runtime::spawn(async move {
        while let Some(evt) = rx.recv().await {
            match &evt {
                CommandEvent::Stderr(bytes) => {
                    let line = String::from_utf8_lossy(bytes);
                    let line = line.trim();
                    if !line.is_empty() {
                        log::error!(target: "watcher", "[{watcher_name}] {line}");
                    }
                    continue;
                }
                CommandEvent::Error(err) => {
                    log::error!(target: "watcher", "[{watcher_name}] errore processo: {err}");
                    continue;
                }
                CommandEvent::Terminated(payload) => {
                    log::warn!(target: "watcher", "[{watcher_name}] processo terminato (codice {:?})", payload.code);
                    continue;
                }
                _ => {}
            }
            if let CommandEvent::Stdout(bytes) = evt {
                let line = String::from_utf8_lossy(&bytes);
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let envelope: WatcherEnvelope = match serde_json::from_str(line) {
                    Ok(e) => e,
                    Err(_) => {
                        // Riga di log normale del watcher (non un envelope
                        // JSON) — prima veniva scartata in silenzio, ora
                        // finisce nello stesso flusso di log dell'app
                        // (vedi target Webview sul plugin log qui sopra),
                        // così il pannello "Log" in Impostazioni →
                        // Sviluppatore la mostra come tutto il resto.
                        log::info!(target: "watcher", "[{watcher_name}] {line}");
                        continue;
                    }
                };

                if let Some(icons_stdin) = &app_icons_stdin {
                    if let Some(app) = envelope
                        .event
                        .get("data")
                        .and_then(|d| d.get("app"))
                        .and_then(|a| a.as_str())
                    {
                        let riga = format!("{app}\n");
                        if let Ok(mut guard) = icons_stdin.lock() {
                            if let Some(child) = guard.as_mut() {
                                let _ = child.write(riga.as_bytes());
                            }
                        }

                        // Stesso identico nome app appena inoltrato al
                        // watcher icone, ora anche alla categorizzazione
                        // automatica AI (categorization.rs) — non
                        // bloccante, vedi il commento su su_nuova_app per
                        // il perché. Modulo "ai-categorization" spegnibile
                        // a parte dal menu Moduli (letto dal vivo qui, non
                        // in base a quando il watcher finestra è partito —
                        // vedi spawn_watcher/stop_watcher per il perché è
                        // un caso speciale senza processo vero).
                        if let Some((app_handle, stato, app_data_dir)) = &categorization_ctx {
                            // Registrazione dell'app conosciuta SEMPRE,
                            // indipendentemente da AI/categorizzazione —
                            // richiesta esplicita dell'utente: l'elenco
                            // deve restare aggiornato dall'installazione
                            // in poi, a prescindere da quando/se viene
                            // mai collegata una chiave AI (usato anche
                            // dalle Impostazioni, sezioni Notifiche e
                            // Categorizzazione).
                            categorization::registra_app_se_nuova(app_data_dir, &app.to_lowercase());

                            let categorizzazione_attiva = app_handle
                                .try_state::<ModulesConfigState>()
                                .map(|c| *c.0.lock().unwrap().get("ai-categorization").unwrap_or(&true))
                                .unwrap_or(true);
                            if categorizzazione_attiva {
                                categorization::su_nuova_app(
                                    server.clone(),
                                    app_data_dir.clone(),
                                    stato.clone(),
                                    hostname.clone(),
                                    app.to_lowercase(),
                                );
                            }
                        }
                    }
                }

                elabora_envelope_watcher(&envelope, &server, &hostname).await;
            }
        }
    });
}

/// Avvia `aw-watcher-app-icons` e mette il `CommandChild` nell'handle
/// condiviso (sostituendo quello che c'era, se ce n'era già uno morto) —
/// usato sia all'avvio dell'app (se il modulo è acceso in config) sia da
/// un click sul menu Moduli che lo riaccende.
/// Emesso ogni volta che la finestra torna in primo piano da nascosta
/// (tray, doppio click, o una seconda istanza rilanciata — vedi i tre
/// punti di chiamata). Il frontend (App.vue) lo ascolta per ricontrollare
/// gli aggiornamenti — l'evento nativo `onFocusChanged` di Tauri si è
/// rivelato inaffidabile per questo caso specifico (nascondere una
/// finestra con `hide()` non genera sempre un vero evento di perdita
/// del focus lato Tauri, quindi mostrarla di nuovo non genera sempre un
/// evento di "cambio" — bug reale segnalato dall'utente: lasciando
/// l'app in tray per un po' e poi riportandola in primo piano, il
/// controllo aggiornamenti non ripartiva mai). Un evento esplicito,
/// emesso esattamente nei punti in cui NOI decidiamo di mostrare la
/// finestra, non dipende da quella semantica ambigua.
fn emit_finestra_mostrata(window: &tauri::WebviewWindow) {
    let _ = window.emit("trackflow://finestra-mostrata", ());
}

fn spawn_app_icons(app_handle: &AppHandle, app_data_dir: &Path, icons_stdin: &IconsHandle) {
    match app_handle.shell().sidecar("aw-watcher-app-icons") {
        Ok(cmd) => match cmd.args(["--app-data-dir", &app_data_dir.to_string_lossy()]).spawn() {
            Ok((_rx, child)) => {
                if let Ok(mut guard) = icons_stdin.lock() {
                    *guard = Some(child);
                }
                // Scansione proattiva delle app GIÀ aperte in questo
                // momento — senza questa, il watcher (puramente reattivo:
                // estrae un'icona solo quando un'app diventa la finestra
                // ATTIVA, vedi aw-watcher-app-icons-rust/src/main.rs)
                // lascia icone generiche per ogni app già in Home da dati
                // storici finché l'utente non la riapre e ci clicca
                // sopra almeno una volta. Segnalato dall'utente su
                // un'installazione pulita. Copre solo le app già in
                // esecuzione ORA, non quelle usate in passato e non più
                // aperte — per quelle resta il comportamento reattivo di
                // sempre.
                invia_processi_in_esecuzione_a_icone(icons_stdin);
            }
            Err(e) => log::error!("Impossibile avviare aw-watcher-app-icons: {e}"),
        },
        Err(e) => log::error!("Sidecar aw-watcher-app-icons non trovato nel bundle: {e}"),
    }
}

/// Elenca i processi attualmente in esecuzione (ToolHelp32 snapshot,
/// stessa API usata da Task Manager) e li spinge sulla pipe stdin di
/// aw-watcher-app-icons esattamente come farebbe il watcher finestra per
/// l'app attiva — riusa tutta la logica di filtro/estrazione già esistente
/// lì (app di sistema nascoste, icone già note saltate, ecc.) senza
/// doverla duplicare qui.
#[cfg(windows)]
fn invia_processi_in_esecuzione_a_icone(icons_stdin: &IconsHandle) {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
    };

    let mut nomi: HashSet<String> = HashSet::new();
    unsafe {
        let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
            log::warn!("Impossibile enumerare i processi in esecuzione per la scansione icone iniziale");
            return;
        };
        let mut voce: PROCESSENTRY32W = std::mem::zeroed();
        voce.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        if Process32FirstW(snapshot, &mut voce).is_ok() {
            loop {
                let fine = voce.szExeFile.iter().position(|&c| c == 0).unwrap_or(voce.szExeFile.len());
                let nome = String::from_utf16_lossy(&voce.szExeFile[..fine]).to_lowercase();
                if !nome.is_empty() {
                    nomi.insert(nome);
                }
                if Process32NextW(snapshot, &mut voce).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snapshot);
    }

    if let Ok(mut guard) = icons_stdin.lock() {
        if let Some(child) = guard.as_mut() {
            for nome in nomi {
                let riga = format!("{nome}\n");
                let _ = child.write(riga.as_bytes());
            }
        }
    }
}

#[cfg(not(windows))]
fn invia_processi_in_esecuzione_a_icone(_icons_stdin: &IconsHandle) {}

/// Avvia uno dei 5 watcher "generici" (non app-icons, vedi
/// `spawn_app_icons`), lo registra in `SidecarProcesses` e ne collega lo
/// stdout al server — stessa funzione usata sia all'avvio (per i moduli
/// accesi in config) sia da un click sul menu Moduli.
fn spawn_watcher(
    app_handle: &AppHandle,
    name: &str,
    server: Arc<AppServer>,
    hostname: String,
    app_data_dir: &Path,
    icons_stdin: IconsHandle,
) {
    if name == "aw-watcher-app-icons" {
        spawn_app_icons(app_handle, app_data_dir, &icons_stdin);
        return;
    }
    if name == "ai-categorization" {
        // Virtuale come VoiSpeed sotto — nessun processo da avviare, il
        // toggle viene letto dal vivo dentro spawn_stdout_drain ad ogni
        // evento (vedi categorization.rs). Accenderlo dal menu Moduli non
        // deve far nulla qui: la spunta/il salvataggio su file sono già
        // gestiti dall'handler del menu, PRIMA di chiamare questa
        // funzione.
        return;
    }
    if name == "aw-watcher-voispeed" {
        // Non è un sidecar (vedi ALL_MODULES) — riprende il polling
        // in-process solo se un account risulta già collegato. Accendere
        // il modulo dal menu Moduli non avvia da solo un login: la prima
        // volta serve comunque il pulsante "Collega VoiSpeed" nelle
        // Impostazioni.
        if voispeed::load_identity(app_data_dir).is_some() {
            voispeed::avvia_polling(app_handle.clone(), server, hostname);
        }
        return;
    }

    let cmd = match app_handle.shell().sidecar(name) {
        Ok(c) => c,
        Err(e) => {
            log::error!("Sidecar {name} non trovato nel bundle: {e}");
            return;
        }
    };
    let cmd = if matches!(name, "aw-watcher-screenshot" | "aw-watcher-vpn" | "aw-watcher-claude-code") {
        // Stessa cartella scrivibile condivisa delle icone. Screenshot ci
        // salva sotto <app-data-dir>/screenshots (vedi
        // ScreenshotGalleryModal.vue); vpn/claude-code ci leggono
        // client_mapping.txt/project_mapping.txt e ci salvano lo stato —
        // prima cercavano questi file "accanto all'exe", percorso che
        // cambia tra dev/release/installazione reale e in un'installazione
        // vera sarebbe di sola lettura (stesso problema già risolto per
        // le icone).
        cmd.args(["--app-data-dir", &app_data_dir.to_string_lossy()])
    } else {
        cmd
    };

    match cmd.spawn() {
        Ok((rx, child)) => {
            let processes = app_handle.state::<SidecarProcesses>();
            processes.0.lock().unwrap().insert(name.to_string(), child);
            let forward_to_icons = if name == "aw-watcher-window" { Some(icons_stdin) } else { None };
            // Stessa condizione di forward_to_icons (solo il watcher
            // finestra ci dice quale app è in primo piano) — la
            // categorizzazione automatica AI si aggancia allo stesso
            // punto, vedi categorization.rs.
            let categorization_ctx = if name == "aw-watcher-window" {
                app_handle
                    .try_state::<Arc<categorization::CategorizationState>>()
                    .map(|s| (app_handle.clone(), s.inner().clone(), app_data_dir.to_path_buf()))
            } else {
                None
            };
            spawn_stdout_drain(name.to_string(), rx, server, hostname, forward_to_icons, categorization_ctx);
        }
        Err(e) => log::error!("Impossibile avviare {name}: {e}"),
    }
}

/// Ferma un modulo già in esecuzione (per nome) — usato da un click sul
/// menu Moduli che lo spegne. Nessun errore se non era in esecuzione
/// (già spento, o mai partito).
fn stop_watcher(app_handle: &AppHandle, name: &str) {
    if name == "aw-watcher-app-icons" {
        if let Some(icons) = app_handle.try_state::<IconsHandle>() {
            if let Ok(mut guard) = icons.lock() {
                if let Some(child) = guard.take() {
                    let _ = child.kill();
                }
            }
        }
        return;
    }
    if name == "aw-watcher-voispeed" {
        voispeed::ferma_polling(app_handle);
        return;
    }
    if name == "ai-categorization" {
        // Virtuale, vedi spawn_watcher — nessun processo da fermare.
        return;
    }
    let processes = app_handle.state::<SidecarProcesses>();
    let removed = processes.0.lock().unwrap().remove(name);
    if let Some(child) = removed {
        let _ = child.kill();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    setup_kill_on_exit_job();

    tauri::Builder::default()
        // DEVE essere il primo plugin registrato (richiesto da Tauri per
        // funzionare correttamente su Windows). Senza questo, ogni click
        // sul collegamento/lancio di app.exe mentre un'istanza è già in
        // esecuzione avviava un SECONDO processo indipendente — bug
        // reale trovato durante i test dell'installer: server Rocket in
        // conflitto sulla porta 5600 (schermata bianca nella nuova
        // istanza, che non riesce a partire), icone tray duplicate,
        // watcher duplicati. Ora un secondo lancio si limita a
        // richiamare in primo piano la finestra dell'istanza già aperta,
        // stesso comportamento già usato per il doppio click sulla tray
        // qui sotto.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
                emit_finestra_mostrata(&window);
            }
        }))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_shell::init())
        // Dialogo "Salva con nome" nativo + scrittura file — servono a
        // util/export.ts's downloadFile() per il pulsante "Esporta come
        // CSV" (i link <a download> non funzionano dentro una webview
        // Tauri, vedi il commento in quel file).
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(SidecarProcesses(std::sync::Mutex::new(HashMap::new())))
        .invoke_handler(tauri::generate_handler![
            voispeed::voispeed_connect,
            voispeed::voispeed_status,
            voispeed::voispeed_disconnect,
            voispeed::voispeed_refresh_check,
            agent::ai_agent_get_config,
            agent::ai_agent_save_config,
            agent::ai_agent_send_message,
            agent::ai_agent_new_conversation,
            agent::ai_agent_list_models,
            categorization::elenca_app_conosciute,
            devtools::apri_devtools,
            watcher_status::stato_watcher,
            watcher_status::avvia_watcher_sessione,
            watcher_status::ferma_watcher_sessione,
            watcher_status::riavvia_watcher,
            watcher_status::imposta_moduli_iniziali,
            watcher_status::moduli_gia_configurati,
            watcher_status::imposta_modulo_watcher,
            folder_shortcuts::apri_cartella_dati,
            folder_shortcuts::apri_cartella_log,
            folder_shortcuts::apri_cartella_config_afk,
            folder_shortcuts::apri_cartella_watcher,
            folder_shortcuts::apri_cartella_database,
            custom_watchers::elenca_watcher_personalizzati,
            custom_watchers::ricarica_watcher_personalizzati,
            custom_watchers::crea_watcher_personalizzato_semplice,
            custom_watchers::crea_watcher_personalizzato_esperto,
            custom_watchers::apri_cartella_custom_watcher,
            custom_watchers::elimina_watcher_personalizzato,
            custom_watchers::leggi_log_watcher,
            custom_watchers::imposta_file_watcher,
            custom_watchers::elenca_modelli_visualizzazione_watcher,
            custom_modules::elenca_moduli_personalizzati,
            custom_modules::ricarica_moduli_personalizzati,
            custom_modules::crea_modulo_personalizzato,
            custom_modules::apri_cartella_custom_modulo,
            vpn_mapping::leggi_mapping_vpn,
            vpn_mapping::salva_mapping_vpn_manuale,
            notifications::invia_notifica_generica,
            updater::controlla_aggiornamento,
            updater::scarica_e_prepara_aggiornamento,
            updater::installa_aggiornamento_e_riavvia,
            autostart::imposta_avvio_automatico,
            autostart::avvio_automatico_abilitato,
            about::leggi_changelog
        ])
        .register_asynchronous_uri_scheme_protocol("trackflow", |ctx, request, responder| {
            let app_handle = ctx.app_handle().clone();
            tauri::async_runtime::spawn(async move {
                let server = app_handle.state::<Arc<AppServer>>();
                let response = server.dispatch(request).await;
                responder.respond(response);
            });
        })
        .setup(|app| {
            app.handle().plugin(
                tauri_plugin_log::Builder::default()
                    .level(log::LevelFilter::Info)
                    // Rocket (il server HTTP in-process, vedi
                    // aw-server-rust-src) logga a livello Info OGNI
                    // singola richiesta — con un watcher finestra che
                    // manda un heartbeat al secondo sono 3 righe/secondo
                    // di puro rumore, che nel pannello "Log" seppelliva
                    // subito le righe utili (avvii/errori dei watcher).
                    // Alzata a Warn solo per questo target: gli errori
                    // veri di Rocket restano visibili, il traffico
                    // normale no.
                    .level_for("rocket", log::LevelFilter::Warn)
                    // Oltre ai due target di default (Stdout + file nella
                    // cartella log), manda ogni riga anche alla webview
                    // come evento "log://log" — usato dal pannello "Log"
                    // in Impostazioni → Sviluppatore per mostrare i log
                    // dal vivo senza dover reindirizzare stdout/stderr a
                    // mano su file, come si è fatto per tutta questa
                    // sessione di sviluppo.
                    .targets([
                        tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
                        tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir {
                            file_name: None,
                        }),
                        tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Webview),
                    ])
                    .build(),
            )?;

            let app_handle = app.handle().clone();
            let resource_dir = app
                .path()
                .resource_dir()
                .expect("Impossibile risolvere la cartella risorse dell'app");
            let webui_dir = resource_dir.join("dist");
            let watcher_templates_dir = resource_dir.join("watcher-templates");

            let app_data_dir = std::env::var("LOCALAPPDATA")
                .map(PathBuf::from)
                .unwrap_or_else(|_| resource_dir.clone())
                .join("TrackFlow")
                .join("app-data");
            let _ = std::fs::create_dir_all(&app_data_dir);

            // Solo qui, non subito dopo un'installazione/aggiornamento:
            // arrivare fin qui in .setup() senza essere andati in panic
            // è già una conferma ragionevole che QUESTA versione
            // funziona, prima di eventualmente cancellare quella
            // precedente rimasta come rete di sicurezza. Vedi il
            // commento su pulisci_versioni_vecchie() per il perché
            // (richiesta esplicita dell'utente: le cartelle di versione
            // si accumulavano all'infinito, mai ripulite).
            updater::pulisci_versioni_vecchie();

            let hostname = gethostname::gethostname().to_string_lossy().to_string();

            // Quali moduli erano accesi/spenti l'ultima volta — riletto
            // PRIMA di avviare qualunque sidecar, così un modulo spento
            // dall'utente non riparte mai da solo (richiesta esplicita).
            let modules_config = load_modules_config(&app_data_dir);

            app.manage(AppDataDirState(app_data_dir.clone()));
            app.manage(ModulesConfigState(std::sync::Mutex::new(modules_config.clone())));

            // VoiSpeed: se risultava già collegato in una sessione
            // precedente (identità persistita, MAI il token — vedi
            // voispeed.rs), lo stato riparte già "connesso" e il ciclo di
            // polling viene avviato più sotto, appena il server è pronto.
            let voispeed_identity_pregressa = voispeed::load_identity(&app_data_dir);
            let voispeed_state = Arc::new(VoiSpeedState::new());
            if let Some(identity) = &voispeed_identity_pregressa {
                *voispeed_state.identity.lock().unwrap() = Some(identity.clone());
                voispeed_state.status.lock().unwrap().connected = true;
            }
            app.manage(voispeed_state);
            app.manage(Arc::new(agent::AiAgentState::new()));
            app.manage(Arc::new(categorization::CategorizationState::new()));
            let icons_stdin: IconsHandle = Arc::new(std::sync::Mutex::new(None));
            app.manage(icons_stdin.clone());
            app.manage(custom_watchers::CustomWatcherProcesses::new());

            // Server in-process, protocollo personalizzato, watcher —
            // SEMPRE, sia in debug che in release. Prima questo blocco
            // girava solo in release (una `cargo build` di debug puntava
            // invece a un vite dev server esterno, senza toccare affatto
            // il codice della Fase 5), il che costringeva a una build
            // release completa — lenta — per ogni minima modifica da
            // verificare. Ora `cargo build` (debug, veloce) seguito da
            // `target/debug/app.exe` esercita lo STESSO identico
            // percorso di produzione, solo non ottimizzato — la build
            // release "vera" (e l'installer) serve solo quando si è
            // pronti a distribuire, non ad ogni iterazione. `resource_dir()`
            // risolve comunque correttamente la cartella `dist` anche
            // senza passare da `tauri build`/l'installer, bundling incluso
            // — verificato empiricamente lanciando ripetutamente l'exe
            // grezzo prodotto da `cargo build` durante questa stessa
            // sessione.
            {
                let app_handle = app_handle.clone();
                let app_data_dir = app_data_dir.clone();
                let hostname = hostname.clone();
                let modules_config = modules_config.clone();
                let icons_stdin = icons_stdin.clone();

                tauri::async_runtime::spawn(async move {
                    let server =
                        Arc::new(build_app_server(&app_data_dir, &webui_dir, &watcher_templates_dir).await);
                    app_handle.manage(server.clone());

                    // Prima si fermava dopo 15s e creava/mostrava la finestra
                    // comunque, pronto o no il server — se capitava davvero di
                    // superare i 15s (macchina lenta, disco occupato, ecc.) la
                    // primissima navigazione verso "trackflow://" partiva
                    // contro un server non ancora pronto, con buone probabilità
                    // di fallire e mostrare la pagina di errore nativa di
                    // WebView2 ("Impossibile raggiungere la pagina") segnalata
                    // dall'utente. Ora si continua ad aspettare (loggando ogni
                    // 15s) finché il server non risponde davvero: in condizioni
                    // normali richiede comunque una frazione di secondo, quindi
                    // non introduce un'attesa percepibile.
                    loop {
                        if wait_for_server_ready(&server, std::time::Duration::from_secs(15)).await {
                            break;
                        }
                        log::error!("Il server in-process non ha ancora risposto dopo 15s, continuo ad aspettare...");
                    }

                    // La finestra viene creata SOLO ora, DIRETTAMENTE puntata sul
                    // protocollo personalizzato "trackflow" — WebView2 associa la
                    // gestione degli schemi personalizzati alla webview al momento
                    // della sua creazione, non ad ogni navigazione: creare prima
                    // una finestra con un'altra origine (o quella di default di
                    // Tauri) e poi chiamare `.navigate()` verso "trackflow://"
                    // NON funziona (verificato: `.navigate()` ritorna Ok ma il
                    // gestore del protocollo non viene mai invocato, pagina
                    // vuota). Le operazioni sulla finestra vanno eseguite sul
                    // thread principale — `run_on_main_thread` lo garantisce dato
                    // che questo codice gira in un task asincrono in background.
                    let main_thread_handle = app_handle.clone();
                    // Clone dedicato: app_data_dir "vero" resta disponibile più
                    // sotto per i watcher, questo entra nella closure move qui
                    // sotto insieme al resto.
                    let app_data_dir_finestra = app_data_dir.clone();
                    let stato_salvato = window_state::carica(&app_data_dir_finestra);
                    let _ = app_handle.run_on_main_thread(move || {
                        let mut builder = WebviewWindowBuilder::new(
                            &main_thread_handle,
                            "main",
                            WebviewUrl::CustomProtocol(
                                "trackflow://localhost/".parse().expect("URL non valido"),
                            ),
                        )
                        .title("TrackFlow")
                        .min_inner_size(900.0, 600.0)
                        .resizable(true)
                        .visible(false)
                        // Niente barra del titolo nativa di Windows —
                        // sostituita da una barra personalizzata dentro
                        // la webui stessa (App.vue, componente
                        // CustomTitlebar.vue), con gli stessi 3 pulsanti
                        // (riduci/ingrandisci/chiudi) integrati nel tema
                        // dell'app invece che nello stile di Windows.
                        .decorations(false);
                        // NON disattivare drag_and_drop qui (un tentativo
                        // precedente lo aveva fatto): su Windows serve al vero
                        // WebView2/OLE per registrare la finestra come una
                        // destinazione di drop valida e mostrare il cursore
                        // corretto durante il trascinamento — disattivarlo
                        // lasciava il cursore "vietato" per qualunque
                        // trascinamento reale da Esplora risorse, anche se gli
                        // eventi DOM standard sembravano tecnicamente
                        // disponibili. Il drag&drop dei file in Buckets.vue usa
                        // invece l'API dedicata di Tauri
                        // (getCurrentWebview().onDragDropEvent), pensata apposta
                        // per convivere con questa gestione nativa invece di
                        // sostituirla.

                        // Richiesta esplicita: riapre la finestra dove/come
                        // l'utente l'aveva lasciata l'ultima volta (schermo,
                        // posizione, dimensione) invece che sempre al centro
                        // dello schermo principale — Windows non lo fa da solo
                        // per una finestra creata così (vedi window_state.rs).
                        // La validazione contro un monitor scollegato avviene
                        // DOPO la creazione, quando available_monitors() è
                        // disponibile — qui si applica lo stato salvato per
                        // intero, ottimisticamente.
                        builder = match stato_salvato {
                            Some(stato) => builder
                                .position(stato.x as f64, stato.y as f64)
                                .inner_size(stato.width as f64, stato.height as f64),
                            None => builder.inner_size(1400.0, 900.0),
                        };

                        let window = builder.build();

                        match window {
                            Ok(window) => {
                                // Un monitor secondario scollegato dall'ultima
                                // sessione lascerebbe altrimenti la finestra
                                // fuori da qualunque schermo visibile — corretto
                                // PRIMA di un eventuale show(), mai visibile
                                // all'utente.
                                if let Some(stato) = stato_salvato {
                                    if !window_state::dentro_uno_schermo(&window, &stato) {
                                        let _ = window.center();
                                    }
                                }
                                // Richiesta esplicita: avviato dalla voce di
                                // avvio automatico di Windows (vedi
                                // autostart.rs, che passa questo argomento
                                // solo lì, mai da un doppio click manuale
                                // sull'icona), la finestra resta nascosta —
                                // l'app è comunque pienamente attiva
                                // (watcher, icona nella tray), richiamabile
                                // in ogni momento dalla tray stessa.
                                let avvio_minimizzato =
                                    std::env::args().any(|a| a == "--minimized");
                                if !avvio_minimizzato {
                                    let _ = window.show();
                                }
                                // Windows stesso riposiziona la finestra da solo
                                // circa 1-2s dopo la creazione (osservato: succede
                                // anche SENZA nessuna posizione esplicita nostra,
                                // quindi non è un effetto del ripristino sopra —
                                // sembra un "ricorda l'ultima posizione per
                                // quest'app" del sistema stesso, probabilmente
                                // accumulato da tutti gli avvii di sviluppo di
                                // questa sessione). Riafferma la posizione/
                                // dimensione salvata un attimo più tardi, così ha
                                // sempre l'ultima parola invece di essere
                                // silenziosamente sovrascritta.
                                if let Some(stato) = stato_salvato {
                                    let window_per_riaffermazione = window.clone();
                                    std::thread::spawn(move || {
                                        std::thread::sleep(std::time::Duration::from_millis(2000));
                                        let _ = window_per_riaffermazione
                                            .set_position(tauri::PhysicalPosition::new(stato.x, stato.y));
                                        let _ = window_per_riaffermazione.set_size(
                                            tauri::PhysicalSize::new(stato.width, stato.height),
                                        );
                                    });
                                }
                                let window_clone = window.clone();
                                // Debounce con un "numero di turno" condiviso:
                                // ogni Moved/Resized incrementa il contatore e
                                // avvia un salvataggio ritardato di 600ms — se
                                // nel frattempo arriva un altro evento (es. si
                                // sta ancora trascinando/ridimensionando), il
                                // contatore è già avanzato quando il salvataggio
                                // ritardato precedente si sveglia, che allora
                                // rinuncia senza scrivere. Solo l'ULTIMO evento
                                // di una sequenza arriva davvero a scrivere su
                                // disco, invece di un file scritto decine di
                                // volte al secondo durante un trascinamento.
                                let turno_salvataggio =
                                    std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
                                let app_data_dir_eventi = app_data_dir_finestra.clone();
                                window.on_window_event(move |event| match event {
                                    tauri::WindowEvent::CloseRequested { api, .. } => {
                                        api.prevent_close();
                                        let _ = window_clone.hide();
                                    }
                                    tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_) => {
                                        let turno = turno_salvataggio
                                            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                                            + 1;
                                        let window_per_salvataggio = window_clone.clone();
                                        let app_data_dir_per_salvataggio =
                                            app_data_dir_eventi.clone();
                                        let turno_salvataggio2 = turno_salvataggio.clone();
                                        std::thread::spawn(move || {
                                            std::thread::sleep(std::time::Duration::from_millis(
                                                600,
                                            ));
                                            if turno_salvataggio2
                                                .load(std::sync::atomic::Ordering::SeqCst)
                                                != turno
                                            {
                                                return;
                                            }
                                            if let (Ok(pos), Ok(size)) = (
                                                window_per_salvataggio.outer_position(),
                                                window_per_salvataggio.inner_size(),
                                            ) {
                                                window_state::salva(
                                                    &app_data_dir_per_salvataggio,
                                                    &window_state::StatoFinestra {
                                                        x: pos.x,
                                                        y: pos.y,
                                                        width: size.width,
                                                        height: size.height,
                                                    },
                                                );
                                            }
                                        });
                                    }
                                    _ => {}
                                });
                                attach_navigation_retry(&window);
                            }
                            Err(e) => log::error!("Impossibile creare la finestra principale: {e}"),
                        }
                    });

                    // aw-watcher-app-icons per primo (se acceso in
                    // config): non manda heartbeat/eventi al datastore
                    // (scrive direttamente file — icone/colori/nomi —
                    // nella cartella scrivibile), ma riceve in ingresso,
                    // sulla propria pipe stdin, i nomi delle app scoperte
                    // dal watcher finestra (vedi spawn_stdout_drain).
                    if *modules_config.get("aw-watcher-app-icons").unwrap_or(&true) {
                        spawn_app_icons(&app_handle, &app_data_dir, &icons_stdin);
                    }

                    for name in WATCHERS {
                        if *modules_config.get(*name).unwrap_or(&true) {
                            spawn_watcher(
                                &app_handle,
                                name,
                                server.clone(),
                                hostname.clone(),
                                &app_data_dir,
                                icons_stdin.clone(),
                            );
                        }
                    }

                    // Watcher personalizzati (scritti dall'utente, non
                    // compilati come sidecar) — scoperti sotto
                    // app_data_dir/custom/watchers/, avviati dopo tutti i
                    // watcher integrati. Vedi custom_watchers.rs.
                    {
                        let tracked = app_handle.state::<custom_watchers::CustomWatcherProcesses>();
                        custom_watchers::spawn_custom_watchers(
                            server.clone(),
                            hostname.clone(),
                            &app_data_dir,
                            &tracked,
                        );
                    }

                    // Pulizia giornaliera dei log dei watcher personalizzati
                    // (richiesta esplicita: righe più vecchie di 7 giorni
                    // eliminate, un controllo al giorno). Vedi
                    // custom_watchers::avvia_pulizia_log_periodica.
                    custom_watchers::avvia_pulizia_log_periodica(app_data_dir.clone());

                    // VoiSpeed: riprende da solo il ciclo di aggiornamento
                    // se risultava già collegato prima di questo avvio
                    // (identità persistita — vedi sopra) E il modulo non è
                    // stato spento dal menu Moduli (stessa condizione già
                    // applicata a tutti gli altri watcher sopra). Un primo
                    // collegamento invece parte solo dal pulsante "Collega
                    // VoiSpeed" nelle Impostazioni (comando
                    // voispeed_connect).
                    if *modules_config.get("aw-watcher-voispeed").unwrap_or(&true)
                        && voispeed::load_identity(&app_data_dir).is_some()
                    {
                        voispeed::avvia_polling(app_handle.clone(), server.clone(), hostname.clone());
                    }

                    // Notifiche VPN "cliente non mappato": parte sempre
                    // (il controllo periodico stesso verifica ad ogni
                    // giro se il watcher VPN è acceso, vedi
                    // vpn_notify.rs) — niente da condizionare qui.
                    vpn_notify::avvia_controllo(app_handle.clone(), server.clone());
                });
            }

            // Tray icon: Mostra/Nascondi finestra, sottomenu Moduli
            // (accendi/spegni ogni watcher, con la spunta accanto a
            // quelli attivi — la scelta si salva subito e resta valida
            // anche dopo un riavvio, vedi load/save_modules_config), Esci
            // (termina anche tutti i processi sidecar, non solo la
            // finestra).
            let show_hide = MenuItem::with_id(app, "show_hide", "Mostra/Nascondi", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Esci", true, None::<&str>)?;

            let mut module_items: HashMap<String, CheckMenuItem<tauri::Wry>> = HashMap::new();
            let mut module_item_list: Vec<CheckMenuItem<tauri::Wry>> = Vec::new();
            for (name, label) in ALL_MODULES {
                let checked = *modules_config.get(*name).unwrap_or(&true);
                let item = CheckMenuItem::with_id(
                    app,
                    format!("module:{name}"),
                    *label,
                    true,
                    checked,
                    None::<&str>,
                )?;
                module_items.insert((*name).to_string(), item.clone());
                module_item_list.push(item);
            }
            let module_item_refs: Vec<&dyn IsMenuItem<tauri::Wry>> =
                module_item_list.iter().map(|i| i as &dyn IsMenuItem<tauri::Wry>).collect();
            let modules_submenu = Submenu::with_id_and_items(app, "modules_submenu", "Moduli", true, &module_item_refs)?;
            app.manage(ModuleMenuItems(std::sync::Mutex::new(module_items)));

            let tray_menu = Menu::with_items(app, &[&show_hide, &modules_submenu, &quit])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&tray_menu)
                .tooltip("TrackFlow")
                .on_menu_event(|app, event| {
                    let id = event.id.as_ref();

                    if let Some(module_name) = id.strip_prefix("module:") {
                        let new_value = {
                            let config_state = app.state::<ModulesConfigState>();
                            let current = *config_state.0.lock().unwrap().get(module_name).unwrap_or(&true);
                            !current
                        };
                        watcher_status::imposta_modulo(app, module_name, new_value);
                        return;
                    }

                    match id {
                        "show_hide" => {
                            if let Some(window) = app.get_webview_window("main") {
                                if window.is_visible().unwrap_or(false) {
                                    let _ = window.hide();
                                } else {
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                    emit_finestra_mostrata(&window);
                                }
                            }
                        }
                        "quit" => {
                            let processes = app.state::<SidecarProcesses>();
                            for (_, child) in processes.0.lock().unwrap().drain() {
                                let _ = child.kill();
                            }
                            // aw-watcher-app-icons non è nella mappa
                            // generica sopra — va terminato a parte.
                            if let Some(icons) = app.try_state::<IconsHandle>() {
                                if let Ok(mut guard) = icons.lock() {
                                    if let Some(child) = guard.take() {
                                        let _ = child.kill();
                                    }
                                }
                            }
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                // Doppio click sull'icona nella tray (system tray) apre
                // l'app — richiesta esplicita dell'utente, prima non
                // c'era nessun handler qui, quindi il doppio click non
                // faceva nulla di definito da noi (comportamento
                // segnalato come "apre a schermo intero" era in realtà
                // lo stato massimizzato residuo dell'ultima sessione).
                // Solo show()+set_focus(), NON tocchiamo lo stato
                // massimizzato/normale della finestra: si apre così
                // com'era rimasta l'ultima volta, non forziamo il
                // fullscreen.
                .on_tray_icon_event(|tray, event| {
                    // Sia il singolo click sinistro (gesto più naturale/
                    // atteso su Windows) sia il doppio click aprono la
                    // finestra — prima gestivamo solo il doppio click,
                    // lasciando il click singolo senza effetto.
                    // `button_state == Up` sul singolo click evita di
                    // scattare due volte (down+up) per lo stesso click.
                    let apri = matches!(
                        event,
                        TrayIconEvent::DoubleClick { button: tauri::tray::MouseButton::Left, .. }
                    ) || matches!(
                        event,
                        TrayIconEvent::Click {
                            button: tauri::tray::MouseButton::Left,
                            button_state: tauri::tray::MouseButtonState::Up,
                            ..
                        }
                    );
                    if apri {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                            emit_finestra_mostrata(&window);
                        }
                    }
                })
                .build(app)?;

            // Nota: la finestra "main" non esiste ancora a questo punto
            // (creata più tardi, in modo asincrono, dopo che il server è
            // pronto — vedi sopra); il suo handler di chiusura (nascondi
            // invece di uscire) viene registrato lì, insieme alla
            // creazione.

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

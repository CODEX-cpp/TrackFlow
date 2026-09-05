//! Esporta/importa la configurazione dell'utente (non i dati tracciati
//! in generale) per portarla da un PC all'altro — richiesta esplicita
//! dopo aver sperimentato di persona, in questa stessa sessione di
//! sviluppo, la scomodità di dover riconfigurare tutto da zero su una
//! seconda macchina (login Claude Desktop, timeout AFK personalizzato,
//! mappatura VPN...).
//!
//! Formato: un unico file JSON — niente zip: le icone, uniche cose
//! binarie coinvolte, sono codificate in base64 dentro lo stesso file,
//! così c'è un solo formato da gestire invece di due. Ogni sezione è
//! indipendente sia in esportazione che in importazione: l'utente
//! sceglie con delle caselle di spunta cosa includere/applicare in
//! entrambe le direzioni (vedi `sezioni_disponibili_in_import`, che
//! dice al frontend quali caselle mostrare per un file che magari non
//! le contiene tutte).
//!
//! Compatibilità tra versioni (richiesta esplicita dell'utente,
//! preoccupato che un file esportato da una versione più nuova rompa
//! qualcosa importato in una più vecchia, o viceversa): ogni campo di
//! ogni sezione è `#[serde(default)]` — una sezione mancante (file più
//! vecchio, funzionalità non ancora esistente quando è stato esportato)
//! diventa semplicemente `None`/vuota, mai un errore di parsing. Un
//! file più NUOVO importato in una versione più vecchia di TrackFlow
//! ignora da solo i campi che non riconosce (comportamento di default
//! di serde: nessun `deny_unknown_fields` da nessuna parte qui).
//! `versione_app` è solo informativa, mostrata all'utente se rileva una
//! versione diversa dalla propria — nessuna migrazione automatica in
//! questa prima versione.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Manager};

use crate::{AppDataDirState, AppServer};

/// Nomi delle sezioni, usati sia dal frontend (caselle di spunta) sia
/// come chiavi in `sezioni_disponibili_in_import`.
pub const SEZIONE_IMPOSTAZIONI: &str = "impostazioni";
pub const SEZIONE_PROGETTI: &str = "progetti";
pub const SEZIONE_MODULI_HOME: &str = "moduli_home";
pub const SEZIONE_ICONE: &str = "icone";
pub const SEZIONE_AFK_FINESTRA: &str = "afk_finestra";

const NOME_FILE_ICONE_LEGGIBILI: &str = "appAutoNames.json";
const NOME_FILE_ICONE_COLORI: &str = "appIconColors.json";
const NOME_CARTELLA_ICONE: &str = "app-icons";

/// Impostazioni escluse dalla sezione "impostazioni" perché specifiche
/// di QUESTO PC (non ha senso portarle su un altro) o bookkeeping
/// interno usa-e-getta — `projects`/`views` escluse anche loro, ma
/// perché diventano sezioni proprie (vedi SEZIONE_PROGETTI/MODULI_HOME).
const CHIAVI_IMPOSTAZIONI_ESCLUSE: &[&str] = &[
    "projects",
    "views",
    "developerModeEnabled",
    "diagnosticsLogFolder",
    "autostartDefaultApplied",
    "aiChat.dimensioniPannello",
];

const BUCKET_STOPWATCH: &str = "aw-stopwatch";
const BUCKET_TYPE_STOPWATCH: &str = "general.stopwatch";

// Stessi id/tipo/client fissi usati da aw-watcher-afk-rust — nessun
// suffisso hostname (vedi il commento in quel main.rs), così un import
// su un PC diverso si fonde nello stesso identico bucket invece di
// crearne uno "per macchina" a parte.
const BUCKET_AFK: &str = "aw-watcher-afk";
const BUCKET_TYPE_AFK: &str = "afkstatus";
const CLIENT_AFK: &str = "aw-watcher-afk";

#[derive(Serialize, Deserialize, Default)]
pub struct DatiEsportati {
    pub versione_formato: u32,
    #[serde(default)]
    pub versione_app: String,
    #[serde(default)]
    pub esportato_il: String,

    #[serde(default)]
    pub impostazioni: Option<SezioneImpostazioni>,
    #[serde(default)]
    pub progetti: Option<SezioneProgetti>,
    #[serde(default)]
    pub moduli_home: Option<Value>,
    #[serde(default)]
    pub icone: Option<SezioneIcone>,
    #[serde(default)]
    pub afk_finestra: Option<SezioneAfkFinestra>,
}

/// Versione del FORMATO (non della versione di TrackFlow) — da
/// incrementare SOLO se una modifica futura rompe la compatibilità con
/// file già esportati (mai per una nuova sezione opzionale in più, che
/// da sola non rompe nulla grazie a `#[serde(default)]` ovunque).
const VERSIONE_FORMATO: u32 = 1;

#[derive(Serialize, Deserialize, Default)]
pub struct SezioneImpostazioni {
    #[serde(default)]
    pub chiavi: HashMap<String, Value>,
    #[serde(default)]
    pub moduli_watcher_config: Option<Value>,
    #[serde(default)]
    pub vpn_mapping_manuale: Vec<VoceVpnEsportata>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct VoceVpnEsportata {
    pub indirizzo: String,
    pub cliente: String,
}

#[derive(Serialize, Deserialize, Default)]
pub struct SezioneProgetti {
    #[serde(default)]
    pub progetti: Option<Value>,
    /// Eventi grezzi del bucket aw-stopwatch (timestamp/durata/dati) —
    /// SENZA l'id originale (riassegnato dal datastore di destinazione,
    /// un id di un altro database non significherebbe nulla qui).
    #[serde(default)]
    pub eventi_cronometro: Vec<Value>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct SezioneIcone {
    #[serde(default)]
    pub nomi_leggibili: Option<Value>,
    #[serde(default)]
    pub colori: Option<Value>,
    /// Nome file (es. "excel.exe.png") -> contenuto PNG in base64.
    #[serde(default)]
    pub file: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct SezioneAfkFinestra {
    #[serde(default)]
    pub aw_watcher_afk_toml: Option<String>,
    #[serde(default)]
    pub aw_watcher_window_toml: Option<String>,
    /// Eventi grezzi del bucket aw-watcher-afk (timestamp/durata/dati:
    /// "afk"/"not-afk") — richiesta esplicita dell'utente: questa sezione
    /// esportava solo la configurazione (timeout/poll_time personalizzati,
    /// quasi mai toccati), non la vera cronologia di quando e quanto sei
    /// stato assente, che è il dato che serve davvero per ripristinarla
    /// correttamente su un altro PC. Non gli eventi di "Finestra attiva"
    /// (currentwindow): molto più numerosi (poll_time 1s) e già inclusi
    /// per intero nella sezione "Eventi tracciati" separata, che resta il
    /// posto giusto per quelli.
    #[serde(default)]
    pub eventi_afk: Vec<Value>,
}

impl AppServer {
    /// Tutte le impostazioni in un colpo solo (`/api/0/settings`, senza
    /// chiave — vedi `settings_get` in aw-server-rust-src) invece di una
    /// richiesta per chiave: l'export ne legge una ventina, sarebbe
    /// stato lento e inutilmente complicato farne una alla volta.
    pub(crate) async fn get_all_settings(&self) -> HashMap<String, Value> {
        let local_resp = self.client.get("/api/0/settings").header(Self::host_header()).dispatch().await;
        if local_resp.status().code != 200 {
            return HashMap::new();
        }
        let bytes = local_resp.into_bytes().await.unwrap_or_default();
        serde_json::from_slice(&bytes).unwrap_or_default()
    }

    /// Crea il bucket SENZA passare dalla cache `known_buckets` di
    /// `ensure_bucket` — bug reale scoperto proprio testando lo scenario
    /// "esporta, elimina il bucket, reimporta" con un test isolato:
    /// quella cache, una volta che segna un bucket come "già creato" in
    /// questa sessione, non viene MAI invalidata se il bucket viene
    /// cancellato nel frattempo (es. dal pulsante "Elimina bucket" di
    /// Watchers) — `ensure_bucket` salterebbe quindi la vera POST di
    /// creazione, lasciando il bucket assente e l'import successivo
    /// rifiutato con "non esiste su questo dispositivo". La creazione è
    /// comunque idempotente lato server (bucket già esistente = no-op),
    /// quindi bypassare la cache qui non ha altri effetti collaterali.
    async fn crea_bucket_senza_cache(&self, bucket_id: &str, bucket_type: &str, client_name: &str, hostname: &str) {
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
    }

    /// Passa dallo stesso endpoint `/api/0/import` usato dal pulsante
    /// "Importa bucket" di Watchers, invece di un `insert_event` grezzo
    /// per ogni evento — quell'endpoint ha già un dedup per evento
    /// (timestamp+durata+dati, vedi `import()` in
    /// aw-server-rust-src/aw-server/src/endpoints/import.rs) mentre
    /// `insert_event` crea sempre una riga nuova. Senza questo, importare
    /// due volte lo stesso file di export (o esportare, cancellare un
    /// progetto e reimportarlo) duplicherebbe ogni sessione del
    /// cronometro ad ogni giro — lo stesso genere di bug già risolto per
    /// gli eventi "grezzi" tramite `event_identity`, qui esteso alla
    /// sezione progetti. Il bucket target deve già esistere sotto lo
    /// stesso id esatto (vedi `ensure_bucket` nel chiamante) — l'endpoint
    /// rifiuta un import verso un bucket sconosciuto.
    pub(crate) async fn import_bucket_events(&self, corpo: &str) -> Result<(), String> {
        let local_resp = self
            .client
            .post("/api/0/import")
            .header(rocket::http::ContentType::JSON)
            .header(Self::host_header())
            .body(corpo.to_string())
            .dispatch()
            .await;
        let status = local_resp.status().code;
        if !(200..300).contains(&status) {
            let bytes = local_resp.into_bytes().await.unwrap_or_default();
            return Err(format!(
                "import eventi fallito (status {status}): {}",
                String::from_utf8_lossy(&bytes)
            ));
        }
        Ok(())
    }

    /// Cancella per davvero una chiave di impostazione (DELETE
    /// `/api/0/settings/<key>`) — usata solo dal test distruttivo qui
    /// sotto per simulare "elimino i dati e reimporto", non da nessun
    /// percorso di export/import vero e proprio (che sovrascrive sempre,
    /// non cancella mai nulla).
    #[cfg(test)]
    async fn delete_setting_di_test(&self, key: &str) {
        let _ = self.client.delete(format!("/api/0/settings/{key}")).header(Self::host_header()).dispatch().await;
    }
}

fn leggi_json_file(percorso: &Path) -> Option<Value> {
    let contenuto = std::fs::read_to_string(percorso).ok()?;
    serde_json::from_str(&contenuto).ok()
}

async fn costruisci_sezione_impostazioni(
    server: &AppServer,
    app_handle: &AppHandle,
    app_data_dir: &Path,
) -> SezioneImpostazioni {
    let mut chiavi = server.get_all_settings().await;
    for esclusa in CHIAVI_IMPOSTAZIONI_ESCLUSE {
        chiavi.remove(*esclusa);
    }

    let moduli_watcher_config = leggi_json_file(&app_data_dir.join("modules-config.json"));

    let vpn_mapping_manuale = crate::vpn_mapping::leggi_mapping_vpn(app_handle.clone())
        .unwrap_or_default()
        .into_iter()
        .filter(|v| v.origine == "manuale")
        .map(|v| VoceVpnEsportata { indirizzo: v.indirizzo, cliente: v.cliente })
        .collect();

    SezioneImpostazioni { chiavi, moduli_watcher_config, vpn_mapping_manuale }
}

/// Interroga TUTTI gli eventi mai registrati in un bucket, stesso motore
/// AQL usato ovunque nell'app — nessun limite temporale stretto, sia il
/// cronometro progetti sia la cronologia AFK possono avere dati vecchi di
/// mesi che vanno comunque portati con sé.
async fn leggi_tutti_gli_eventi(server: &AppServer, bucket_id: &str) -> Vec<Value> {
    let query_lines =
        vec![format!("events = query_bucket(\"{bucket_id}\");"), "RETURN = events;".to_string()];
    let timeperiod = "2000-01-01T00:00:00+00:00/2100-01-01T00:00:00+00:00".to_string();
    let Ok(risposta) = server.query(vec![timeperiod], query_lines).await else {
        return Vec::new();
    };
    risposta
        .as_array()
        .and_then(|periodi| periodi.first())
        .and_then(|eventi| eventi.as_array())
        .map(|eventi| {
            eventi
                .iter()
                .map(|e| {
                    // Niente "id" (riassegnato dal datastore di
                    // destinazione), solo timestamp/duration/data.
                    serde_json::json!({
                        "timestamp": e["timestamp"],
                        "duration": e["duration"],
                        "data": e["data"],
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

async fn costruisci_sezione_progetti(server: &AppServer) -> SezioneProgetti {
    SezioneProgetti {
        progetti: server.get_setting("projects").await,
        eventi_cronometro: leggi_tutti_gli_eventi(server, BUCKET_STOPWATCH).await,
    }
}

fn costruisci_sezione_icone(app_data_dir: &Path) -> SezioneIcone {
    use base64::Engine;

    let nomi_leggibili = leggi_json_file(&app_data_dir.join(NOME_FILE_ICONE_LEGGIBILI));
    let colori = leggi_json_file(&app_data_dir.join(NOME_FILE_ICONE_COLORI));

    let mut file = HashMap::new();
    if let Ok(voci) = std::fs::read_dir(app_data_dir.join(NOME_CARTELLA_ICONE)) {
        for voce in voci.flatten() {
            let percorso = voce.path();
            if percorso.extension().and_then(|e| e.to_str()) != Some("png") {
                continue;
            }
            let Some(nome_file) = percorso.file_name().and_then(|n| n.to_str()) else { continue };
            let Ok(bytes) = std::fs::read(&percorso) else { continue };
            file.insert(nome_file.to_string(), base64::engine::general_purpose::STANDARD.encode(bytes));
        }
    }

    SezioneIcone { nomi_leggibili, colori, file }
}

async fn costruisci_sezione_afk_finestra(server: &AppServer, app_data_dir: &Path) -> SezioneAfkFinestra {
    SezioneAfkFinestra {
        aw_watcher_afk_toml: std::fs::read_to_string(app_data_dir.join("aw-watcher-afk.toml")).ok(),
        aw_watcher_window_toml: std::fs::read_to_string(app_data_dir.join("aw-watcher-window.toml")).ok(),
        eventi_afk: leggi_tutti_gli_eventi(server, BUCKET_AFK).await,
    }
}

/// Costruisce il file di export completo — solo le sezioni richieste
/// vengono popolate, le altre restano `None` (mai calcolate, così
/// spuntare solo "Progetti" non fa comunque leggere icone/impostazioni
/// inutilmente). Ritorna il JSON già pronto per essere scritto su disco
/// dal frontend (stesso `downloadFile()` già usato per CSV/altri
/// export, vedi util/export.ts) — nessun dialogo di salvataggio qui,
/// resta un problema del frontend come per tutto il resto dell'app.
#[tauri::command]
pub async fn esporta_dati(app_handle: AppHandle, sezioni: Vec<String>) -> Result<String, String> {
    let dir = app_handle
        .try_state::<AppDataDirState>()
        .ok_or_else(|| "Cartella dati non ancora pronta, riprova tra poco".to_string())?
        .0
        .clone();
    let server = app_handle
        .try_state::<Arc<AppServer>>()
        .ok_or_else(|| "Server non ancora pronto, riprova tra poco".to_string())?
        .inner()
        .clone();

    let mut dati = DatiEsportati {
        versione_formato: VERSIONE_FORMATO,
        versione_app: env!("CARGO_PKG_VERSION").to_string(),
        esportato_il: chrono::Utc::now().to_rfc3339(),
        ..Default::default()
    };

    if sezioni.iter().any(|s| s == SEZIONE_IMPOSTAZIONI) {
        dati.impostazioni = Some(costruisci_sezione_impostazioni(&server, &app_handle, &dir).await);
    }
    if sezioni.iter().any(|s| s == SEZIONE_PROGETTI) {
        dati.progetti = Some(costruisci_sezione_progetti(&server).await);
    }
    if sezioni.iter().any(|s| s == SEZIONE_MODULI_HOME) {
        dati.moduli_home = server.get_setting("views").await;
    }
    if sezioni.iter().any(|s| s == SEZIONE_ICONE) {
        dati.icone = Some(costruisci_sezione_icone(&dir));
    }
    if sezioni.iter().any(|s| s == SEZIONE_AFK_FINESTRA) {
        dati.afk_finestra = Some(costruisci_sezione_afk_finestra(&server, &dir).await);
    }

    serde_json::to_string_pretty(&dati).map_err(|e| e.to_string())
}

/// Legge SOLO l'intestazione di un file di export già scelto
/// dall'utente (dialogo di apertura file, gestito dal frontend) per
/// dirgli quali caselle di spunta mostrare nella schermata di import —
/// un file esportato da una versione più vecchia di TrackFlow, prima
/// che esistesse una certa sezione, non deve proporre una casella per
/// una sezione che semplicemente non c'è.
#[derive(Serialize)]
pub struct InfoImport {
    pub versione_app_origine: String,
    pub sezioni_disponibili: Vec<String>,
}

#[tauri::command]
pub fn sezioni_disponibili_in_import(contenuto: String) -> Result<InfoImport, String> {
    let dati: DatiEsportati = serde_json::from_str(&contenuto).map_err(|e| format!("File non valido: {e}"))?;
    let mut sezioni_disponibili = Vec::new();
    if dati.impostazioni.is_some() {
        sezioni_disponibili.push(SEZIONE_IMPOSTAZIONI.to_string());
    }
    if dati.progetti.is_some() {
        sezioni_disponibili.push(SEZIONE_PROGETTI.to_string());
    }
    if dati.moduli_home.is_some() {
        sezioni_disponibili.push(SEZIONE_MODULI_HOME.to_string());
    }
    if dati.icone.is_some() {
        sezioni_disponibili.push(SEZIONE_ICONE.to_string());
    }
    if dati.afk_finestra.is_some() {
        sezioni_disponibili.push(SEZIONE_AFK_FINESTRA.to_string());
    }
    Ok(InfoImport { versione_app_origine: dati.versione_app, sezioni_disponibili })
}

async fn applica_sezione_impostazioni(server: &AppServer, app_handle: &AppHandle, sezione: &SezioneImpostazioni) {
    for (chiave, valore) in &sezione.chiavi {
        let _ = server.set_setting(chiave, valore).await;
    }
    if let Some(config) = &sezione.moduli_watcher_config {
        if let Some(dir) = app_handle.try_state::<AppDataDirState>() {
            if let Ok(testo) = serde_json::to_string_pretty(config) {
                let _ = std::fs::write(dir.0.join("modules-config.json"), testo);
            }
        }
    }
    if !sezione.vpn_mapping_manuale.is_empty() {
        let voci = sezione
            .vpn_mapping_manuale
            .iter()
            .cloned()
            .map(|v| crate::vpn_mapping::VoceMappingVpnInput { indirizzo: v.indirizzo, cliente: v.cliente })
            .collect();
        let _ = crate::vpn_mapping::salva_mapping_vpn_manuale(app_handle.clone(), voci);
    }
}

async fn applica_sezione_progetti(server: &AppServer, sezione: &SezioneProgetti, hostname: &str) {
    if let Some(progetti) = &sezione.progetti {
        let _ = server.set_setting("projects", progetti).await;
    }
    if !sezione.eventi_cronometro.is_empty() {
        // Crea il bucket PRIMA con questo id esatto, così l'import sotto
        // lo trova per corrispondenza diretta invece di dover risolvere
        // per "client" (che con client "unknown" potrebbe non essere
        // univoco) — vedi il commento su import_bucket_events sopra.
        // `crea_bucket_senza_cache`, non `ensure_bucket`: vedi il suo
        // commento, la cache di quest'ultimo può restare convinta che il
        // bucket esista anche dopo che è stato eliminato nel frattempo.
        server.crea_bucket_senza_cache(BUCKET_STOPWATCH, BUCKET_TYPE_STOPWATCH, "unknown", hostname).await;
        let corpo = serde_json::json!({
            "buckets": {
                BUCKET_STOPWATCH: {
                    "id": BUCKET_STOPWATCH,
                    "type": BUCKET_TYPE_STOPWATCH,
                    "client": "unknown",
                    "hostname": hostname,
                    "created": Value::Null,
                    "events": sezione.eventi_cronometro,
                }
            }
        });
        if let Err(e) = server.import_bucket_events(&corpo.to_string()).await {
            log::warn!("Import eventi cronometro progetti fallito: {e}");
        }
    }
}

fn applica_sezione_icone(app_data_dir: &Path, sezione: &SezioneIcone) {
    use base64::Engine;

    if let Some(nomi) = &sezione.nomi_leggibili {
        if let Ok(testo) = serde_json::to_string_pretty(nomi) {
            let _ = std::fs::write(app_data_dir.join(NOME_FILE_ICONE_LEGGIBILI), testo);
        }
    }
    if let Some(colori) = &sezione.colori {
        if let Ok(testo) = serde_json::to_string_pretty(colori) {
            let _ = std::fs::write(app_data_dir.join(NOME_FILE_ICONE_COLORI), testo);
        }
    }
    if !sezione.file.is_empty() {
        let cartella = app_data_dir.join(NOME_CARTELLA_ICONE);
        let _ = std::fs::create_dir_all(&cartella);
        for (nome_file, contenuto_b64) in &sezione.file {
            // Percorso ricostruito solo da un nome file (mai da un
            // valore col separatore di cartella) — stessa cautela già
            // vista altrove nell'app per input che finiscono in un
            // percorso su disco.
            if nome_file.contains('/') || nome_file.contains('\\') || nome_file.contains("..") {
                continue;
            }
            if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(contenuto_b64) {
                let _ = std::fs::write(cartella.join(nome_file), bytes);
            }
        }
    }
}

async fn applica_sezione_afk_finestra(server: &AppServer, app_data_dir: &Path, sezione: &SezioneAfkFinestra, hostname: &str) {
    if let Some(toml) = &sezione.aw_watcher_afk_toml {
        let _ = std::fs::write(app_data_dir.join("aw-watcher-afk.toml"), toml);
    }
    if let Some(toml) = &sezione.aw_watcher_window_toml {
        let _ = std::fs::write(app_data_dir.join("aw-watcher-window.toml"), toml);
    }
    if !sezione.eventi_afk.is_empty() {
        // Stesso schema (e stesso motivo) del dedup già applicato alla
        // sezione progetti: crea il bucket senza passare dalla cache
        // (può essere stato eliminato nel frattempo) e importa gli
        // eventi dall'endpoint con deduplica, non con insert_event
        // grezzo — altrimenti reimportare lo stesso file duplicherebbe
        // ogni intervallo di assenza ad ogni giro.
        server.crea_bucket_senza_cache(BUCKET_AFK, BUCKET_TYPE_AFK, CLIENT_AFK, hostname).await;
        let corpo = serde_json::json!({
            "buckets": {
                BUCKET_AFK: {
                    "id": BUCKET_AFK,
                    "type": BUCKET_TYPE_AFK,
                    "client": CLIENT_AFK,
                    "hostname": hostname,
                    "created": Value::Null,
                    "events": sezione.eventi_afk,
                }
            }
        });
        if let Err(e) = server.import_bucket_events(&corpo.to_string()).await {
            log::warn!("Import eventi AFK fallito: {e}");
        }
    }
}

/// Applica solo le sezioni scelte dall'utente nella schermata di
/// import — un file può contenere più sezioni di quelle selezionate
/// (l'utente ha deselezionato qualcosa apposta), quelle vengono
/// semplicemente ignorate.
#[tauri::command]
pub async fn importa_dati(app_handle: AppHandle, contenuto: String, sezioni: Vec<String>) -> Result<(), String> {
    let dati: DatiEsportati = serde_json::from_str(&contenuto).map_err(|e| format!("File non valido: {e}"))?;

    let dir = app_handle
        .try_state::<AppDataDirState>()
        .ok_or_else(|| "Cartella dati non ancora pronta, riprova tra poco".to_string())?
        .0
        .clone();
    let server = app_handle
        .try_state::<Arc<AppServer>>()
        .ok_or_else(|| "Server non ancora pronto, riprova tra poco".to_string())?
        .inner()
        .clone();
    let hostname = gethostname::gethostname().to_string_lossy().to_string();

    if sezioni.iter().any(|s| s == SEZIONE_IMPOSTAZIONI) {
        if let Some(sezione) = &dati.impostazioni {
            applica_sezione_impostazioni(&server, &app_handle, sezione).await;
        }
    }
    if sezioni.iter().any(|s| s == SEZIONE_PROGETTI) {
        if let Some(sezione) = &dati.progetti {
            applica_sezione_progetti(&server, sezione, &hostname).await;
        }
    }
    if sezioni.iter().any(|s| s == SEZIONE_MODULI_HOME) {
        if let Some(views) = &dati.moduli_home {
            let _ = server.set_setting("views", views).await;
        }
    }
    if sezioni.iter().any(|s| s == SEZIONE_ICONE) {
        if let Some(sezione) = &dati.icone {
            applica_sezione_icone(&dir, sezione);
        }
    }
    if sezioni.iter().any(|s| s == SEZIONE_AFK_FINESTRA) {
        if let Some(sezione) = &dati.afk_finestra {
            applica_sezione_afk_finestra(&server, &dir, sezione, &hostname).await;
        }
    }

    Ok(())
}

#[cfg(test)]
mod test_manuale {
    //! Test manuale, NON eseguito da un normale `cargo test` (`#[ignore]`)
    //! — verifica dal vivo contro il database e la cartella dati REALI di
    //! questo PC i pezzi che non hanno bisogno di un vero `AppHandle`
    //! Tauri (che non si può costruire fuori da un'app in esecuzione):
    //! lettura di tutte le impostazioni, eventi del cronometro progetti,
    //! icone, config AFK/finestra, e il giro completo di serializzazione/
    //! deserializzazione del formato di export. La sola parte NON
    //! coperta qui è la mappatura VPN (`leggi_mapping_vpn`/
    //! `salva_mapping_vpn_manuale`), che ha bisogno di un AppHandle solo
    //! per risolvere la cartella dati — plumbing sottile, stesso schema
    //! già collaudato altrove, rischio basso. Va lanciato con `app.exe`
    //! GIÀ CHIUSO (stesso database SQLite, evitare accessi concorrenti).
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    #[ignore]
    async fn test_reale_costruzione_ed_esportazione() {
        let app_data_dir = PathBuf::from(std::env::var("LOCALAPPDATA").unwrap())
            .join("TrackFlow")
            .join("app-data");
        let cartella_neutra = std::env::temp_dir();
        let server = Arc::new(crate::build_app_server(&app_data_dir, &cartella_neutra, &cartella_neutra).await);

        let mut chiavi = server.get_all_settings().await;
        println!("Impostazioni trovate: {} chiavi -> {:?}", chiavi.len(), chiavi.keys().collect::<Vec<_>>());
        assert!(!chiavi.is_empty(), "ci si aspettano impostazioni reali su questo PC");
        for esclusa in CHIAVI_IMPOSTAZIONI_ESCLUSE {
            chiavi.remove(*esclusa);
        }
        assert!(!chiavi.contains_key("projects"), "projects deve restare escluso da 'impostazioni'");
        assert!(!chiavi.contains_key("views"), "views deve restare escluso da 'impostazioni'");

        let eventi_stopwatch = leggi_tutti_gli_eventi(&server, BUCKET_STOPWATCH).await;
        println!("Eventi cronometro progetti trovati: {}", eventi_stopwatch.len());

        let icone = costruisci_sezione_icone(&app_data_dir);
        println!("Icone trovate: {} file, nomi_leggibili={}, colori={}", icone.file.len(), icone.nomi_leggibili.is_some(), icone.colori.is_some());
        assert!(!icone.file.is_empty(), "ci si aspettano icone reali già estratte su questo PC");

        let afk_finestra = costruisci_sezione_afk_finestra(&server, &app_data_dir).await;
        println!(
            "Config AFK/finestra: afk={} window={} eventi_afk={}",
            afk_finestra.aw_watcher_afk_toml.is_some(),
            afk_finestra.aw_watcher_window_toml.is_some(),
            afk_finestra.eventi_afk.len()
        );
        assert!(afk_finestra.aw_watcher_afk_toml.is_some(), "migrato in una sessione precedente, deve esistere");
        assert!(!afk_finestra.eventi_afk.is_empty(), "ci si aspettano eventi AFK reali su questo PC");

        // Giro completo: costruisce il DatiEsportati vero con tutto
        // quanto sopra, lo serializza, lo RIDESERIALIZZA (come farebbe
        // importa_dati con un file scelto dall'utente) e verifica che i
        // dati sopravvivano intatti — la parte più a rischio di un
        // formato di questo genere.
        let dati = DatiEsportati {
            versione_formato: VERSIONE_FORMATO,
            versione_app: env!("CARGO_PKG_VERSION").to_string(),
            esportato_il: chrono::Utc::now().to_rfc3339(),
            impostazioni: Some(SezioneImpostazioni { chiavi, moduli_watcher_config: None, vpn_mapping_manuale: vec![] }),
            progetti: Some(SezioneProgetti { progetti: server.get_setting("projects").await, eventi_cronometro: eventi_stopwatch }),
            moduli_home: server.get_setting("views").await,
            icone: Some(icone),
            afk_finestra: Some(afk_finestra),
        };
        let json = serde_json::to_string_pretty(&dati).unwrap();
        println!("Export completo: {} byte", json.len());

        let riletto: DatiEsportati = serde_json::from_str(&json).expect("il file appena scritto deve rileggersi da solo");
        assert_eq!(riletto.impostazioni.unwrap().chiavi.len(), dati.impostazioni.as_ref().unwrap().chiavi.len());
        assert_eq!(riletto.icone.unwrap().file.len(), dati.icone.as_ref().unwrap().file.len());

        // Compatibilità all'indietro: un JSON che rappresenta un file
        // "vecchio" (prima che esistesse la sezione icone/afk, es.)
        // deve rileggersi comunque, con quei campi semplicemente None —
        // mai un errore di parsing.
        let vecchio_formato = serde_json::json!({
            "versione_formato": 1,
            "versione_app": "0.1.20",
            "esportato_il": "2026-01-01T00:00:00Z",
            "impostazioni": { "chiavi": { "theme": "dark" } },
        });
        let riletto_vecchio: DatiEsportati =
            serde_json::from_value(vecchio_formato).expect("un file più vecchio, con sezioni mancanti, deve rileggersi comunque");
        assert!(riletto_vecchio.icone.is_none());
        assert!(riletto_vecchio.afk_finestra.is_none());
        assert!(riletto_vecchio.progetti.is_none());
        assert_eq!(riletto_vecchio.impostazioni.unwrap().chiavi.get("theme").unwrap(), "dark");

        // Compatibilità in avanti: un JSON con un campo IN PIÙ che
        // questa versione non conosce ancora non deve rompere nulla.
        let formato_futuro = serde_json::json!({
            "versione_formato": 1,
            "versione_app": "9.9.9",
            "esportato_il": "2099-01-01T00:00:00Z",
            "impostazioni": { "chiavi": {} },
            "una_sezione_futura_sconosciuta": { "qualcosa": 123 },
        });
        let riletto_futuro: DatiEsportati =
            serde_json::from_value(formato_futuro).expect("un campo sconosciuto in più non deve rompere l'import");
        assert_eq!(riletto_futuro.versione_app, "9.9.9");

        server.datastore.close();
    }

    /// Verifica il vero timore dell'utente ("non succeda come il vecchio
    /// bug che importava più volte gli stessi dati"), su una cartella dati
    /// e un database ISOLATI e temporanei — MAI quelli reali di questo
    /// PC, proprio perché il test include un giro di "esporta, cancella,
    /// reimporta" che sarebbe distruttivo su dati veri. Copre due cose
    /// insieme:
    /// 1. Import ripetuto dello stesso file (o "esporta -> elimina il
    ///    progetto -> reimporta lo stesso file") non deve duplicare gli
    ///    eventi del cronometro — la sezione "progetti" è l'unica delle
    ///    cinque che contiene una lista (le altre quattro sono coppie
    ///    chiave/valore o file, dove "importare" è sempre un sovrascrivere,
    ///    mai un aggiungere, quindi strutturalmente non possono duplicare).
    /// 2. Esportare e reimportare CIASCUNA sezione singolarmente restituisce
    ///    davvero lo stesso contenuto — non solo che il JSON sopravvive al
    ///    giro di serde (già coperto sopra), ma che i dati veri (bucket,
    ///    file su disco, impostazioni) tornano identici dopo un ciclo
    ///    scrivi->esporta->cancella->importa.
    #[tokio::test]
    #[ignore]
    async fn test_isolato_dedup_e_sezioni_singole() {
        let cartella_test = std::env::temp_dir().join(format!(
            "trackflow-test-export-import-{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        std::fs::create_dir_all(&cartella_test).unwrap();
        let server = Arc::new(crate::build_app_server(&cartella_test, &cartella_test, &cartella_test).await);
        let hostname = "pc-di-test";

        // --- 1. Dedup degli eventi cronometro progetti ---

        // Simula dati "già esistenti" (come se il progetto fosse già stato
        // usato prima che esistesse questa funzionalità di export/import):
        // 3 eventi inseriti direttamente, come fa projectTimerMixin.ts.
        server.ensure_bucket(BUCKET_STOPWATCH, BUCKET_TYPE_STOPWATCH, "unknown", hostname).await;
        for i in 0..3 {
            let evento = serde_json::json!({
                "timestamp": format!("2026-01-0{}T10:00:00+00:00", i + 1),
                "duration": 1800.0,
                "data": { "project": "Test", "label": format!("sessione {i}") },
            });
            server.insert_event(BUCKET_STOPWATCH, &evento).await;
        }
        let eventi_iniziali = leggi_tutti_gli_eventi(&server, BUCKET_STOPWATCH).await;
        assert_eq!(eventi_iniziali.len(), 3, "precondizione: 3 eventi inseriti direttamente");

        // "Esporta" (legge esattamente quei 3 eventi).
        let sezione_progetti = costruisci_sezione_progetti(&server).await;
        assert_eq!(sezione_progetti.eventi_cronometro.len(), 3);

        // Reimporta lo STESSO file esportato — se ci fosse il vecchio bug,
        // ora ci sarebbero 6 eventi invece di 3.
        applica_sezione_progetti(&server, &sezione_progetti, hostname).await;
        let dopo_primo_reimport = leggi_tutti_gli_eventi(&server, BUCKET_STOPWATCH).await;
        assert_eq!(
            dopo_primo_reimport.len(),
            3,
            "reimportare lo stesso export una volta non deve duplicare gli eventi"
        );

        // Lo stesso identico file, importato una SECONDA volta di seguito
        // (scenario esplicito segnalato: "che non succeda come il vecchio
        // bug che importava più volte gli stessi dati").
        applica_sezione_progetti(&server, &sezione_progetti, hostname).await;
        let dopo_secondo_reimport = leggi_tutti_gli_eventi(&server, BUCKET_STOPWATCH).await;
        assert_eq!(
            dopo_secondo_reimport.len(),
            3,
            "reimportare lo stesso export una seconda volta non deve duplicare gli eventi"
        );

        // Scenario "esporta, elimina, reimporta": cancella per davvero il
        // bucket (come fa 'elimina bucket'/'elimina progetto'), poi
        // reimporta il file esportato in precedenza — deve tornare
        // esattamente agli stessi 3 eventi, non 0 e non più di 3.
        server.datastore.delete_bucket(BUCKET_STOPWATCH).expect("cancellazione bucket di test");
        applica_sezione_progetti(&server, &sezione_progetti, hostname).await;
        let dopo_elimina_e_reimporta = leggi_tutti_gli_eventi(&server, BUCKET_STOPWATCH).await;
        assert_eq!(
            dopo_elimina_e_reimporta.len(),
            3,
            "dopo elimina+reimporta ci si aspettano di nuovo esattamente i 3 eventi originali"
        );

        // --- 2. Esportare e reimportare ogni sezione singolarmente ---

        // Sezione icone: crea un file PNG finto + i due json di supporto,
        // esporta, CANCELLA tutto dal disco, reimporta, verifica che sia
        // tornato tutto identico (bytes compresi, tramite il giro base64).
        let cartella_icone = cartella_test.join("app-icons");
        std::fs::create_dir_all(&cartella_icone).unwrap();
        let bytes_finti_png = vec![137u8, 80, 78, 71, 1, 2, 3, 4, 5];
        std::fs::write(cartella_icone.join("finta.exe.png"), &bytes_finti_png).unwrap();
        std::fs::write(
            cartella_test.join("appAutoNames.json"),
            r#"{"finta.exe":"App Finta"}"#,
        )
        .unwrap();
        std::fs::write(cartella_test.join("appIconColors.json"), r##"{"finta.exe":"#ff0000"}"##).unwrap();

        let sezione_icone = costruisci_sezione_icone(&cartella_test);
        assert_eq!(sezione_icone.file.len(), 1);
        assert!(sezione_icone.nomi_leggibili.is_some());
        assert!(sezione_icone.colori.is_some());

        std::fs::remove_dir_all(&cartella_icone).unwrap();
        std::fs::remove_file(cartella_test.join("appAutoNames.json")).unwrap();
        std::fs::remove_file(cartella_test.join("appIconColors.json")).unwrap();
        assert!(!cartella_icone.exists(), "precondizione: icone davvero cancellate prima di reimportare");

        applica_sezione_icone(&cartella_test, &sezione_icone);
        let bytes_riletti = std::fs::read(cartella_icone.join("finta.exe.png")).expect("icona reimportata");
        assert_eq!(bytes_riletti, bytes_finti_png, "i byte dell'icona devono tornare identici dopo il giro base64");
        let nomi_riletti: Value =
            serde_json::from_str(&std::fs::read_to_string(cartella_test.join("appAutoNames.json")).unwrap()).unwrap();
        assert_eq!(nomi_riletti["finta.exe"], "App Finta");

        // Sezione AFK/finestra: file di config (stesso schema di sopra)
        // PIÙ, richiesta esplicita dell'utente, la vera cronologia di
        // assenza (eventi del bucket aw-watcher-afk) — con lo stesso
        // controllo anti-duplicazione già verificato per il cronometro
        // progetti, perché anche questa è una lista, non una coppia
        // chiave/valore.
        std::fs::write(cartella_test.join("aw-watcher-afk.toml"), "timeout = 999\n").unwrap();
        std::fs::write(cartella_test.join("aw-watcher-window.toml"), "poll_time = 7\n").unwrap();
        server.crea_bucket_senza_cache(BUCKET_AFK, BUCKET_TYPE_AFK, CLIENT_AFK, hostname).await;
        for i in 0..2 {
            let evento = serde_json::json!({
                "timestamp": format!("2026-02-0{}T09:00:00+00:00", i + 1),
                "duration": 600.0,
                "data": { "status": "afk" },
            });
            server.insert_event(BUCKET_AFK, &evento).await;
        }
        let sezione_afk = costruisci_sezione_afk_finestra(&server, &cartella_test).await;
        assert_eq!(sezione_afk.aw_watcher_afk_toml.as_deref(), Some("timeout = 999\n"));
        assert_eq!(sezione_afk.eventi_afk.len(), 2, "precondizione: 2 eventi AFK inseriti");

        std::fs::remove_file(cartella_test.join("aw-watcher-afk.toml")).unwrap();
        std::fs::remove_file(cartella_test.join("aw-watcher-window.toml")).unwrap();

        // Reimporta lo stesso export due volte di seguito — stesso timore
        // esplicito dell'utente ("non succeda come il vecchio bug"), qui
        // sulla cronologia AFK invece che sul cronometro progetti.
        applica_sezione_afk_finestra(&server, &cartella_test, &sezione_afk, hostname).await;
        applica_sezione_afk_finestra(&server, &cartella_test, &sezione_afk, hostname).await;
        assert_eq!(
            std::fs::read_to_string(cartella_test.join("aw-watcher-afk.toml")).unwrap(),
            "timeout = 999\n",
            "config AFK deve tornare identica dopo cancella+reimporta"
        );
        let eventi_afk_dopo_reimport = leggi_tutti_gli_eventi(&server, BUCKET_AFK).await;
        assert_eq!(
            eventi_afk_dopo_reimport.len(),
            2,
            "reimportare due volte lo stesso export non deve duplicare gli eventi AFK"
        );

        // Sezione moduli_home (views) e impostazioni generali: chiave/
        // valore, quindi "importare" è sempre un sovrascrivere — nessun
        // rischio di duplicazione strutturale come per gli eventi, ma
        // verifichiamo comunque che il valore torni identico dopo un
        // giro completo scrivi->esporta->sovrascrivi con altro->reimporta.
        let views_originali = serde_json::json!({ "home": ["modulo_a", "modulo_b"] });
        server.set_setting("views", &views_originali).await.unwrap();
        let views_esportate = server.get_setting("views").await.expect("views appena scritte");
        assert_eq!(views_esportate, views_originali);

        server.set_setting("views", &serde_json::json!({ "home": [] })).await.unwrap();
        assert_ne!(server.get_setting("views").await.unwrap(), views_originali);

        server.set_setting("views", &views_esportate).await.unwrap();
        assert_eq!(
            server.get_setting("views").await.unwrap(),
            views_originali,
            "moduli Home devono tornare identici dopo il reimport"
        );

        server.set_setting("theme", &serde_json::json!("dark")).await.unwrap();
        let mut chiavi_esportate = server.get_all_settings().await;
        assert_eq!(chiavi_esportate.get("theme").unwrap(), "dark");
        chiavi_esportate.remove("views"); // non fa parte della sezione "impostazioni", è la sua sezione propria
        server.set_setting("theme", &serde_json::json!("light")).await.unwrap();
        for (chiave, valore) in &chiavi_esportate {
            server.set_setting(chiave, valore).await.unwrap();
        }
        assert_eq!(
            server.get_setting("theme").await.unwrap(),
            "dark",
            "impostazioni generali devono tornare identiche dopo il reimport"
        );

        server.datastore.close();
        let _ = std::fs::remove_dir_all(&cartella_test);
    }

    /// Scenario esplicitamente richiesto: modifica e salva la
    /// disposizione dei moduli Home, "formatta tutto" (database
    /// azzerato, come un'installazione pulita), poi ripristina SOLO la
    /// sezione "moduli_home" — verifica che la disposizione torni
    /// esattamente quella salvata, senza che nessun'altra impostazione
    /// riappaia insieme (la sezione non selezionata non deve
    /// "trascinarsi dietro" nulla). Cartella/database isolati e
    /// temporanei, come gli altri test qui sopra.
    #[tokio::test]
    #[ignore]
    async fn test_isolato_ripristino_solo_moduli_home() {
        let cartella_test = std::env::temp_dir().join(format!(
            "trackflow-test-moduli-home-{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        std::fs::create_dir_all(&cartella_test).unwrap();
        let mut server = Arc::new(crate::build_app_server(&cartella_test, &cartella_test, &cartella_test).await);

        // Disposizione realistica (stessa forma della vera impostazione
        // "views" letta dal database reale di questo PC), poi MODIFICATA
        // a mano come farebbe l'utente trascinando i moduli nella Home:
        // "top_apps" spostato in fondo, "workflow_grid" ridimensionato da
        // 3 a 2, e un nuovo modulo aggiunto in cima — così il test
        // verifica un vero cambiamento, non solo che un valore statico
        // sopravviva al giro.
        let disposizione_modificata = serde_json::json!([{
            "id": "summary",
            "name": "Summary",
            "elements": [
                { "id": "nuovo-1111-2222-3333-444455556666", "size": 3, "type": "top_editor_files" },
                { "id": "6213cc7f-f5c2-4ba8-a3c2-e39ed8b338bc", "size": 3, "type": "top_titles" },
                { "id": "da9539cc-2510-4d97-bfc6-8cb7452bc44c", "size": 3, "type": "top_claude_usage" },
                { "id": "a4515a7e-e499-4d58-8b6d-872389cf34eb", "size": 2, "type": "workflow_grid", "props": {} },
                { "id": "24a55517-a574-4565-ba05-42b1e4e83191", "size": 3, "type": "top_vpn_clients" },
                {
                    "id": "28af6631-7a6e-4b7c-b23e-32d9dc0c298d",
                    "type": "custom_watcher_view",
                    "props": { "bucketId": "custom-watcher-rrest", "gridWidth": 2, "templateId": "valore-numero", "title": "rrest" }
                },
                { "id": "48be515e-99ae-4bcc-97c2-8801c0446205", "size": 3, "type": "top_apps" }
            ]
        }]);
        server.set_setting("views", &disposizione_modificata).await.unwrap();

        // Altre impostazioni presenti insieme, come su un PC vero — per
        // verificare dopo che "ripristina solo moduli Home" non le
        // riporta anche loro.
        server.set_setting("theme", &serde_json::json!("dark")).await.unwrap();
        server.set_setting("locale", &serde_json::json!("it")).await.unwrap();
        server
            .set_setting("projects", &serde_json::json!([{ "name": "Progetto Test", "budget": 100 }]))
            .await
            .unwrap();

        // "Esporta" la sola sezione moduli Home — stesso identico valore
        // che esporta_dati scriverebbe nel file.
        let moduli_home_esportati = server.get_setting("views").await.expect("views appena scritte");
        assert_eq!(moduli_home_esportati, disposizione_modificata);

        // "Formatta tutto": chiude il datastore e cancella il file del
        // database, poi ricostruisce il server sulla stessa cartella —
        // equivalente a un'installazione pulita, nessuna impostazione
        // sopravvive.
        server.datastore.close();
        drop(server);
        // Anche i file WAL/SHM del checkpoint, non solo il .db principale
        // — altrimenti SQLite potrebbe ripristinare da lì dei dati non
        // ancora scritti nel file base, vanificando il "formatta tutto".
        for suffisso in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(cartella_test.join(format!("sqlite.db{suffisso}")));
        }
        server = Arc::new(crate::build_app_server(&cartella_test, &cartella_test, &cartella_test).await);

        assert!(server.get_setting("views").await.is_none(), "dopo il formatta tutto non deve restare nulla");
        assert!(server.get_setting("theme").await.is_none(), "precondizione: formattato per davvero");
        assert!(server.get_setting("projects").await.is_none(), "precondizione: formattato per davvero");

        // Ripristina SOLO la sezione moduli Home, come farebbe
        // importa_dati con solo quella casella spuntata.
        server.set_setting("views", &moduli_home_esportati).await.unwrap();

        let disposizione_ripristinata = server.get_setting("views").await.expect("views deve essere tornata");
        assert_eq!(
            disposizione_ripristinata, disposizione_modificata,
            "la disposizione dei moduli Home deve tornare ESATTAMENTE quella modificata prima del formatta tutto"
        );
        // Verifica elemento per elemento, non solo l'uguaglianza globale —
        // così se qualcosa non torna si vede subito COSA, non solo che
        // "qualcosa" è diverso.
        let elementi = disposizione_ripristinata[0]["elements"].as_array().unwrap();
        assert_eq!(elementi.len(), 7, "numero di moduli in Home");
        assert_eq!(elementi[0]["type"], "top_editor_files", "il modulo spostato in cima deve restare in cima");
        assert_eq!(elementi[3]["type"], "workflow_grid");
        assert_eq!(elementi[3]["size"], 2, "il ridimensionamento del modulo deve sopravvivere");
        assert_eq!(elementi[6]["type"], "top_apps", "il modulo spostato in fondo deve restare in fondo");

        // Le altre impostazioni NON devono essere riapparse — solo
        // "moduli_home" è stata ripristinata, nessun'altra sezione.
        assert!(server.get_setting("theme").await.is_none(), "il tema non fa parte della sezione moduli_home");
        assert!(server.get_setting("projects").await.is_none(), "i progetti non fanno parte della sezione moduli_home");

        println!(
            "Disposizione Home ripristinata correttamente: {}",
            serde_json::to_string_pretty(&disposizione_ripristinata).unwrap()
        );

        server.datastore.close();
        let _ = std::fs::remove_dir_all(&cartella_test);
    }

    /// Test DISTRUTTIVO richiesto esplicitamente dall'utente su questo PC
    /// di sviluppo ("dati fittizi... non avere paura di eliminare o
    /// modificare i dati") — a differenza di tutti i test sopra (isolati
    /// su una cartella temporanea), questo lavora DAVVERO sul database
    /// reale di questo PC: esporta la disposizione Home e la cronologia
    /// AFK vere, le CANCELLA per davvero (non una copia), le reimporta e
    /// verifica che tornino identiche, poi reimporta una seconda volta
    /// per verificare che non si duplichi nulla — la prova più diretta
    /// possibile che l'intero ciclo funzioni con dati reali, non solo
    /// sintetici. NON lanciare su un PC con dati reali importanti: qui è
    /// autorizzato esplicitamente solo perché i dati di questo PC sono
    /// fittizi. Scrive comunque un backup di sicurezza su disco prima di
    /// cancellare, rimosso solo se il ripristino verifica corretto.
    #[tokio::test]
    #[ignore]
    async fn test_reale_distruttivo_elimina_e_reimporta() {
        let app_data_dir = PathBuf::from(std::env::var("LOCALAPPDATA").unwrap())
            .join("TrackFlow")
            .join("app-data");
        let server = Arc::new(crate::build_app_server(&app_data_dir, &app_data_dir, &app_data_dir).await);
        let hostname = gethostname::gethostname().to_string_lossy().to_string();

        // --- Cattura lo stato reale PRIMA di toccare qualunque cosa ---
        let views_prima = server.get_setting("views").await;
        assert!(views_prima.is_some(), "precondizione: 'views' deve esistere davvero su questo PC");
        let eventi_afk_prima = leggi_tutti_gli_eventi(&server, BUCKET_AFK).await;
        assert!(!eventi_afk_prima.is_empty(), "precondizione: eventi AFK reali devono esistere su questo PC");
        println!("PRIMA: 'views' presente, {} eventi AFK reali", eventi_afk_prima.len());

        // "Esporta" le due sezioni per davvero, sui dati veri.
        let sezione_afk_esportata = costruisci_sezione_afk_finestra(&server, &app_data_dir).await;
        assert_eq!(sezione_afk_esportata.eventi_afk.len(), eventi_afk_prima.len());

        // Backup di sicurezza su disco PRIMA di cancellare — extra
        // cautela anche se autorizzato esplicitamente, così un imprevisto
        // qualsiasi resta comunque recuperabile a mano.
        let backup = serde_json::json!({ "views": views_prima, "eventi_afk": eventi_afk_prima });
        let percorso_backup = app_data_dir.join("backups").join("test-distruttivo-backup.json");
        std::fs::write(&percorso_backup, serde_json::to_string_pretty(&backup).unwrap())
            .expect("scrittura backup di sicurezza");
        println!("Backup di sicurezza scritto in {}", percorso_backup.display());

        // --- Elimina per davvero ---
        server.delete_setting_di_test("views").await;
        server.datastore.delete_bucket(BUCKET_AFK).expect("cancellazione bucket AFK reale");

        assert!(server.get_setting("views").await.is_none(), "'views' deve essere davvero sparita");
        assert!(
            leggi_tutti_gli_eventi(&server, BUCKET_AFK).await.is_empty(),
            "il bucket AFK deve essere davvero vuoto/assente"
        );
        println!("Eliminati per davvero: 'views' e il bucket {BUCKET_AFK}");

        // --- Reimporta SOLO le due sezioni esportate prima ---
        server.set_setting("views", views_prima.as_ref().unwrap()).await.unwrap();
        applica_sezione_afk_finestra(&server, &app_data_dir, &sezione_afk_esportata, &hostname).await;

        let views_dopo = server.get_setting("views").await.expect("'views' deve essere tornata");
        assert_eq!(
            views_dopo,
            *views_prima.as_ref().unwrap(),
            "la disposizione Home deve tornare esattamente identica"
        );

        let eventi_afk_dopo = leggi_tutti_gli_eventi(&server, BUCKET_AFK).await;
        assert_eq!(
            eventi_afk_dopo.len(),
            eventi_afk_prima.len(),
            "il numero di eventi AFK deve tornare esattamente quello di prima"
        );

        // Reimporta una SECONDA volta — stesso identico file esportato —
        // per verificare che non si duplichi nulla, sui dati veri.
        applica_sezione_afk_finestra(&server, &app_data_dir, &sezione_afk_esportata, &hostname).await;
        let eventi_afk_dopo_secondo_reimport = leggi_tutti_gli_eventi(&server, BUCKET_AFK).await;
        assert_eq!(
            eventi_afk_dopo_secondo_reimport.len(),
            eventi_afk_prima.len(),
            "reimportare una seconda volta non deve duplicare gli eventi AFK reali"
        );

        println!(
            "DOPO: 'views' ripristinata identica, {} eventi AFK (invariati dopo doppio reimport)",
            eventi_afk_dopo_secondo_reimport.len()
        );

        // Il backup di sicurezza non serve più: tutto è tornato
        // esattamente come prima, verificato sopra.
        let _ = std::fs::remove_file(&percorso_backup);

        server.datastore.close();
    }
}

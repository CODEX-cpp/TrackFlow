//! Categorizzazione automatica delle app via Claude — richiesta esplicita
//! dell'utente: quando compare un'app mai categorizzata, se è configurata
//! una chiave API AI (vedi agent.rs, stessa config/provider riusati qui),
//! le viene chiesto di assegnarla a una categoria (es. Lavoro, Svago),
//! creandone una nuova se serve. Se non è configurata nessuna chiave, non
//! succede nulla — nessun fallback manuale qui, quello arriverà con
//! l'interfaccia Impostazioni dedicata (fuori scope per questa fase).
//!
//! Vincolo esplicito dell'utente: l'AI può assegnare app e creare
//! categorie, ma NON può eliminare categorie né spostare un'app già
//! assegnata altrove. Il vincolo è imposto qui lato Rust — non c'è
//! nessuno strumento esposto per farlo, indipendentemente da cosa
//! chiederebbe il modello (stessa filosofia dei tool di sola lettura in
//! agent.rs: non ci si fida di un'istruzione nel prompt per una garanzia
//! che invece va imposta nel codice).
//!
//! Innescata da `spawn_stdout_drain` in lib.rs, nello stesso punto in cui
//! ogni nome app viene già inoltrato al watcher icone — vedi `su_nuova_app`
//! più sotto.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::{json, Value};
use tokio::sync::Mutex as AsyncMutex;

use crate::agent::{self, AiAgentConfig};
use crate::AppServer;

/// Una categoria e le app assegnate — stessa forma che finirà salvata
/// sotto la chiave impostazioni "appCategories" (via
/// `AppServer::get_setting`/`set_setting`), la stessa che in futuro
/// leggerà anche l'interfaccia Impostazioni dedicata (non ancora
/// costruita, fuori scope qui).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppCategory {
    pub name: String,
    #[serde(default)]
    pub apps: Vec<String>,
}

const SETTINGS_KEY: &str = "appCategories";

/// Stato condiviso per la categorizzazione automatica — gestito come
/// risorsa Tauri (`app.manage(...)`), stesso pattern di `AiAgentState`.
pub struct CategorizationState {
    /// Serializza le richieste una alla volta: senza, due app "nuove"
    /// scoperte quasi in contemporanea potrebbero leggere entrambe
    /// "nessuna categoria esiste ancora" e finire entrambe nel percorso
    /// bulk, o due assegnazioni concorrenti potrebbero sovrascriversi a
    /// vicenda al salvataggio.
    lock: AsyncMutex<()>,
    /// App già gestite in questa sessione (già categorizzata O già
    /// tentata almeno una volta, con successo o meno) — evita di
    /// richiamare Claude ad ogni singolo cambio di finestra per un'app
    /// che resta ostinatamente non categorizzata (es. un errore di rete
    /// una tantum). Non persistito apposta: un nuovo tentativo dopo un
    /// riavvio dell'app non è un problema, anzi è la rete di sicurezza
    /// per un fallimento transitorio.
    gia_gestite: std::sync::Mutex<HashSet<String>>,
}

impl CategorizationState {
    pub fn new() -> Self {
        Self { lock: AsyncMutex::new(()), gia_gestite: std::sync::Mutex::new(HashSet::new()) }
    }
}

const MAX_ITERAZIONI_TOOL: u8 = 20;

const SYSTEM_PROMPT: &str = "Sei un sistema che organizza automaticamente in categorie le applicazioni desktop usate su un PC (es. Lavoro, Svago, Comunicazione, Sviluppo, Intrattenimento, Sistema — inventane altre se servono, brevi e chiare, in italiano). Ricevi le categorie già esistenti (con le app già assegnate) e un elenco di app NON ancora categorizzate. Per OGNI app dell'elenco fornito, usa lo strumento assegna_app_categoria per assegnarla alla categoria più adatta: se una categoria esistente va bene, usa quella; altrimenti creane una nuova con crea_categoria prima di assegnare. Assegna tutte le app dell'elenco prima di terminare, non lasciarne nessuna senza categoria. Non hai nessuno strumento per eliminare una categoria o spostare un'app già assegnata: se un'app risulta già presente in una categoria esistente, ignorala e basta.";

fn definisci_strumenti() -> Vec<Value> {
    vec![
        json!({
            "name": "crea_categoria",
            "description": "Crea una nuova categoria vuota per raggruppare app (es. \"Lavoro\", \"Svago\"). Se esiste già una categoria con lo stesso nome (anche a maiuscole/minuscole diverse), non fa nulla.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "nome": { "type": "string", "description": "Nome della categoria, breve e chiaro, in italiano" },
                },
                "required": ["nome"],
            },
        }),
        json!({
            "name": "assegna_app_categoria",
            "description": "Assegna un'app (nome esatto come fornito nell'elenco) a una categoria, esistente o appena creata con crea_categoria. Funziona SOLO se l'app non è già assegnata a nessuna categoria — non è possibile spostare un'app già categorizzata, il tentativo viene rifiutato.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "app": { "type": "string", "description": "Nome esatto dell'app come fornito nell'elenco da categorizzare" },
                    "categoria": { "type": "string", "description": "Nome della categoria a cui assegnarla" },
                },
                "required": ["app", "categoria"],
            },
        }),
    ]
}

/// Unico punto che decide cosa Claude può davvero fare qui: crea_categoria
/// e assegna_app_categoria, nient'altro — vedi il commento in testa al
/// modulo sul perché niente eliminazione/spostamento. Non propaga mai un
/// errore Rust al modello: torna un risultato con chiave "errore" così
/// può correggersi da solo nel giro successivo.
/// Il modello a volte ripete anche il "nome leggibile" dato come contesto
/// nel prompt (es. "app.exe (TrackFlow)") invece del solo identificativo
/// grezzo ("app.exe") — verificato dal vivo, capita davvero anche
/// istruendolo esplicitamente nel prompt a non farlo. Non ci si può
/// fidare del prompt per una garanzia che invece va imposta qui: se
/// `app` non combacia con nessuno degli identificativi validi ma
/// combacia dopo aver tagliato da " (" in poi, si usa quella versione
/// tagliata — altrimenti l'input viene rifiutato, MAI salvato così com'è
/// se non è un identificativo riconosciuto (altrimenti l'app non
/// verrebbe più riconosciuta come già categorizzata in futuro, dato che
/// gli eventi reali della finestra mandano sempre e solo il nome grezzo).
fn normalizza_app(app: &str, identificativi_validi: &[String]) -> Option<String> {
    if identificativi_validi.iter().any(|a| a.eq_ignore_ascii_case(app)) {
        return Some(app.to_string());
    }
    // Taglia da una parentesi tonda o quadra in poi, qualunque sia il
    // formato usato nel prompt (vedi elenco_app in esegui_categorizzazione)
    // — non ci si fida che il modello segua esattamente le istruzioni di
    // formato.
    let taglio = app.find(" (").or_else(|| app.find(" ["));
    if let Some(idx) = taglio {
        let base = app[..idx].trim();
        if identificativi_validi.iter().any(|a| a.eq_ignore_ascii_case(base)) {
            return Some(base.to_string());
        }
    }
    None
}

fn esegui_strumento(
    categorie: &mut Vec<AppCategory>,
    identificativi_validi: &[String],
    nome: &str,
    input: &Value,
) -> Value {
    match nome {
        "crea_categoria" => {
            let Some(nome_categoria) = input["nome"].as_str().map(str::trim).filter(|s| !s.is_empty())
            else {
                return json!({ "errore": "nome categoria mancante o vuoto" });
            };
            if categorie.iter().any(|c| c.name.eq_ignore_ascii_case(nome_categoria)) {
                return json!({ "risultato": format!("La categoria '{nome_categoria}' esiste già.") });
            }
            categorie.push(AppCategory { name: nome_categoria.to_string(), apps: Vec::new() });
            json!({ "risultato": format!("Categoria '{nome_categoria}' creata.") })
        }
        "assegna_app_categoria" => {
            let Some(app_grezzo) = input["app"].as_str().map(str::trim).filter(|s| !s.is_empty()) else {
                return json!({ "errore": "app mancante o vuota" });
            };
            let Some(nome_categoria) =
                input["categoria"].as_str().map(str::trim).filter(|s| !s.is_empty())
            else {
                return json!({ "errore": "categoria mancante o vuota" });
            };
            let Some(app) = normalizza_app(app_grezzo, identificativi_validi) else {
                return json!({
                    "errore": format!("'{app_grezzo}' non è un identificativo valido dell'elenco fornito — usa esattamente il nome prima di un'eventuale parentesi.")
                });
            };
            if categorie.iter().any(|c| c.apps.iter().any(|a| a.eq_ignore_ascii_case(&app))) {
                return json!({
                    "errore": format!("'{app}' è già assegnata a una categoria — non può essere spostata.")
                });
            }
            match categorie.iter_mut().find(|c| c.name.eq_ignore_ascii_case(nome_categoria)) {
                Some(categoria) => categoria.apps.push(app.clone()),
                None => categorie
                    .push(AppCategory { name: nome_categoria.to_string(), apps: vec![app.clone()] }),
            }
            json!({ "risultato": format!("'{app}' assegnata a '{nome_categoria}'.") })
        }
        altro => json!({ "errore": format!("strumento sconosciuto: {altro}") }),
    }
}

/// `pub(crate)`: riusata anche da `agent.rs` per i tool di lettura/
/// scrittura categorie della chat (gestione manuale su richiesta
/// dell'utente in conversazione — vincoli diversi da questo modulo, che
/// riguarda solo la categorizzazione automatica di app nuove).
pub(crate) async fn carica_categorie(server: &AppServer) -> Vec<AppCategory> {
    match server.get_setting(SETTINGS_KEY).await {
        Some(valore) => serde_json::from_value(valore).unwrap_or_default(),
        None => Vec::new(),
    }
}

pub(crate) async fn salva_categorie(server: &AppServer, categorie: &[AppCategory]) -> Result<(), String> {
    server.set_setting(SETTINGS_KEY, &json!(categorie)).await
}

const APP_CONOSCIUTE_FILE: &str = "known-apps.json";

fn known_apps_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(APP_CONOSCIUTE_FILE)
}

fn carica_known_apps_da_file(app_data_dir: &Path) -> Option<Vec<String>> {
    let contenuto = std::fs::read_to_string(known_apps_path(app_data_dir)).ok()?;
    serde_json::from_str(&contenuto).ok()
}

fn salva_known_apps_su_file(app_data_dir: &Path, app: &[String]) {
    if let Ok(json) = serde_json::to_string_pretty(app) {
        let _ = std::fs::write(known_apps_path(app_data_dir), json);
    }
}

/// Chiamata per OGNI evento del watcher finestra (`spawn_stdout_drain` in
/// lib.rs), NON solo quando la categorizzazione AI è configurata —
/// l'elenco delle app conosciute deve restare aggiornato dall'installazione
/// in poi, a prescindere da quando/se viene mai collegata una chiave AI
/// (richiesta esplicita dell'utente). Economica nel caso comune (l'app è
/// già nell'elenco, nessuna scrittura); scrive il file solo quando compare
/// davvero un'app mai vista prima.
pub fn registra_app_se_nuova(app_data_dir: &Path, app: &str) {
    let mut app_conosciute = carica_known_apps_da_file(app_data_dir).unwrap_or_default();
    if app_conosciute.iter().any(|a| a == app) {
        return;
    }
    app_conosciute.push(app.to_string());
    salva_known_apps_su_file(app_data_dir, &app_conosciute);
}

/// Ricostruzione una tantum dell'elenco interrogando la Timeline reale
/// (bucket finestra, raggruppato per nome app; stesso motore AQL usato
/// ovunque nell'app, nessun limite temporale stretto) — usata SOLO da
/// `carica_app_conosciute` sotto, quando `known-apps.json` non esiste
/// ancora (aggiornamento da una versione precedente a questa
/// funzionalità, con mesi di Timeline già raccolti ma mai registrati
/// incrementalmente). Bug reale che questo intero meccanismo sostituisce:
/// la versione precedente leggeva la cartella delle icone (`app-icons/`),
/// fonte sbagliata perché contiene anche processi di sistema mai apparsi
/// come finestra attiva (audiodg.exe, conhost.exe, svchost.exe, ecc. — la
/// scansione di avvio in lib.rs, `invia_processi_in_esecuzione_a_icone`,
/// manda all'estrattore icone OGNI processo in esecuzione sul sistema, non
/// solo quelli mostrati a schermo) — le due cose erano collegate solo per
/// riuso di comodo, non per un legame logico vero.
async fn ricostruisci_app_conosciute_da_timeline(server: &AppServer, _hostname: &str) -> Vec<String> {
    // Niente più suffisso "_<hostname>" — vedi il commento in
    // aw-watcher-afk-rust/src/main.rs. Il parametro resta per non
    // scombinare i chiamanti, che lo passano già per altre ragioni.
    let bucket = "aw-watcher-window".to_string();
    let query_lines = vec![
        format!("events = flood(query_bucket(\"{bucket}\"));"),
        "apps = merge_events_by_keys(events, [\"app\"]);".to_string(),
        "RETURN = apps;".to_string(),
    ];
    let timeperiod = "2000-01-01T00:00:00+00:00/2100-01-01T00:00:00+00:00".to_string();

    let Ok(risposta) = server.query(vec![timeperiod], query_lines).await else {
        return Vec::new();
    };
    risposta
        .as_array()
        .and_then(|periodi| periodi.first())
        .and_then(|eventi| eventi.as_array())
        .map(|eventi| {
            eventi.iter().filter_map(|e| e["data"]["app"].as_str().map(str::to_string)).collect()
        })
        .unwrap_or_default()
}

/// Punto d'ingresso unico per leggere l'elenco delle app conosciute:
/// legge `known-apps.json` (veloce, il caso comune), oppure — solo la
/// primissima volta che il file non esiste ancora — lo ricostruisce
/// interrogando la Timeline reale e lo salva per tutte le letture
/// successive.
pub(crate) async fn carica_app_conosciute(
    server: &AppServer,
    app_data_dir: &Path,
    hostname: &str,
) -> Vec<String> {
    if let Some(app) = carica_known_apps_da_file(app_data_dir) {
        return app;
    }
    let app = ricostruisci_app_conosciute_da_timeline(server, hostname).await;
    salva_known_apps_su_file(app_data_dir, &app);
    app
}

/// Nomi leggibili noti (`appAutoNames.json`, scritto dal watcher icone) —
/// solo per dare più contesto a Claude nel prompt (es. "code.exe
/// (Visual Studio Code)"), non usati come chiave di assegnazione (quella
/// resta sempre il nome grezzo, coerente con tutto il resto dell'app).
/// Stesso trattamento BOM-tolerante del watcher icone stesso — file
/// piccolo, letto una volta per ogni chiamata di categorizzazione.
pub(crate) fn carica_nomi_leggibili(app_data_dir: &Path) -> HashMap<String, String> {
    let path = app_data_dir.join("appAutoNames.json");
    let Ok(bytes) = std::fs::read(&path) else {
        return HashMap::new();
    };
    let content = String::from_utf8_lossy(&bytes);
    let content = content.strip_prefix('\u{feff}').unwrap_or(&content);
    serde_json::from_str::<HashMap<String, Value>>(content)
        .map(|m| m.into_iter().filter_map(|(k, v)| v.as_str().map(|s| (k, s.to_string()))).collect())
        .unwrap_or_default()
}

fn app_gia_categorizzata(categorie: &[AppCategory], app: &str) -> bool {
    categorie.iter().any(|c| c.apps.iter().any(|a| a.eq_ignore_ascii_case(app)))
}

/// Costruisce il messaggio iniziale (categorie esistenti + elenco app da
/// categorizzare) — fattorizzato fuori da `esegui_categorizzazione`
/// perché serve identico a entrambi i percorsi (API diretta "anthropic"
/// e Claude Desktop via bridge MCP, vedi sotto).
fn costruisci_messaggio_iniziale(
    categorie: &[AppCategory],
    da_categorizzare: &[String],
    nomi_leggibili: &HashMap<String, String>,
) -> String {
    let elenco_categorie = if categorie.is_empty() {
        "Nessuna categoria esiste ancora.".to_string()
    } else {
        categorie
            .iter()
            .map(|c| {
                let app = if c.apps.is_empty() { "(vuota)".to_string() } else { c.apps.join(", ") };
                format!("- {}: {app}", c.name)
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let elenco_app = da_categorizzare
        .iter()
        .map(|app| match nomi_leggibili.get(app) {
            Some(leggibile) if leggibile != app => {
                format!("- {app}  [solo per contesto, non usarlo come identificativo: {leggibile}]")
            }
            _ => format!("- {app}"),
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "Categorie esistenti:\n{elenco_categorie}\n\nApp da categorizzare (NON ancora assegnate a nessuna categoria). Il testo tra parentesi quadre, quando presente, è solo un aiuto per capire di che app si tratta — l'IDENTIFICATIVO ESATTO da passare a assegna_app_categoria è SEMPRE e SOLO la parte prima delle parentesi quadre:\n{elenco_app}"
    )
}

/// Il vero ciclo tool-calling per il provider "anthropic" (API diretta a
/// consumo, unico che parla il formato Anthropic messages/tools così
/// com'è) — nessuna cronologia persistita (a differenza della chat,
/// questo è un task una tantum, non una conversazione): un unico
/// messaggio iniziale con le categorie esistenti + l'elenco da
/// categorizzare, poi round di tool_use/tool_result finché il modello si
/// ferma o si supera il tetto di iterazioni. Per il provider Claude
/// Desktop, che parla solo MCP, vedi
/// `esegui_categorizzazione_claude_desktop` invece — il ciclo lì è
/// interamente interno al CLI Claude Code, non guidato da qui.
async fn esegui_categorizzazione_anthropic(
    config: &AiAgentConfig,
    categorie: &mut Vec<AppCategory>,
    da_categorizzare: &[String],
    messaggio_iniziale: &str,
) -> Result<(), String> {
    let strumenti = definisci_strumenti();
    let mut messaggi = vec![json!({ "role": "user", "content": messaggio_iniziale })];

    for _ in 0..MAX_ITERAZIONI_TOOL {
        let risposta =
            agent::invia_provider(config, SYSTEM_PROMPT, messaggi.clone(), strumenti.clone()).await?;
        let blocchi = risposta["content"].as_array().cloned().unwrap_or_default();
        messaggi.push(json!({ "role": "assistant", "content": blocchi.clone() }));

        if risposta["stop_reason"] != "tool_use" {
            return Ok(());
        }

        let mut risultati_blocchi = Vec::new();
        for blocco in &blocchi {
            if blocco["type"] == "tool_use" {
                let tool_id = blocco["id"].as_str().unwrap_or("").to_string();
                let nome = blocco["name"].as_str().unwrap_or("").to_string();
                let risultato = esegui_strumento(categorie, da_categorizzare, &nome, &blocco["input"]);
                risultati_blocchi.push(json!({
                    "type": "tool_result",
                    "tool_use_id": tool_id,
                    "content": risultato.to_string(),
                }));
            }
        }
        messaggi.push(json!({ "role": "user", "content": risultati_blocchi }));
    }

    log::warn!(
        "Categorizzazione AI: raggiunto il tetto di {MAX_ITERAZIONI_TOOL} iterazioni — salvo comunque quanto assegnato finora."
    );
    Ok(())
}

/// Stessa funzionalità di `esegui_categorizzazione_anthropic`, ma per il
/// provider Claude Desktop — che non parla l'API Anthropic diretta (nessuna
/// chiave, vedi claude_subscription.rs), solo MCP. Qui il ciclo
/// tool_use/tool_result NON è guidato da questo codice: un'unica chiamata
/// a `claude_subscription::esegui_task_una_tantum` avvia un bridge MCP
/// minimale che espone crea_categoria/assegna_app_categoria, e il CLI
/// Claude Code stesso richiama quegli strumenti (via HTTP verso il
/// bridge, gestiti dalla stessa `esegui_strumento` di sempre) finché
/// decide di aver finito — noi riceviamo solo il risultato finale.
/// `categorie` viene condiviso con la closure passata al bridge dietro
/// un Mutex perché quella gira su un thread separato (il thread che
/// accetta le connessioni del bridge), non nel task async chiamante.
async fn esegui_categorizzazione_claude_desktop(
    config: &AiAgentConfig,
    categorie: &mut Vec<AppCategory>,
    da_categorizzare: &[String],
    messaggio_iniziale: &str,
    app_data_dir: &Path,
) -> Result<(), String> {
    let condivise = Arc::new(std::sync::Mutex::new(std::mem::take(categorie)));
    let identificativi = da_categorizzare.to_vec();
    let condivise_per_gestore = condivise.clone();
    let gestore = move |nome: &str, input: &Value| {
        let mut c = condivise_per_gestore.lock().unwrap();
        esegui_strumento(&mut c, &identificativi, nome, input)
    };

    let esito = crate::claude_subscription::esegui_task_una_tantum(
        &config.model,
        SYSTEM_PROMPT,
        messaggio_iniziale,
        definisci_strumenti(),
        gestore,
        "claude-agent-workdir-categorizzazione",
        app_data_dir,
    )
    .await;

    // Recuperate SEMPRE, anche in caso di errore a metà — quel che è
    // stato assegnato prima dell'errore va comunque salvato (stessa
    // filosofia del ramo "anthropic": un tetto/errore non deve buttare
    // via il lavoro già fatto).
    *categorie = match Arc::try_unwrap(condivise) {
        Ok(mutex) => mutex.into_inner().unwrap(),
        Err(arc) => arc.lock().unwrap().clone(),
    };

    esito.map(|_| ())
}

async fn esegui_categorizzazione(
    config: &AiAgentConfig,
    categorie: &mut Vec<AppCategory>,
    da_categorizzare: &[String],
    nomi_leggibili: &HashMap<String, String>,
    app_data_dir: &Path,
) -> Result<(), String> {
    let messaggio_iniziale = costruisci_messaggio_iniziale(categorie, da_categorizzare, nomi_leggibili);

    if config.provider == crate::claude_subscription::PROVIDER_ID {
        esegui_categorizzazione_claude_desktop(
            config,
            categorie,
            da_categorizzare,
            &messaggio_iniziale,
            app_data_dir,
        )
        .await
    } else {
        esegui_categorizzazione_anthropic(config, categorie, da_categorizzare, &messaggio_iniziale).await
    }
}

/// Punto d'ingresso chiamato da `spawn_stdout_drain` (lib.rs) per ogni
/// nome app ricevuto dal watcher finestra. Non blocca il chiamante: fa
/// solo un controllo sincrono economico (AI configurata? app già gestita
/// in questa sessione?) e poi lancia il resto — compresa la chiamata di
/// rete a Claude, che può richiedere diversi secondi — in un task
/// separato, così il drenaggio degli eventi verso il datastore non si
/// ferma mai ad aspettarla.
pub fn su_nuova_app(
    server: Arc<AppServer>,
    app_data_dir: PathBuf,
    stato: Arc<CategorizationState>,
    hostname: String,
    app: String,
) {
    let Some(config) = agent::load_config(&app_data_dir) else {
        return;
    };
    // Bug reale segnalato dall'utente: con il provider Claude Desktop
    // (nessuna chiave API, l'abbonamento fa da autenticazione) questo
    // controllo tornava sempre vero e la categorizzazione automatica
    // non scattava mai, nonostante il provider fosse configurato e
    // funzionante per la chat. "Configurato" ora significa: chiave
    // presente PER "anthropic", oppure semplicemente questo provider
    // (che non ne ha bisogno).
    if config.provider != crate::claude_subscription::PROVIDER_ID && config.api_key.trim().is_empty() {
        return;
    }

    {
        let mut gia = stato.gia_gestite.lock().unwrap();
        if gia.contains(&app) {
            return;
        }
        gia.insert(app.clone());
    }

    tauri::async_runtime::spawn(async move {
        let _guard = stato.lock.lock().await;

        // Rilegge le categorie DENTRO il lock (non riusa una cache) —
        // può essere cambiato nel frattempo (un'altra app appena
        // categorizzata da un giro precedente ancora in corso).
        let mut categorie = carica_categorie(&server).await;
        if app_gia_categorizzata(&categorie, &app) {
            return;
        }

        let (da_categorizzare, bulk) = if categorie.is_empty() {
            (carica_app_conosciute(&server, &app_data_dir, &hostname).await, true)
        } else {
            (vec![app.clone()], false)
        };
        if da_categorizzare.is_empty() {
            return;
        }

        let nomi_leggibili = carica_nomi_leggibili(&app_data_dir);
        log::info!(
            "Categorizzazione AI: {} app da categorizzare ({})",
            da_categorizzare.len(),
            if bulk { "primo giro, nessuna categoria esistente ancora" } else { "incrementale" }
        );

        if let Err(e) = esegui_categorizzazione(
            &config,
            &mut categorie,
            &da_categorizzare,
            &nomi_leggibili,
            &app_data_dir,
        )
        .await
        {
            log::error!("Categorizzazione AI fallita: {e}");
            return;
        }

        if let Err(e) = salva_categorie(&server, &categorie).await {
            log::error!("Salvataggio categorie fallito: {e}");
        }
    });
}

/// Forza una scansione di TUTTE le app già conosciute ma non ancora
/// categorizzate — chiamata da `agent::ai_agent_save_config` subito dopo
/// che l'utente salva una chiave AI valida.
///
/// Bug reale segnalato dall'utente (2026-08-15): senza questo, la
/// categorizzazione automatica scatta solo al prossimo cambio di finestra
/// attiva (`su_nuova_app` sopra) — chi installa l'app, la usa per ore e
/// SOLO DOPO collega la chiave AI si ritrova con ore di app già tracciate
/// ma mai riviste finché non ricapita per caso di passarci sopra di nuovo,
/// anche a distanza di giorni. `su_nuova_app` fa già una scansione "bulk"
/// simile, ma SOLO se non esiste ancora nessuna categoria (`categorie.is_empty()`,
/// pensato per il primissimo avvio) — qui invece si considera sempre e
/// comunque ogni app conosciuta non ancora assegnata, a prescindere da
/// quante categorie esistano già.
pub fn forza_scansione_iniziale(
    server: Arc<AppServer>,
    app_data_dir: PathBuf,
    stato: Arc<CategorizationState>,
    hostname: String,
    config: AiAgentConfig,
) {
    // Stesso criterio di `su_nuova_app` — vedi il commento lì per il bug
    // reale col provider Claude Desktop.
    if config.provider != crate::claude_subscription::PROVIDER_ID && config.api_key.trim().is_empty() {
        return;
    }

    tauri::async_runtime::spawn(async move {
        let _guard = stato.lock.lock().await;

        let mut categorie = carica_categorie(&server).await;
        let da_categorizzare: Vec<String> = carica_app_conosciute(&server, &app_data_dir, &hostname)
            .await
            .into_iter()
            .filter(|app| !app_gia_categorizzata(&categorie, app))
            .collect();
        if da_categorizzare.is_empty() {
            return;
        }

        let nomi_leggibili = carica_nomi_leggibili(&app_data_dir);
        log::info!(
            "Categorizzazione AI: scansione forzata dopo il collegamento della chiave, {} app da categorizzare",
            da_categorizzare.len()
        );

        if let Err(e) = esegui_categorizzazione(
            &config,
            &mut categorie,
            &da_categorizzare,
            &nomi_leggibili,
            &app_data_dir,
        )
        .await
        {
            log::error!("Categorizzazione AI (scansione forzata) fallita: {e}");
            return;
        }

        if let Err(e) = salva_categorie(&server, &categorie).await {
            log::error!("Salvataggio categorie fallito (scansione forzata): {e}");
        }
    });
}

/// App conosciuta, con nome leggibile quando disponibile — usata
/// dall'interfaccia Impostazioni (sezione categorie, gestione manuale)
/// per mostrare l'elenco completo delle app da categorizzare/vedere già
/// categorizzate. Stessa identica fonte dati (`carica_app_conosciute`)
/// già usata dalla categorizzazione automatica AI, per restare coerenti
/// fra le due — un'app che l'AI considera "conosciuta" è la stessa che
/// vede l'utente qui.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AppConosciuta {
    pub app: String,
    pub nome_leggibile: Option<String>,
}

#[tauri::command]
pub async fn elenca_app_conosciute(app_handle: tauri::AppHandle) -> Vec<AppConosciuta> {
    use tauri::Manager;
    let Some(dir) = app_handle.try_state::<crate::AppDataDirState>() else {
        return Vec::new();
    };
    let Some(server) = app_handle.try_state::<Arc<AppServer>>() else {
        return Vec::new();
    };
    let hostname = gethostname::gethostname().to_string_lossy().to_string();
    let mut app = carica_app_conosciute(&server, &dir.0, &hostname).await;
    app.sort();
    let nomi = carica_nomi_leggibili(&dir.0);
    app.into_iter()
        .map(|a| {
            let nome_leggibile = nomi.get(&a).filter(|n| n.as_str() != a).cloned();
            AppConosciuta { app: a, nome_leggibile }
        })
        .collect()
}

#[cfg(test)]
mod test_manuale {
    //! Test manuale, NON eseguito da un normale `cargo test` (`#[ignore]`)
    //! — verifica dal vivo, contro il database reale e un vero
    //! `claude.exe`, che la categorizzazione automatica funziona anche
    //! col provider Claude Desktop (bug reale segnalato dall'utente:
    //! "quando claude desktop... viene collegato, la categorizzazione
    //! automatica non avviene" — vedi i due fix in `su_nuova_app`/
    //! `forza_scansione_iniziale` più sopra). Va lanciato con `app.exe`
    //! GIÀ CHIUSO (stesso database SQLite, evitare accessi concorrenti).
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_categorizzazione_reale_claude_desktop() {
        let app_data_dir = PathBuf::from(std::env::var("LOCALAPPDATA").unwrap())
            .join("TrackFlow")
            .join("app-data");
        let cartella_neutra = std::env::temp_dir();

        let server = Arc::new(crate::build_app_server(&app_data_dir, &cartella_neutra, &cartella_neutra).await);
        let hostname = gethostname::gethostname().to_string_lossy().to_string();

        let config = agent::load_config(&app_data_dir).expect("config AI non trovata — collega prima un provider");
        assert_eq!(config.provider, crate::claude_subscription::PROVIDER_ID, "questo test verifica proprio quel provider");

        let mut categorie = carica_categorie(&server).await;
        println!("Categorie PRIMA: {categorie:?}");

        let da_categorizzare: Vec<String> = carica_app_conosciute(&server, &app_data_dir, &hostname)
            .await
            .into_iter()
            .filter(|app| !app_gia_categorizzata(&categorie, app))
            .collect();
        println!("Da categorizzare: {} app", da_categorizzare.len());
        assert!(!da_categorizzare.is_empty(), "nessuna app da categorizzare — test non significativo");

        let nomi_leggibili = carica_nomi_leggibili(&app_data_dir);

        esegui_categorizzazione(&config, &mut categorie, &da_categorizzare, &nomi_leggibili, &app_data_dir)
            .await
            .expect("categorizzazione fallita");

        println!("Categorie DOPO: {categorie:?}");
        assert!(!categorie.is_empty(), "doveva essere stata creata almeno una categoria");

        salva_categorie(&server, &categorie).await.expect("salvataggio fallito");
        server.datastore.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crea_categoria_e_idempotente_case_insensitive() {
        let mut categorie: Vec<AppCategory> = Vec::new();
        let valide: Vec<String> = Vec::new();

        let r = esegui_strumento(&mut categorie, &valide, "crea_categoria", &json!({ "nome": "Lavoro" }));
        assert!(r.get("risultato").is_some());
        assert_eq!(categorie.len(), 1);

        // Stesso nome, maiuscole diverse: non deve creare un duplicato.
        let r = esegui_strumento(&mut categorie, &valide, "crea_categoria", &json!({ "nome": "lavoro" }));
        assert!(r.get("risultato").is_some());
        assert_eq!(categorie.len(), 1);
    }

    #[test]
    fn assegna_app_categoria_crea_la_categoria_se_mancante() {
        let mut categorie: Vec<AppCategory> = Vec::new();
        let valide = vec!["code.exe".to_string()];

        let r = esegui_strumento(
            &mut categorie,
            &valide,
            "assegna_app_categoria",
            &json!({ "app": "code.exe", "categoria": "Sviluppo" }),
        );
        assert!(r.get("risultato").is_some(), "{r:?}");
        assert_eq!(categorie.len(), 1);
        assert_eq!(categorie[0].name, "Sviluppo");
        assert_eq!(categorie[0].apps, vec!["code.exe".to_string()]);
    }

    #[test]
    fn assegna_app_categoria_rifiuta_di_spostare_un_app_gia_assegnata() {
        let mut categorie =
            vec![AppCategory { name: "Sviluppo".to_string(), apps: vec!["code.exe".to_string()] }];
        let valide = vec!["code.exe".to_string()];

        let r = esegui_strumento(
            &mut categorie,
            &valide,
            "assegna_app_categoria",
            &json!({ "app": "code.exe", "categoria": "Svago" }),
        );
        assert!(r.get("errore").is_some(), "doveva essere rifiutato: {r:?}");
        // Nessuna modifica: resta dove era, non è comparsa da nessun'altra parte.
        assert_eq!(categorie.len(), 1);
        assert_eq!(categorie[0].name, "Sviluppo");
        assert_eq!(categorie[0].apps, vec!["code.exe".to_string()]);
    }

    #[test]
    fn esegui_strumento_non_espone_ne_eliminazione_ne_spostamento() {
        // Garanzia strutturale, non solo comportamentale: verifica che i
        // due unici nomi di tool riconosciuti siano esattamente questi
        // due — qualunque altro nome (es. "elimina_categoria",
        // "sposta_app") torna un errore "strumento sconosciuto", non
        // viene eseguito.
        let mut categorie =
            vec![AppCategory { name: "Sviluppo".to_string(), apps: vec!["code.exe".to_string()] }];
        let valide = vec!["code.exe".to_string()];

        let r = esegui_strumento(&mut categorie, &valide, "elimina_categoria", &json!({ "nome": "Sviluppo" }));
        assert!(r.get("errore").is_some());
        assert_eq!(categorie.len(), 1, "la categoria non deve essere stata toccata");

        let r = esegui_strumento(
            &mut categorie,
            &valide,
            "sposta_app",
            &json!({ "app": "code.exe", "categoria": "Svago" }),
        );
        assert!(r.get("errore").is_some());
        assert_eq!(categorie[0].apps, vec!["code.exe".to_string()], "l'app non deve essere stata spostata");
    }

    #[test]
    fn normalizza_app_riconosce_identificativo_esatto() {
        let valide = vec!["code.exe".to_string()];
        assert_eq!(normalizza_app("code.exe", &valide), Some("code.exe".to_string()));
        // Case-insensitive nel confronto (come tutto il resto del
        // modulo) — ritorna la stringa così come fornita dal chiamante,
        // non la forma canonica dell'elenco valido: in pratica non
        // importa, gli identificativi validi sono sempre già minuscoli
        // (vedi tutte_le_app_conosciute/lowercase applicato negli eventi
        // finestra), e ogni confronto successivo resta comunque
        // case-insensitive.
        assert_eq!(normalizza_app("CODE.EXE", &valide), Some("CODE.EXE".to_string()));
    }

    #[test]
    fn normalizza_app_taglia_il_nome_leggibile_ripetuto_dal_modello() {
        // Bug reale trovato verificando dal vivo con una chiave Claude
        // vera (2026-08-12): il modello a volte ripete anche il nome
        // leggibile dato come contesto nel prompt invece del solo
        // identificativo grezzo. Qui si verifica che venga recuperato
        // correttamente invece di salvare la stringa sporca as-is.
        let valide = vec!["app.exe".to_string()];
        assert_eq!(
            normalizza_app("app.exe (TrackFlow)", &valide),
            Some("app.exe".to_string()),
            "formato con parentesi tonde"
        );
        assert_eq!(
            normalizza_app("app.exe  [solo per contesto, non usarlo come identificativo: TrackFlow]", &valide),
            Some("app.exe".to_string()),
            "formato con parentesi quadre"
        );
    }

    #[test]
    fn normalizza_app_rifiuta_identificativi_sconosciuti() {
        let valide = vec!["code.exe".to_string()];
        assert_eq!(normalizza_app("firefox.exe", &valide), None);
        assert_eq!(normalizza_app("firefox.exe (Firefox)", &valide), None);
    }

    #[test]
    fn app_gia_categorizzata_controlla_tutte_le_categorie() {
        let categorie = vec![
            AppCategory { name: "Lavoro".to_string(), apps: vec!["code.exe".to_string()] },
            AppCategory { name: "Svago".to_string(), apps: vec!["steam.exe".to_string()] },
        ];
        assert!(app_gia_categorizzata(&categorie, "code.exe"));
        assert!(app_gia_categorizzata(&categorie, "STEAM.EXE"));
        assert!(!app_gia_categorizzata(&categorie, "firefox.exe"));
    }
}

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

/// Tutte le app mai scoperte dal watcher icone (scansiona la cartella
/// icone condivisa — stesso elenco che alimenta i moduli "Top
/// Applications" ecc., vedi aw-watcher-app-icons-rust/src/main.rs) — usato
/// solo per il caso "nessuna categoria esiste ancora", per categorizzarle
/// tutte in un colpo solo invece che una alla volta ad ogni riavvio.
pub(crate) fn tutte_le_app_conosciute(app_data_dir: &Path) -> Vec<String> {
    let icons_dir = app_data_dir.join("app-icons");
    let Ok(entries) = std::fs::read_dir(&icons_dir) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter_map(|e| {
            let path = e.path();
            if path.extension().and_then(|x| x.to_str()) == Some("png") {
                path.file_stem().and_then(|s| s.to_str()).map(str::to_string)
            } else {
                None
            }
        })
        .collect()
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

/// Il vero ciclo tool-calling — nessuna cronologia persistita (a
/// differenza della chat, questo è un task una tantum, non una
/// conversazione): un unico messaggio iniziale con le categorie esistenti
/// + l'elenco da categorizzare, poi round di tool_use/tool_result finché
/// il modello si ferma o si supera il tetto di iterazioni.
async fn esegui_categorizzazione(
    config: &AiAgentConfig,
    categorie: &mut Vec<AppCategory>,
    da_categorizzare: &[String],
    nomi_leggibili: &HashMap<String, String>,
) -> Result<(), String> {
    let strumenti = definisci_strumenti();

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

    let messaggio_iniziale = format!(
        "Categorie esistenti:\n{elenco_categorie}\n\nApp da categorizzare (NON ancora assegnate a nessuna categoria). Il testo tra parentesi quadre, quando presente, è solo un aiuto per capire di che app si tratta — l'IDENTIFICATIVO ESATTO da passare a assegna_app_categoria è SEMPRE e SOLO la parte prima delle parentesi quadre:\n{elenco_app}"
    );

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
    app: String,
) {
    let Some(config) = agent::load_config(&app_data_dir) else {
        return;
    };
    if config.api_key.trim().is_empty() {
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
            (tutte_le_app_conosciute(&app_data_dir), true)
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

        if let Err(e) =
            esegui_categorizzazione(&config, &mut categorie, &da_categorizzare, &nomi_leggibili).await
        {
            log::error!("Categorizzazione AI fallita: {e}");
            return;
        }

        if let Err(e) = salva_categorie(&server, &categorie).await {
            log::error!("Salvataggio categorie fallito: {e}");
        }
    });
}

/// App conosciuta, con nome leggibile quando disponibile — usata
/// dall'interfaccia Impostazioni (sezione categorie, gestione manuale)
/// per mostrare l'elenco completo delle app da categorizzare/vedere già
/// categorizzate. Stessa identica fonte dati (`tutte_le_app_conosciute`)
/// già usata dalla categorizzazione automatica AI, per restare coerenti
/// fra le due — un'app che l'AI considera "conosciuta" è la stessa che
/// vede l'utente qui.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AppConosciuta {
    pub app: String,
    pub nome_leggibile: Option<String>,
}

#[tauri::command]
pub fn elenca_app_conosciute(app_handle: tauri::AppHandle) -> Vec<AppConosciuta> {
    use tauri::Manager;
    let Some(dir) = app_handle.try_state::<crate::AppDataDirState>() else {
        return Vec::new();
    };
    let mut app = tutte_le_app_conosciute(&dir.0);
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

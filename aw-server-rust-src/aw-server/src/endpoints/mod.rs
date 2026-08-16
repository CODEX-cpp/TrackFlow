use rust_embed::RustEmbed;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use gethostname::gethostname;
use rocket::fs::FileServer;
use rocket::http::ContentType;
use rocket::serde::json::Json;
use rocket::State;

use crate::config::AWConfig;

use aw_datastore::Datastore;
use aw_models::Info;

#[derive(RustEmbed)]
#[folder = "$AW_WEBUI_DIR"]
struct EmbeddedAssets;

pub struct AssetResolver {
    asset_path: Option<PathBuf>,
}

impl AssetResolver {
    pub fn new(asset_path: Option<PathBuf>) -> Self {
        Self { asset_path }
    }

    fn resolve(&self, file_path: &str) -> Option<Vec<u8>> {
        if let Some(asset_path) = &self.asset_path {
            let content = std::fs::read(asset_path.join(file_path));
            if let Ok(data) = content {
                return Some(data);
            }
        }
        Some(EmbeddedAssets::get(file_path)?.data.to_vec())
    }
}

// The Datastore is just a cheap handle to the DB worker thread (a crossbeam
// channel sender), which serializes all DB access internally. No mutex is
// needed here — wrapping it in one would serialize all HTTP requests, letting
// a slow query block every heartbeat.
pub struct ServerState {
    pub datastore: Datastore,
    pub asset_resolver: AssetResolver,
    pub device_id: String,
}

#[macro_use]
mod util;
mod apikey;
mod bucket;
mod cors;
mod export;
mod extension_cors;
mod hostcheck;
mod import;
mod query;
mod settings;

pub use util::HttpErrorJson;

#[get("/")]
fn root_index(state: &State<ServerState>) -> Option<(ContentType, Vec<u8>)> {
    get_file("index.html".into(), state)
}

#[get("/css/<file..>")]
fn root_css(file: PathBuf, state: &State<ServerState>) -> Option<(ContentType, Vec<u8>)> {
    get_file(Path::new("css").join(file), state)
}

#[get("/fonts/<file..>")]
fn root_fonts(file: PathBuf, state: &State<ServerState>) -> Option<(ContentType, Vec<u8>)> {
    get_file(Path::new("fonts").join(file), state)
}

#[get("/js/<file..>")]
fn root_js(file: PathBuf, state: &State<ServerState>) -> Option<(ContentType, Vec<u8>)> {
    get_file(Path::new("js").join(file), state)
}

#[get("/static/<file..>")]
fn root_static(file: PathBuf, state: &State<ServerState>) -> Option<(ContentType, Vec<u8>)> {
    get_file(Path::new("static").join(file), state)
}

// Le rotte /css, /js, /static sopra rispecchiano l'output della vecchia
// build webpack. Da quando la webui è passata a Vite, tutto (JS e CSS,
// nomi con hash) finisce invece sotto /assets/ — senza questa rotta,
// index.html si carica (root_index) ma ogni asset referenziato dà 404,
// Vue non si monta mai e la finestra resta bianca.
#[get("/assets/<file..>")]
fn root_assets(file: PathBuf, state: &State<ServerState>) -> Option<(ContentType, Vec<u8>)> {
    get_file(Path::new("assets").join(file), state)
}

// Vite (via vite-plugin-pwa) genera anche altri file sciolti alla radice
// di dist/ (registerSW.js, sw.js, workbox-*.js, manifest.webmanifest) non
// coperti da nessuna rotta sopra. Rank basso (= priorità minore): le
// rotte con percorso letterale già definite (favicon.ico, logo.png,
// dark.css, manifest.json) vengono comunque provate prima da Rocket.
#[get("/<file>", rank = 10)]
fn root_file(file: PathBuf, state: &State<ServerState>) -> Option<(ContentType, Vec<u8>)> {
    get_file(file, state)
}

#[get("/favicon.ico")]
fn root_favicon(state: &State<ServerState>) -> Option<(ContentType, Vec<u8>)> {
    get_file("favicon.ico".into(), state)
}

#[get("/dark.css")]
fn root_dark(state: &State<ServerState>) -> Option<(ContentType, Vec<u8>)> {
    get_file("dark.css".into(), state)
}

#[get("/logo.png")]
fn root_logo(state: &State<ServerState>) -> Option<(ContentType, Vec<u8>)> {
    get_file("logo.png".into(), state)
}

#[get("/manifest.json")]
fn root_manifest(state: &State<ServerState>) -> Option<(ContentType, Vec<u8>)> {
    get_file("manifest.json".into(), state)
}

#[get("/")]
fn server_info(config: &State<AWConfig>, state: &State<ServerState>) -> Json<Info> {
    #[allow(clippy::or_fun_call)]
    let hostname = gethostname().into_string().unwrap_or("unknown".to_string());
    const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

    Json(Info {
        hostname,
        version: format!("v{} (rust)", VERSION.unwrap_or("(unknown)")),
        testing: config.testing,
        device_id: state.device_id.clone(),
    })
}

// Mappa nome-modulo-personalizzato -> cartella su disco, condivisa (stesso
// Arc) con il lato Tauri (custom_modules.rs) che la ripopola ogni volta
// che l'utente apre il selettore "Aggiungi modulo -> Personalizzato" o
// preme "Aggiorna" — una sola route generica montata una volta sola
// all'avvio, che risolve il nome a runtime leggendo questa mappa, invece
// di dover rimontare l'intero Rocket per ogni nuova cartella scoperta
// (non supportato: la mount-table di Rocket è fissa dopo l'avvio). Vedi
// `custom_static` più sotto per il meccanismo "storico", a mount fisso,
// usato solo per la cartella interna "app-data".
pub type CustomPagesRegistry = std::sync::RwLock<HashMap<String, PathBuf>>;

#[get("/<name>/<file..>")]
fn custom_page(
    name: String,
    file: PathBuf,
    registry: &State<Arc<CustomPagesRegistry>>,
) -> Option<(ContentType, Vec<u8>)> {
    let cartella = {
        let mappa = registry.read().ok()?;
        mappa.get(&name)?.clone()
    };
    let file = if file.as_os_str().is_empty() {
        PathBuf::from("index.html")
    } else {
        file
    };
    let percorso = cartella.join(&file);

    // Protezione anti path-traversal: il file risolto deve restare
    // dentro la cartella del modulo (un `file..` con `..` letterali
    // potrebbe altrimenti uscirne).
    let base_canonica = std::fs::canonicalize(&cartella).ok()?;
    let file_canonico = std::fs::canonicalize(&percorso).ok()?;
    if !file_canonico.starts_with(&base_canonica) {
        return None;
    }

    let dati = std::fs::read(&file_canonico).ok()?;
    let content_type = file
        .extension()
        .and_then(OsStr::to_str)
        .and_then(ContentType::from_extension)
        .unwrap_or(ContentType::Bytes);
    Some((content_type, dati))
}

// Cartella dei modelli di visualizzazione per i watcher personalizzati
// (bundled come risorsa Tauri, dentro l'installazione dell'app — non in
// app_data_dir — così un aggiornamento dell'app aggiorna anche i modelli,
// richiesta esplicita dell'utente). A differenza di CustomPagesRegistry
// sopra, l'insieme dei modelli è fisso per una data versione dell'app
// (non scoperto/registrato a runtime), quindi qui basta il percorso
// della cartella base: `template_id` risolve direttamente la
// sottocartella, nessuna mappa serve.
pub struct WatcherTemplatesDir(pub PathBuf);

#[get("/<template_id>/<file..>")]
fn watcher_template_page(
    template_id: String,
    file: PathBuf,
    dir: &State<Arc<WatcherTemplatesDir>>,
) -> Option<(ContentType, Vec<u8>)> {
    let cartella = dir.0.join(&template_id);
    let file = if file.as_os_str().is_empty() {
        PathBuf::from("index.html")
    } else {
        file
    };
    let percorso = cartella.join(&file);

    // Stessa protezione anti path-traversal di custom_page sopra.
    let base_canonica = std::fs::canonicalize(&cartella).ok()?;
    let file_canonico = std::fs::canonicalize(&percorso).ok()?;
    if !file_canonico.starts_with(&base_canonica) {
        return None;
    }

    let dati = std::fs::read(&file_canonico).ok()?;
    let content_type = file
        .extension()
        .and_then(OsStr::to_str)
        .and_then(ContentType::from_extension)
        .unwrap_or(ContentType::Bytes);
    Some((content_type, dati))
}

fn get_file(file: PathBuf, state: &State<ServerState>) -> Option<(ContentType, Vec<u8>)> {
    let asset = state.asset_resolver.resolve(&file.display().to_string())?;

    let content_type = file
        .extension()
        .and_then(OsStr::to_str)
        .and_then(ContentType::from_extension)
        .unwrap_or(ContentType::Bytes);

    Some((content_type, asset))
}

pub fn build_rocket(
    server_state: ServerState,
    config: AWConfig,
    custom_pages_registry: Arc<CustomPagesRegistry>,
    watcher_templates_dir: Arc<WatcherTemplatesDir>,
) -> rocket::Rocket<rocket::Build> {
    info!(
        "Starting aw-server-rust at {}:{}",
        config.address, config.port
    );
    let cors = cors::cors(&config);
    let extension_cors = extension_cors::ExtensionCorsScope::new(&config);
    let hostcheck = hostcheck::HostCheck::new(&config);
    let apikey = apikey::ApiKeyCheck::new(&config);
    let custom_static = config.custom_static.clone();

    let mut rocket = rocket::custom(config.to_rocket_config())
        .attach(cors.clone())
        // Attached before the other request fairings so a blocked extension
        // request is rewritten to the 403 route before they inspect the path.
        .attach(extension_cors)
        .attach(hostcheck)
        .attach(apikey)
        .manage(cors)
        .manage(server_state)
        .manage(config)
        .manage(custom_pages_registry)
        .manage(watcher_templates_dir)
        .mount(
            "/",
            routes![
                root_index,
                root_favicon,
                root_fonts,
                root_css,
                root_js,
                root_static,
                root_assets,
                // custom static files
                root_dark,
                root_logo,
                root_manifest,
                root_file
            ],
        )
        .mount("/api/0/info", routes![server_info])
        .mount(
            "/api/0/buckets",
            routes![
                bucket::bucket_new,
                bucket::bucket_delete,
                bucket::buckets_get,
                bucket::bucket_get,
                bucket::bucket_events_get,
                bucket::bucket_events_create,
                bucket::bucket_events_heartbeat,
                bucket::bucket_event_count,
                bucket::bucket_events_get_single,
                bucket::bucket_events_delete_by_id,
                bucket::bucket_export
            ],
        )
        .mount("/api/0/query", routes![query::query])
        .mount(
            "/api/0/import",
            routes![import::bucket_import_json, import::bucket_import_form],
        )
        .mount("/api/0/export", routes![export::buckets_export])
        .mount(
            "/api/0/settings",
            routes![
                settings::setting_get,
                settings::setting_set,
                settings::setting_delete,
                settings::settings_get,
            ],
        )
        .mount("/", rocket_cors::catch_all_options_routes())
        .mount("/pages/custom", routes![custom_page])
        .mount("/pages/watcher-templates", routes![watcher_template_page]);

    // for each custom static directory, mount it at the given name
    for (name, dir) in custom_static {
        info!(
            "Serving /pages/{} custom static directory from {}",
            name, dir
        );
        rocket = rocket.mount(&format!("/pages/{name}"), FileServer::from(dir));
    }
    rocket
}

mod tests {
    #[test]
    fn test_filesystem_resolver() {
        let resolver = super::AssetResolver::new(Some(".".into()));

        let content = resolver.resolve("Cargo.toml").unwrap();

        assert!(String::from_utf8(content).unwrap().contains("aw-server"));
    }

    #[test]
    fn test_resolver_without_asset() {
        let resolver = super::AssetResolver::new(Some(".".into()));

        let content = resolver.resolve("Cargo.json");

        assert!(content.is_none());
    }
}

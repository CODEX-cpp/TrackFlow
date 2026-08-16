//! Moduli di visualizzazione personalizzati — HTML/CSS/JS scritti
//! dall'utente, mostrati in un iframe dentro una card di Home (vedi
//! CustomVisualization.vue). Ogni modulo vive in una propria cartella
//! sotto `app_data_dir/custom/modules/<id>/`, con un `index.html`
//! obbligatorio e un `manifest.json` opzionale (solo il titolo mostrato).
//!
//! Il meccanismo di servizio è una route Rocket generica, montata una
//! sola volta all'avvio (`/pages/custom/<nome>/<file..>`, vedi
//! aw-server-rust-src/aw-server/src/endpoints/mod.rs), che risolve il
//! nome a runtime leggendo un registro condiviso e mutabile
//! (`AppServer::custom_pages_registry`) — questa funzione lo
//! ripopola scansionando la cartella, invece di richiedere un riavvio
//! per ogni modulo nuovo (rilevamento "a caldo", vedi BLUEPRINT.md).

use std::path::{Path, PathBuf};

use tauri::Manager;

use crate::{AppDataDirState, AppServer};

const NOME_FILE_INDEX: &str = "index.html";
const NOME_FILE_MANIFEST: &str = "manifest.json";

#[derive(serde::Deserialize)]
struct ModuleManifest {
    #[serde(default)]
    title: Option<String>,
}

#[derive(serde::Serialize)]
pub struct ModuloPersonalizzatoInfo {
    id: String,
    title: String,
}

pub(crate) fn cartella_custom_modules(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("custom").join("modules")
}

fn scansiona(app_data_dir: &Path) -> Vec<(String, PathBuf, String)> {
    let base = cartella_custom_modules(app_data_dir);
    let Ok(voci) = std::fs::read_dir(&base) else {
        return Vec::new();
    };

    voci.filter_map(Result::ok)
        .filter(|v| v.path().is_dir())
        .filter(|v| v.path().join(NOME_FILE_INDEX).is_file())
        .map(|v| {
            let id = v.file_name().to_string_lossy().to_string();
            let cartella = v.path();
            let titolo = std::fs::read_to_string(cartella.join(NOME_FILE_MANIFEST))
                .ok()
                .and_then(|c| serde_json::from_str::<ModuleManifest>(&c).ok())
                .and_then(|m| m.title)
                .unwrap_or_else(|| id.clone());
            (id, cartella, titolo)
        })
        .collect()
}

/// Ricostruisce da zero il registro nome->cartella letto dalla route
/// `/pages/custom/<nome>/` — chiamata sia da `elenca_moduli_personalizzati`
/// (ogni volta che il selettore "Aggiungi modulo" si apre) sia dal
/// pulsante "Aggiorna" nelle nuove Impostazioni, così una cartella
/// aggiunta o rimossa mentre l'app è aperta si riflette subito, senza
/// bisogno di riavviare.
fn aggiorna_registro(app_handle: &tauri::AppHandle, trovati: &[(String, PathBuf, String)]) {
    let Some(server) = app_handle.try_state::<std::sync::Arc<AppServer>>() else {
        return;
    };
    let Ok(mut mappa) = server.custom_pages_registry.write() else {
        return;
    };
    mappa.clear();
    for (id, cartella, _) in trovati {
        mappa.insert(id.clone(), cartella.clone());
    }
}

#[tauri::command]
pub fn elenca_moduli_personalizzati(app_handle: tauri::AppHandle) -> Vec<ModuloPersonalizzatoInfo> {
    let Some(dir) = app_handle.try_state::<AppDataDirState>() else {
        return Vec::new();
    };
    let trovati = scansiona(&dir.0);
    aggiorna_registro(&app_handle, &trovati);
    trovati
        .into_iter()
        .map(|(id, _, title)| ModuloPersonalizzatoInfo { id, title })
        .collect()
}

#[tauri::command]
pub fn ricarica_moduli_personalizzati(app_handle: tauri::AppHandle) -> Result<(), String> {
    let dir = app_handle
        .try_state::<AppDataDirState>()
        .ok_or_else(|| "Cartella dati non ancora pronta, riprova tra poco".to_string())?;
    let trovati = scansiona(&dir.0);
    aggiorna_registro(&app_handle, &trovati);
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
        "modulo".to_string()
    } else {
        s
    }
}

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

const INDEX_INIZIALE: &str = r#"<!doctype html>
<html>
<head>
<meta charset="utf-8">
<style>
  body { font-family: sans-serif; padding: 1rem; }
</style>
</head>
<body>
  <p>Modulo personalizzato di esempio.</p>
  <p id="dati">Caricamento...</p>
  <script>
    // TrackFlow passa hostname/start/end come parametri nell'URL.
    const params = new URLSearchParams(location.search);
    const start = params.get('start');
    const end = params.get('end');

    // Esempio: interroga l'API REST locale, stessa origine, nessuna
    // chiave richiesta. Sostituisci l'URL con la query che ti serve.
    fetch('/api/0/buckets')
      .then(r => r.json())
      .then(buckets => {
        document.getElementById('dati').textContent =
          'Bucket disponibili: ' + Object.keys(buckets).join(', ');
      })
      .catch(() => {
        document.getElementById('dati').textContent = 'Impossibile leggere i dati.';
      });
  </script>
</body>
</html>
"#;

/// Scaffolding guidato: crea la cartella con un `index.html` minimo già
/// funzionante e un `manifest.json` col titolo scelto — l'utente lo apre
/// e lo modifica con l'editor che preferisce, nessuna procedura più
/// complessa richiesta.
#[tauri::command]
pub fn crea_modulo_personalizzato(app_handle: tauri::AppHandle, nome: String) -> Result<String, String> {
    let dir = app_handle
        .try_state::<AppDataDirState>()
        .ok_or_else(|| "Cartella dati non ancora pronta, riprova tra poco".to_string())?;
    let base = cartella_custom_modules(&dir.0);
    let (_id, cartella) = crea_cartella_univoca(&base, &nome)?;

    std::fs::write(cartella.join(NOME_FILE_INDEX), INDEX_INIZIALE).map_err(|e| e.to_string())?;
    std::fs::write(
        cartella.join(NOME_FILE_MANIFEST),
        serde_json::json!({ "title": nome }).to_string(),
    )
    .map_err(|e| e.to_string())?;

    Ok(cartella.to_string_lossy().to_string())
}

#[tauri::command]
pub fn apri_cartella_custom_modulo(app_handle: tauri::AppHandle, id: String) -> Result<(), String> {
    let dir = app_handle
        .try_state::<AppDataDirState>()
        .ok_or_else(|| "Cartella dati non ancora pronta, riprova tra poco".to_string())?;
    crate::folder_shortcuts::apri_in_esplora_risorse(&cartella_custom_modules(&dir.0).join(id))
}

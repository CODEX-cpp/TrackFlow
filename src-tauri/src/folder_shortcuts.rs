//! Scorciatoie "Apri cartella X" (Impostazioni → Sviluppatore) — invece
//! di doversele andare a cercare a mano in `%LOCALAPPDATA%` (fatto più
//! volte in questa sessione di sviluppo), un pulsante che le apre
//! direttamente in Esplora risorse.

use std::path::{Path, PathBuf};

use tauri::Manager;

fn apri_in_esplora_risorse(path: &Path) -> Result<(), String> {
    // create_dir_all invece di limitarsi a controllare che esista: la
    // cartella di configurazione di aw-watcher-afk, in particolare, non
    // esiste finché l'utente non l'ha mai personalizzata — senza
    // questo il pulsante fallirebbe silenziosamente al primo utilizzo.
    std::fs::create_dir_all(path).map_err(|e| e.to_string())?;
    std::process::Command::new("explorer")
        .arg(path)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn apri_cartella_dati(app_handle: tauri::AppHandle) -> Result<(), String> {
    let dir = app_handle
        .try_state::<crate::AppDataDirState>()
        .ok_or_else(|| "Cartella dati non ancora pronta, riprova tra poco".to_string())?;
    apri_in_esplora_risorse(&dir.0)
}

#[tauri::command]
pub fn apri_cartella_log(app_handle: tauri::AppHandle) -> Result<(), String> {
    let dir = app_handle.path().app_log_dir().map_err(|e| e.to_string())?;
    apri_in_esplora_risorse(&dir)
}

/// Stesso percorso letto/scritto da aw-watcher-afk-rust — vedi il
/// commento su config_file_path() in aw-watcher-afk-rust/src/main.rs
/// per il perché è annidata due volte ("activitywatch" sia come
/// "author" che come "appname" per platformdirs).
#[tauri::command]
pub fn apri_cartella_config_afk() -> Result<(), String> {
    let dir = std::env::var("LOCALAPPDATA")
        .map(PathBuf::from)
        .map_err(|_| "Variabile d'ambiente LOCALAPPDATA non impostata".to_string())?
        .join("activitywatch")
        .join("activitywatch")
        .join("aw-watcher-afk");
    apri_in_esplora_risorse(&dir)
}

/// Cartella dell'eseguibile in esecuzione — in una build debug (l'unica
/// che esiste oggi) è la STESSA cartella dove vivono anche tutti gli
/// eseguibili sidecar dei watcher (app.exe, aw-watcher-afk.exe,
/// aw-watcher-window.exe, ...), verificato controllando
/// `target/debug/` durante la costruzione di questo pulsante — Tauri li
/// risolve a runtime relativi alla cartella dell'app, non da
/// `src-tauri/binaries/` (quella è solo la sorgente usata da build.rs).
#[tauri::command]
pub fn apri_cartella_watcher() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let dir = exe
        .parent()
        .ok_or_else(|| "Impossibile risolvere la cartella dell'eseguibile".to_string())?;
    apri_in_esplora_risorse(dir)
}

/// Cartella del vero database SQLite (`sqlite.db`) usato da aw-server —
/// stesso percorso restituito da `aw_server::dirs::get_data_dir()`, la
/// funzione che `build_app_server()` in lib.rs usa già per calcolare
/// `db_path`. Diversa da "Cartella dati" qui sopra: quella è la
/// cartella scrivibile di TrackFlow (icone, screenshot, mappature),
/// questa è quella di ActivityWatch upstream (`%APPDATA%\activitywatch\
/// aw-server-rust\`, non `%LOCALAPPDATA%`) da cui arriva `aw_server`
/// come libreria.
#[tauri::command]
pub fn apri_cartella_database() -> Result<(), String> {
    let dir = aw_server::dirs::get_data_dir()
        .map_err(|_| "Impossibile risolvere la cartella del database".to_string())?;
    apri_in_esplora_risorse(&dir)
}

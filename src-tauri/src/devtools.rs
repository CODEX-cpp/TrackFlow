//! Comando Tauri per aprire gli strumenti di sviluppo (DevTools) su
//! richiesta, dal toggle "DevTools" nella sezione Sviluppatore delle
//! Impostazioni (vedi DeveloperSettings.vue).
//!
//! Non esiste un comando "chiudi" corrispondente: `close_devtools()` di
//! Tauri/wry è un no-op su Windows (verificato leggendo
//! wry-0.55.1/src/webview2/mod.rs — `pub fn close_devtools(&self) {}`),
//! quindi non c'è modo di forzare la chiusura di una finestra DevTools
//! già aperta da codice. Il blocco quando il toggle è spento avviene
//! invece lato JS (vedi src/util/devtoolsGuard.ts): tasto destro e F12/
//! Ctrl+Shift+I vengono intercettati e bloccati, quindi semplicemente
//! non c'è più modo di *aprirli* nell'app — se erano già aperti restano
//! aperti finché l'utente non li chiude a mano.

use tauri::Manager;

// `open_devtools()` richiede la feature cargo "devtools" del crate
// tauri (vedi Cargo.toml) per funzionare anche nelle build release, non
// solo in debug — senza quella feature era un no-op silenzioso in
// release, e F12/tasto destro nativi di WebView2 restavano comunque
// disabilitati a prescindere dal toggle "DevTools" nelle Impostazioni
// (bug reale trovato mentre si testava un build release: il toggle
// diceva "acceso" ma non apriva nulla).
#[tauri::command]
pub fn apri_devtools(app_handle: tauri::AppHandle) {
    if let Some(window) = app_handle.get_webview_window("main") {
        window.open_devtools();
    }
}

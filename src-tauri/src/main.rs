// Nessuna console, in debug come in release — richiesta esplicita
// dell'utente: il template di default di Tauri la nasconde solo in
// release (`cfg_attr(not(debug_assertions), ...)`), lasciandola aperta
// per ogni lancio del binario debug usato per questa sessione di test
// dal vivo. I log restano comunque disponibili (file + pannello "Log"
// in Impostazioni → Sviluppatore, vedi tauri-plugin-log in lib.rs), non
// serve la console per vederli.
#![windows_subsystem = "windows"]

fn main() {
  app_lib::run();
}

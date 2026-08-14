// Nomi "client" (campo `client` di ogni bucket, impostato dal watcher
// stesso quando crea il bucket — vedi CLIENT_NAME nei sorgenti Rust di
// ogni aw-watcher-*-rust, e trackflow-voispeed in src-tauri/src/
// voispeed.rs) di tutti i watcher che questa app conosce e gestisce
// come sidecar/moduli interni. Usato dal pannello "Stato watcher"
// (Impostazioni → Sviluppatore) per distinguere questi da bucket
// "personalizzati" — un watcher scritto a mano dall'utente (come i
// primi prototipi di questo stesso progetto), o un'estensione ufficiale
// ActivityWatch per il browser: non essendo processi che avviamo/
// fermiamo noi, non c'è nessuno stato di processo da mostrare per loro,
// solo i dati del bucket (ultimo evento, tipo).
export const KNOWN_WATCHER_CLIENTS = new Set([
  'aw-watcher-afk',
  'aw-watcher-window',
  'aw-watcher-vpn',
  'aw-watcher-claude-code',
  'aw-watcher-screenshot',
  'aw-watcher-tray',
  'aw-watcher-vscode',
  'aw-watcher-excel',
  'trackflow-voispeed',
]);

// Stato acceso/spento dei watcher (sottomenu "Moduli" della tray di
// Windows) — quel toggle è nativo, gestito solo lato Rust
// (modules-config.json, vedi lib.rs), senza nessuna UI nella webui.
// Serve comunque leggerlo da qui per decidere se nascondere le corsie
// vuote della Timeline (VPN/Claude/VSCode) quando l'utente non usa
// affatto quella funzionalità — vedi shouldShowLane() in
// HomeTimelineSection.vue.
//
// Nessuna modifica lato Rust necessaria: modules-config.json vive già
// dentro app_data_dir, e quella cartella è già servita in sola lettura
// su /pages/app-data/ (stesso meccanismo di appIconColors.json/
// appAutoNames.json in appNames.ts).
const APP_DATA_URL = '/pages/app-data';

let modulesConfig: Record<string, boolean> = {};

// Richiamata una volta all'avvio (vedi App.vue, stesso punto di
// refreshDynamicAppData()) — fallisce silenziosamente se il file non
// esiste ancora (prima esecuzione: tutti i moduli sono considerati
// abilitati di default, stesso comportamento del lato Rust).
export async function refreshModulesConfig(): Promise<void> {
  await fetch(`${APP_DATA_URL}/modules-config.json`)
    .then(r => (r.ok ? r.json() : null))
    .then(data => {
      if (data) modulesConfig = data;
    })
    .catch(() => undefined);
}

// Di default true (non ancora spento esplicitamente) — stesso
// comportamento di load_modules_config() lato Rust (lib.rs), tranne
// aw-watcher-tray che parte spento di default lì; qui non serve
// replicare quell'eccezione perché nessuna corsia della Timeline
// dipende da quel watcher.
export function isWatcherEnabled(name: string): boolean {
  return modulesConfig[name] ?? true;
}

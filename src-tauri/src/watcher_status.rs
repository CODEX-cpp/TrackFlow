//! Pannello "Stato watcher" (Impostazioni → Sviluppatore → Modalità
//! sviluppatore). Copre solo i moduli "riconosciuti" — quelli in
//! `ALL_MODULES`, gestiti come sidecar da questo stesso processo Tauri
//! (o virtuali com VoiSpeed/categorizzazione AI). Eventuali watcher
//! personalizzati (bucket che esistono sul server ma il cui `client`
//! non corrisponde a nessuno di questi nomi — es. uno scritto a mano
//! dall'utente, come i primi prototipi di questo stesso progetto) NON
//! passano da qui: non essendo processi che avviamo/fermiamo noi, non
//! c'è nulla da riportare lato Rust — vengono rilevati interamente lato
//! webui incrociando l'elenco bucket del server (client sconosciuto =
//! "personalizzato"), vedi ActivePatternSettings-style pattern già
//! usato altrove per "elenca ciò che il server conosce davvero".

use std::collections::HashMap;
use std::sync::Arc;

use tauri::{AppHandle, Manager};

use crate::{
    save_modules_config, spawn_watcher, stop_watcher, AppDataDirState, AppServer, IconsHandle,
    ModuleMenuItems, ModulesConfigState, SidecarProcesses, TrayLabelItems, ALL_MODULES,
    MODULES_CONFIG_FILE,
};

#[derive(Debug, Clone, serde::Serialize)]
pub struct WatcherStatus {
    pub name: String,
    pub label: String,
    /// Scelta persistita dall'utente (menu Moduli della tray / prossimo
    /// avvio) — indipendente da `running`: un avvio manuale "solo
    /// questa sessione" lascia questo a `false` di proposito.
    pub enabled_in_config: bool,
    pub running: bool,
    pub pid: Option<u32>,
    /// `false` solo per "ai-categorization" — un flag letto dal vivo ad
    /// ogni evento, non un processo: niente PID, niente pulsanti
    /// avvia/riavvia/ferma per quella riga.
    pub has_process: bool,
}

#[tauri::command]
pub fn stato_watcher(app_handle: AppHandle) -> Vec<WatcherStatus> {
    let modules_config = app_handle.try_state::<ModulesConfigState>();
    let processes = app_handle.try_state::<SidecarProcesses>();
    let icons = app_handle.try_state::<IconsHandle>();

    ALL_MODULES
        .iter()
        // Solo l'etichetta italiana qui — pannello di diagnostica per
        // sviluppatori (Impostazioni → Sviluppatore), non il menu della
        // tray (che invece sceglie tra le due, vedi lib.rs) — fuori
        // scopo dell'issue GitHub #2 che ha motivato le due etichette.
        .map(|(name, label, _label_en)| {
            let enabled_in_config = modules_config
                .as_ref()
                .map(|c| *c.0.lock().unwrap().get(*name).unwrap_or(&true))
                .unwrap_or(true);

            let (running, pid, has_process) = match *name {
                "aw-watcher-app-icons" => {
                    let guard = icons.as_ref().and_then(|i| i.lock().ok());
                    let child = guard.as_ref().and_then(|g| g.as_ref());
                    (child.is_some(), child.map(|c| c.pid()), true)
                }
                "aw-watcher-voispeed" => (crate::voispeed::is_polling_running(&app_handle), None, true),
                "ai-categorization" => (enabled_in_config, None, false),
                _ => {
                    let guard = processes.as_ref().map(|p| p.0.lock().unwrap());
                    let child = guard.as_ref().and_then(|g| g.get(*name));
                    (child.is_some(), child.map(|c| c.pid()), true)
                }
            };

            WatcherStatus {
                name: (*name).to_string(),
                label: (*label).to_string(),
                enabled_in_config,
                running,
                pid,
                has_process,
            }
        })
        .collect()
}

/// Rinomina a runtime le voci del menu tray (issue GitHub #2 — vedi
/// TrayLabelItems in lib.rs) quando l'utente cambia lingua dalle
/// Impostazioni mentre l'app è già aperta — invocato dal frontend
/// (stores/settings.ts) ogni volta che `locale` viene salvata, non solo
/// all'avvio. `set_text` fallisce solo se l'icona tray non esiste più
/// (finestra in chiusura) — ignorato, niente da fare in quel caso.
#[tauri::command]
pub fn aggiorna_lingua_tray(app_handle: AppHandle, lingua: String) {
    let inglese = lingua == "en";

    if let Some(items) = app_handle.try_state::<TrayLabelItems>() {
        let (show_hide_label, quit_label, modules_label) =
            if inglese { ("Show/Hide", "Quit", "Modules") } else { ("Mostra/Nascondi", "Esci", "Moduli") };
        let _ = items.show_hide.set_text(show_hide_label);
        let _ = items.quit.set_text(quit_label);
        let _ = items.modules_submenu.set_text(modules_label);
    }

    if let Some(module_items) = app_handle.try_state::<ModuleMenuItems>() {
        let guard = module_items.0.lock().unwrap();
        for (name, label_it, label_en) in ALL_MODULES {
            if let Some(item) = guard.get(*name) {
                let _ = item.set_text(if inglese { *label_en } else { *label_it });
            }
        }
    }
}

/// Avvia un watcher SENZA toccare modules-config.json — "solo per
/// questa sessione", richiesta esplicita: se era impostato per non
/// avviarsi, al prossimo riavvio dell'app deve restare spento finché
/// l'utente non lo riaccende per davvero dal menu Moduli.
#[tauri::command]
pub fn avvia_watcher_sessione(app_handle: AppHandle, name: String) {
    let server = app_handle.try_state::<Arc<AppServer>>();
    let dir = app_handle.try_state::<AppDataDirState>();
    let icons_stdin = app_handle.try_state::<IconsHandle>();
    let (Some(server), Some(dir), Some(icons_stdin)) = (server, dir, icons_stdin) else {
        log::warn!("Impossibile avviare {name}: server non ancora pronto, riprova tra poco");
        return;
    };
    let hostname = gethostname::gethostname().to_string_lossy().to_string();
    spawn_watcher(&app_handle, &name, server.inner().clone(), hostname, &dir.0, icons_stdin.inner().clone());
}

/// Ferma un watcher avviato manualmente per la sessione corrente,
/// senza persistere nulla (era già disattivato in config, resta tale).
#[tauri::command]
pub fn ferma_watcher_sessione(app_handle: AppHandle, name: String) {
    stop_watcher(&app_handle, &name);
}

/// Riavvio vero e proprio (watcher che dovrebbe essere attivo ma non lo
/// è più, es. crash) — ferma se serve e riavvia, sempre senza toccare
/// modules-config.json.
#[tauri::command]
pub fn riavvia_watcher(app_handle: AppHandle, name: String) {
    stop_watcher(&app_handle, &name);
    avvia_watcher_sessione(app_handle, name);
}

/// Comando Tauri per il toggle di un watcher integrato nella pagina
/// Watchers (Buckets.vue) — riusa esattamente la stessa `imposta_modulo`
/// del menu Moduli della tray qui sotto, nessuna logica duplicata: il
/// toggle nella pagina, il click nel menu tray e il dialogo di primo
/// avvio finiscono tutti nello stesso posto.
#[tauri::command]
pub fn imposta_modulo_watcher(app_handle: AppHandle, nome: String, attivo: bool) {
    imposta_modulo(&app_handle, &nome, attivo);
}

/// Accende/spegne un modulo PER DAVVERO (persiste su modules-config.json,
/// aggiorna la spunta nel menu Moduli della tray, avvia/ferma il
/// processo) — stessa identica logica usata dal click sul menu Moduli,
/// estratta qui perché serve anche al dialogo di primo avvio (vedi
/// imposta_moduli_iniziali sotto). A differenza di avvia/ferma_watcher_
/// sessione qui sopra, questa è la scelta "vera", che sopravvive al
/// riavvio dell'app.
pub(crate) fn imposta_modulo(app_handle: &AppHandle, module_name: &str, enabled: bool) {
    {
        let config_state = app_handle.state::<ModulesConfigState>();
        let mut config = config_state.0.lock().unwrap();
        config.insert(module_name.to_string(), enabled);
        if let Some(dir) = app_handle.try_state::<AppDataDirState>() {
            save_modules_config(&dir.0, &config);
        }
    }

    if let Some(items) = app_handle.try_state::<ModuleMenuItems>() {
        if let Some(item) = items.0.lock().unwrap().get(module_name) {
            let _ = item.set_checked(enabled);
        }
    }

    if enabled {
        let server = app_handle.try_state::<Arc<AppServer>>();
        let dir = app_handle.try_state::<AppDataDirState>();
        let icons_stdin = app_handle.try_state::<IconsHandle>();
        if let (Some(server), Some(dir), Some(icons_stdin)) = (server, dir, icons_stdin) {
            let hostname = gethostname::gethostname().to_string_lossy().to_string();
            spawn_watcher(
                app_handle,
                module_name,
                server.inner().clone(),
                hostname,
                &dir.0,
                icons_stdin.inner().clone(),
            );
        } else {
            log::warn!("Impossibile avviare {module_name}: server non ancora pronto, riprova tra poco");
        }
    } else {
        stop_watcher(app_handle, module_name);
    }
}

/// Chiamato dal popup di primo avvio (FirstRunWatcherSetup.vue) quando
/// l'utente conferma la sua scelta — applica tutte le scelte in un colpo
/// solo. L'app parte già con i moduli di default attivi (vedi
/// load_modules_config()), quindi qui serve davvero fermare quelli che
/// l'utente ha deselezionato, non solo avviare quelli scelti.
#[tauri::command]
pub fn imposta_moduli_iniziali(app_handle: AppHandle, moduli: HashMap<String, bool>) {
    for (name, enabled) in moduli {
        imposta_modulo(&app_handle, &name, enabled);
    }
}

/// true se l'utente ha già visto/confermato il popup di primo avvio in
/// una sessione precedente (o adesso) — la presenza stessa di
/// modules-config.json basta come segnale: viene scritto solo da
/// imposta_moduli_iniziali sopra o da un cambio manuale nel menu Moduli,
/// mai creato automaticamente al semplice avvio (load_modules_config()
/// legge i default in memoria senza mai salvarli su disco finché
/// l'utente non sceglie qualcosa esplicitamente).
#[tauri::command]
pub fn moduli_gia_configurati(app_handle: AppHandle) -> bool {
    match app_handle.try_state::<AppDataDirState>() {
        Some(dir) => dir.0.join(MODULES_CONFIG_FILE).exists(),
        None => false,
    }
}

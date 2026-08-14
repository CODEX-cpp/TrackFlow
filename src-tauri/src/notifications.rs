//! Invio di toast di sistema — helper condiviso, usato sia dalla
//! notifica VPN (vpn_notify.rs) sia da qualunque altra notifica generica
//! (vedi invia_notifica_generica sotto, chiamata dal frontend per es.
//! per gli avvisi di budget/scadenza progetto in Progetti.vue).

use tauri::{AppHandle, Manager};

/// Mostra un toast di sistema con titolo/corpo dati. Su Windows costruisce
/// il Toast direttamente con tauri-winrt-notification invece di passare
/// da notify_rust — due motivi, trovati per tentativi mentre si
/// sistemava la notifica VPN (prima notifica implementata in questo
/// progetto):
/// 1) notify_rust::Notification::app_id() è indispensabile: al suo
///    interno tauri_plugin_notification::NotificationBuilder (vedi
///    desktop.rs del plugin) imposta l'System.AppUserModel.ID SOLO se
///    l'eseguibile NON gira da target/debug o target/release — mai
///    durante lo sviluppo. Risultato osservato: Windows attribuiva il
///    toast a qualunque identità capitasse (es. "PowerShell"). Qui lo si
///    imposta sempre, incondizionatamente.
/// 2) notify_rust::Notification::image_path() — che sembrerebbe l'ovvio
///    modo per mostrare l'icona dell'app nel toast — su Windows chiama
///    solo Toast::image() di questa stessa libreria, che imposta
///    un'immagine generica nel CORPO del toast, non l'icona in alto a
///    sinistra. Quella è Toast::icon() (parametro "appLogoOverride"),
///    mai esposto da notify_rust: per usarlo davvero va costruito il
///    Toast a mano, come qui sotto.
pub fn invia_toast(app_handle: &AppHandle, titolo: &str, corpo: &str) {
    #[cfg(target_os = "windows")]
    {
        use tauri_winrt_notification::{IconCrop, Toast};

        let identifier = app_handle.config().identifier.clone();
        let mut toast = Toast::new(&identifier).title(titolo).text1(corpo);

        if let Ok(resource_dir) = app_handle.path().resource_dir() {
            let icon_path = resource_dir.join("icons").join("notification-icon.png");
            if icon_path.exists() {
                // Toast::icon() inserisce il percorso così com'è dentro
                // un URI "file:///{}" — con i backslash nativi di
                // Windows quell'URI non è valido, e l'icona viene
                // scartata in silenzio (nessun errore, semplicemente non
                // compare). Serve passare i forward slash.
                let icon_path_uri = icon_path.to_string_lossy().replace('\\', "/");
                toast = toast.icon(std::path::Path::new(&icon_path_uri), IconCrop::Square, "TrackFlow");
            } else {
                log::warn!("Notifica: icona non trovata in {icon_path:?}");
            }
        }

        match toast.show() {
            Ok(_) => log::info!("Notifica inviata (show() ha ritornato Ok)."),
            Err(e) => log::error!("Notifica: show() ha fallito: {e:?}"),
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let mut notifica = notify_rust::Notification::new();
        notifica.summary(titolo).body(corpo);
        match notifica.show() {
            Ok(_) => log::info!("Notifica inviata (show() ha ritornato Ok)."),
            Err(e) => log::error!("Notifica: show() ha fallito: {e}"),
        }
    }
}

/// Notifica generica invocabile dal frontend — usata per tutto ciò che
/// non ha bisogno di rilevamento lato Rust (a differenza della VPN, che
/// deve girare anche a finestra chiusa/in tray): la webview resta viva
/// anche a finestra nascosta (nasconde, non distrugge — vedi il resto
/// dell'app), quindi un controllo periodico lato JS con questo comando
/// come "invia" basta. Primo caso d'uso: avviso di budget ore/scadenza
/// superati in Progetti.vue (vedi projectTimerMixin.ts).
#[tauri::command]
pub fn invia_notifica_generica(app_handle: AppHandle, titolo: String, corpo: String) {
    invia_toast(&app_handle, &titolo, &corpo);
}

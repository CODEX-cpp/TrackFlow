use std::collections::HashSet;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};

use crate::{vpn_mapping, AppServer, SidecarProcesses};

/// Ogni quanto controllare la sessione VPN più recente. Prima 150s, sul
/// presupposto (rivelatosi falso — bug reale segnalato dall'utente) che
/// il campo "cliente" fosse sempre già corretto dal momento della
/// connessione, quindi un intervallo breve non servisse. In realtà il
/// watcher poteva impiegare fino a 30 minuti a ricaricare una mappatura
/// appena registrata (vedi il fix in aw-watcher-vpn-rust/src/main.rs,
/// `sincronizza_mapping_se_serve`), quindi la notifica finiva per
/// arrivare tardi o per un client già associato nel frattempo. Con
/// quella causa risolta (mappatura ricaricata quasi subito, ~15s), non
/// serve più un canale diretto watcher→notifica: basta un intervallo
/// breve qui, stesso ordine di grandezza del poll del watcher.
const INTERVALLO_CONTROLLO_SECONDI: u64 = 20;

/// Indirizzi/nomi grezzi (il valore del campo "cliente" quando il
/// watcher non ha trovato una mappatura) già notificati in questa
/// sessione dell'app. Evita di rimandare la stessa notifica ogni
/// INTERVALLO_CONTROLLO_SECONDI finché la sessione VPN resta aperta e
/// non mappata. Volutamente solo in memoria (si resetta ad ogni riavvio
/// di TrackFlow): non serve persistenza, l'utente la vede comunque la
/// prima volta che succede davvero.
struct StatoNotificheVpn {
    gia_notificati: StdMutex<HashSet<String>>,
}

/// Avvia il controllo periodico in background. Va richiamata una sola
/// volta, dopo che il server in-process è pronto (stesso momento in cui
/// parte il polling di VoiSpeed, vedi lib.rs).
pub fn avvia_controllo(app_handle: AppHandle, server: Arc<AppServer>) {
    app_handle.manage(Arc::new(StatoNotificheVpn {
        gia_notificati: StdMutex::new(HashSet::new()),
    }));

    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(INTERVALLO_CONTROLLO_SECONDI)).await;
            controlla_sessione_corrente(&app_handle, &server).await;
        }
    });
}

async fn controlla_sessione_corrente(app_handle: &AppHandle, server: &AppServer) {
    // Se il watcher VPN non sta girando in questo momento (spento dal
    // menu Moduli, o non ancora avviato) non c'è nulla da controllare —
    // il ciclo resta comunque vivo e riprenderà da solo al giro
    // successivo se nel frattempo viene riacceso.
    let vpn_attivo = app_handle
        .state::<SidecarProcesses>()
        .0
        .lock()
        .map(|processi| processi.contains_key("aw-watcher-vpn"))
        .unwrap_or(false);
    if !vpn_attivo {
        return;
    }

    // Prende l'evento più recente del bucket vpn-sessions. Grazie al
    // meccanismo di heartbeat-merge di aw-server, una sessione ancora
    // aperta è già interrogabile qui (non serve aspettare la
    // disconnessione, vedi discussione in chat) — l'evento più recente
    // rappresenta sempre la sessione corrente o l'ultima chiusa.
    //
    // Bug reale trovato testando con un evento finto: `sort_by_timestamp`
    // ordina dal più VECCHIO (aw-transform::sort_by_timestamp usa
    // `sort_by_key` naturale, crescente) e `limit_events(events, 1)`
    // prende i primi `limit` elementi — cioè il più vecchio nella
    // finestra di 24h, l'esatto opposto di quello che il commento (e il
    // nome della funzione) promettevano. Non esiste un `sort_by_timestamp`
    // discendente nel linguaggio di query, quindi si ordina crescente
    // come sempre e si prende l'ULTIMO elemento lato Rust (vedi `.last()`
    // sotto) invece di limitare a 1 dentro la query stessa.
    let ora = chrono::Utc::now();
    let timeperiod = format!("{}/{}", (ora - chrono::Duration::hours(24)).to_rfc3339(), ora.to_rfc3339());
    let query = vec![
        "events = flood(query_bucket(\"vpn-sessions\"));".to_string(),
        "events = sort_by_timestamp(events);".to_string(),
        "RETURN = events;".to_string(),
    ];
    let risposta = match server.query(vec![timeperiod], query).await {
        Ok(r) => r,
        Err(e) => {
            log::warn!("Controllo notifiche VPN: query fallita: {e}");
            return;
        }
    };

    let cliente = risposta
        .as_array()
        .and_then(|periodi| periodi.first())
        .and_then(|eventi| eventi.as_array())
        .and_then(|eventi| eventi.last())
        .and_then(|evento| evento.get("data"))
        .and_then(|data| data.get("cliente"))
        .and_then(|c| c.as_str())
        .map(|s| s.to_string());

    let Some(cliente) = cliente else {
        return;
    };

    // "Non mappato" = il valore scritto dal watcher nel campo "cliente"
    // non corrisponde al nome di NESSUna mappatura conosciuta (né
    // automatica OpenVPN né manuale ZyWALL) — quando manca una
    // mappatura, il watcher scrive l'indirizzo grezzo così com'è, vedi
    // vpn_mapping.rs / aw-watcher-vpn-rust.
    let nomi_mappati: HashSet<String> = match vpn_mapping::leggi_mapping_vpn(app_handle.clone()) {
        Ok(voci) => voci.into_iter().map(|v| v.cliente).collect(),
        Err(e) => {
            log::warn!("Controllo notifiche VPN: lettura mapping fallita: {e}");
            return;
        }
    };
    if nomi_mappati.contains(&cliente) {
        return;
    }

    if !regola_vpn_abilitata(server).await {
        return;
    }

    let stato = app_handle.state::<Arc<StatoNotificheVpn>>();
    {
        let mut gia_notificati = stato.gia_notificati.lock().unwrap();
        if gia_notificati.contains(&cliente) {
            return;
        }
        gia_notificati.insert(cliente.clone());
    }

    invia_notifica(app_handle, &cliente);
}

/// Legge settingsStore.notifyRules (Impostazioni → Notifiche, scritto
/// dal frontend — vedi util/notifyRules.ts) per sapere se l'utente ha
/// disattivato la regola "Cliente VPN sconosciuto" dalla UI. Nessuna
/// regola di tipo "vpn" trovata (utente mai entrato in quella sezione,
/// o array assente) = abilitata di default, per non silenziare questa
/// notifica per chi non ha mai toccato le nuove Impostazioni Notifiche.
async fn regola_vpn_abilitata(server: &AppServer) -> bool {
    let Some(valore) = server.get_setting("notifyRules").await else {
        return true;
    };
    let Some(regole) = valore.as_array() else {
        return true;
    };
    match regole.iter().find(|r| r.get("type").and_then(|t| t.as_str()) == Some("vpn")) {
        Some(regola) => regola.get("enabled").and_then(|e| e.as_bool()).unwrap_or(true),
        None => true,
    }
}

fn invia_notifica(app_handle: &AppHandle, cliente: &str) {
    log::info!("Invio notifica VPN per cliente non mappato: {cliente}");
    let corpo = format!(
        "Rilevata una connessione VPN da \"{cliente}\", non ancora mappata a nessun cliente. Apri TrackFlow \u{2192} Impostazioni \u{2192} Integrazioni per assegnarla."
    );
    crate::notifications::invia_toast(app_handle, "Sessione VPN senza cliente associato", &corpo);

    // Naviga già ORA la webview sulla sezione giusta (anche se la
    // finestra è nascosta in tray) — vedi commento in App.vue sul
    // perché non possiamo affidarci al click sulla notifica stessa per
    // farlo (nessun activator custom senza pacchettizzare in MSIX).
    let _ = app_handle.emit("vpn-notifica-apri-impostazioni", "integrations");
}

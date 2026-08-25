//! Vista/modifica in Impostazioni del mapping indirizzo VPN → nome
//! cliente usato da `aw-watcher-vpn-rust` — vedi il commento in cima a
//! quel crate per il design completo. In breve: gli indirizzi OpenVPN
//! si risolvono da soli (letti dai profili salvati in OpenVPN Connect,
//! sezione "automatica" del file, riscritta dal watcher ad ogni giro),
//! le ZyWALL no (nessun profilo da cui leggere un nome) e finora
//! andavano scritte a mano nel file `client_mapping.txt` fuori
//! dall'app — richiesta esplicita dell'utente di poterlo fare da qui.
//!
//! Il file non lo tocchiamo mai per la parte automatica: la leggiamo
//! per mostrarla (sola lettura), ma quando si salva riscriviamo SOLO
//! tutto ciò che sta dopo il marcatore di fine sezione automatica,
//! lasciando quella sezione esattamente come l'ha lasciata l'ultimo
//! giro del watcher — stessa logica di `write_openvpn_section` in
//! aw-watcher-vpn-rust/src/main.rs, duplicata qui in piccolo perché
//! quel crate è un binario a sé (sidecar), non una libreria condivisa.

use std::path::Path;
use std::sync::Arc;

use tauri::{AppHandle, Manager};

use crate::AppServer;

/// Bucket delle sessioni VPN — vedi BUCKET_ID in aw-watcher-vpn-rust/
/// src/main.rs, duplicato qui per lo stesso motivo di SEZIONE_AUTO_INIZIO
/// sopra (crate separato, non condiviso).
const BUCKET_ID: &str = "vpn-sessions";

const NOME_FILE: &str = "client_mapping.txt";
const SEZIONE_AUTO_INIZIO: &str =
    "# === INIZIO SEZIONE AUTOMATICA (profili OpenVPN Connect - aggiornata da sola, non modificare qui sotto) ===";
const SEZIONE_AUTO_FINE: &str = "# === FINE SEZIONE AUTOMATICA ===";

#[derive(Debug, Clone, serde::Serialize)]
pub struct VoceMappingVpn {
    pub indirizzo: String,
    pub cliente: String,
    /// "openvpn" = letto in automatico dai profili OpenVPN Connect —
    /// l'indirizzo non è modificabile qui, ma il nome sì: digitarne uno
    /// diverso e salvare crea una sovrascrittura manuale "solo per
    /// TrackFlow" (il profilo OpenVPN vero non viene toccato). "manuale"
    /// = voce scritta a mano (nuova, es. ZyWALL, oppure una
    /// sovrascrittura di un nome automatico) — sia indirizzo che nome
    /// modificabili, rimovibile.
    pub origine: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct VoceMappingVpnInput {
    pub indirizzo: String,
    pub cliente: String,
}

fn parse_righe(testo: &str) -> Vec<(String, String)> {
    testo
        .lines()
        .filter_map(|riga| {
            let riga = riga.trim();
            if riga.is_empty() || riga.starts_with('#') || !riga.contains('=') {
                return None;
            }
            let (indirizzo, cliente) = riga.split_once('=')?;
            let indirizzo = indirizzo.trim();
            let cliente = cliente.trim();
            if indirizzo.is_empty() || cliente.is_empty() {
                return None;
            }
            Some((indirizzo.to_string(), cliente.to_string()))
        })
        .collect()
}

/// Divide il contenuto del file in (parte automatica INCLUSI i
/// marcatori, parte manuale) — se i marcatori non ci sono ancora (file
/// mai scritto dal watcher, o creato a mano prima che esistesse questo
/// meccanismo), tutto il contenuto è trattato come manuale, stessa
/// convenzione di `write_openvpn_section`.
fn dividi_sezioni(contenuto: &str) -> (String, String) {
    match contenuto.find(SEZIONE_AUTO_FINE) {
        Some(pos) => {
            let fine_marcatore = pos + SEZIONE_AUTO_FINE.len();
            let auto = contenuto[..fine_marcatore].to_string();
            let manuale = contenuto[fine_marcatore..].trim_start_matches(['\r', '\n']).to_string();
            (auto, manuale)
        }
        None => (String::new(), contenuto.to_string()),
    }
}

fn percorso_mapping(app_handle: &AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app_handle
        .try_state::<crate::AppDataDirState>()
        .ok_or_else(|| "Cartella dati non ancora pronta, riprova tra poco".to_string())?;
    Ok(dir.0.join(NOME_FILE))
}

#[tauri::command]
pub fn leggi_mapping_vpn(app_handle: AppHandle) -> Result<Vec<VoceMappingVpn>, String> {
    let percorso = percorso_mapping(&app_handle)?;
    let contenuto = std::fs::read_to_string(&percorso).unwrap_or_default();
    let (auto, manuale) = dividi_sezioni(&contenuto);

    // Le manuali vincono sempre sulle automatiche per lo stesso
    // indirizzo — stessa regola di `build_client_mapping` nel watcher.
    let mut mappa: std::collections::HashMap<String, (String, String)> = std::collections::HashMap::new();
    for (indirizzo, cliente) in parse_righe(&auto) {
        mappa.insert(indirizzo.clone(), (cliente, "openvpn".to_string()));
    }
    for (indirizzo, cliente) in parse_righe(&manuale) {
        mappa.insert(indirizzo.clone(), (cliente, "manuale".to_string()));
    }

    let mut voci: Vec<VoceMappingVpn> = mappa
        .into_iter()
        .map(|(indirizzo, (cliente, origine))| VoceMappingVpn { indirizzo, cliente, origine })
        .collect();
    voci.sort_by(|a, b| a.indirizzo.cmp(&b.indirizzo));
    Ok(voci)
}

/// Sostituisce SOLO la parte manuale del file con le voci passate,
/// lasciando la sezione automatica (se presente) esattamente come
/// l'aveva scritta l'ultimo giro del watcher.
#[tauri::command]
pub fn salva_mapping_vpn_manuale(app_handle: AppHandle, voci: Vec<VoceMappingVpnInput>) -> Result<(), String> {
    let percorso = percorso_mapping(&app_handle)?;
    let contenuto_esistente = std::fs::read_to_string(&percorso).unwrap_or_default();
    let (auto, _) = dividi_sezioni(&contenuto_esistente);

    let mut nuovo_contenuto = String::new();
    if !auto.is_empty() {
        nuovo_contenuto.push_str(&auto);
        nuovo_contenuto.push('\n');
    } else {
        nuovo_contenuto.push_str(SEZIONE_AUTO_INIZIO);
        nuovo_contenuto.push('\n');
        nuovo_contenuto.push_str(SEZIONE_AUTO_FINE);
        nuovo_contenuto.push('\n');
    }

    if !voci.is_empty() {
        nuovo_contenuto.push('\n');
        for voce in &voci {
            let indirizzo = voce.indirizzo.trim();
            let cliente = voce.cliente.trim();
            if indirizzo.is_empty() || cliente.is_empty() {
                continue;
            }
            nuovo_contenuto.push_str(&format!("{indirizzo} = {cliente}\n"));
        }
    }

    let parent: &Path = percorso.parent().ok_or_else(|| "percorso non valido".to_string())?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    std::fs::write(&percorso, nuovo_contenuto).map_err(|e| e.to_string())?;

    correggi_eventi_vpn_esistenti(&app_handle, &voci);
    Ok(())
}

/// Corregge nel database SOLO gli eventi già registrati il cui campo
/// "cliente" corrisponde ESATTAMENTE a uno degli indirizzi appena
/// (ri)mappati — cioè solo le sessioni che il watcher aveva dovuto
/// etichettare con l'indirizzo IP grezzo perché non c'era ancora una
/// mappatura, non un rename generico di sessioni già nominate
/// correttamente. Richiesta esplicita dell'utente: "solo ed
/// unicamente per il dato salvato" — il resto della cronologia tracciata
/// resta intoccato (stessa filosofia già seguita altrove nel progetto:
/// mai riscrivere dati già registrati "a caso"), qui si corregge solo un
/// valore che sappiamo per certo essere sbagliato (un IP al posto di un
/// nome) ora che la mappatura giusta è nota. Un errore qui (server non
/// ancora pronto, query fallita) non fa fallire il salvataggio della
/// mappatura stessa — è già scritta su disco, questa è solo una
/// rifinitura sui dati storici.
fn correggi_eventi_vpn_esistenti(app_handle: &AppHandle, voci: &[VoceMappingVpnInput]) {
    let Some(server) = app_handle.try_state::<Arc<AppServer>>() else {
        return;
    };

    let da_correggere: std::collections::HashMap<&str, &str> = voci
        .iter()
        .map(|v| (v.indirizzo.trim(), v.cliente.trim()))
        .filter(|(indirizzo, cliente)| !indirizzo.is_empty() && !cliente.is_empty())
        .collect();
    if da_correggere.is_empty() {
        return;
    }

    let eventi = match server.datastore.get_events(BUCKET_ID, None, None, None) {
        Ok(eventi) => eventi,
        Err(_) => return,
    };

    let mut da_aggiornare = Vec::new();
    for mut evento in eventi {
        let cliente_attuale = evento
            .data
            .get("cliente")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let Some(cliente_attuale) = cliente_attuale else {
            continue;
        };
        if let Some(nuovo_nome) = da_correggere.get(cliente_attuale.as_str()) {
            evento
                .data
                .insert("cliente".to_string(), serde_json::Value::String(nuovo_nome.to_string()));
            da_aggiornare.push(evento);
        }
    }

    if !da_aggiornare.is_empty() {
        log::info!(
            "Corretti {} eventi VPN storici dopo una nuova mappatura indirizzo→cliente",
            da_aggiornare.len()
        );
        let _ = server.datastore.insert_events(BUCKET_ID, &da_aggiornare);
    }
}

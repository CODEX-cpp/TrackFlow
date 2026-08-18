//! Avvio automatico con Windows (Impostazioni → Generale) — scrive/
//! rimuove una voce nella chiave di registro Run dell'utente corrente
//! (`HKCU`, nessun privilegio amministratore richiesto, coerente col
//! resto dell'installazione — vedi `RequestExecutionLevel user` nel
//! .nsi), puntata a `launcher.exe` e mai direttamente ad `app.exe`,
//! stesso motivo di ogni altro collegamento (vedi `launcher/src/
//! main.rs`): è l'unico punto d'ingresso stabile attraverso gli
//! aggiornamenti. Non impostato dall'installer di sua iniziativa — un
//! toggle esplicito nelle Impostazioni, non un comportamento imposto.

use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};
use winreg::RegKey;

const CHIAVE_RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const NOME_VOCE: &str = "TrackFlow";

fn percorso_launcher_tra_virgolette() -> Result<String, String> {
    let installazione = crate::updater::cartella_installazione()?;
    // --minimized: SOLO qui, mai su un collegamento manuale (Menu Start,
    // Desktop) — launcher.exe inoltra ogni argomento così com'è ad
    // app.exe (vedi launcher/src/main.rs), che lo controlla per decidere
    // se mostrare subito la finestra principale o restare in background
    // (richiesta esplicita: all'accensione del PC l'app non deve comparire
    // in primo piano, resta comunque pienamente attiva nella tray).
    Ok(format!(
        "\"{}\" --minimized",
        installazione.join("launcher.exe").display()
    ))
}

#[tauri::command]
pub fn imposta_avvio_automatico(abilita: bool) -> Result<(), String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (chiave, _) = hkcu
        .create_subkey_with_flags(CHIAVE_RUN, KEY_WRITE)
        .map_err(|e| format!("impossibile aprire la chiave di avvio automatico: {e}"))?;
    if abilita {
        let percorso = percorso_launcher_tra_virgolette()?;
        chiave
            .set_value(NOME_VOCE, &percorso)
            .map_err(|e| format!("impossibile scrivere la voce di avvio automatico: {e}"))?;
    } else {
        // Nessun errore se la voce non esiste già — spegnere un
        // avvio automatico mai stato acceso non è un fallimento.
        let _ = chiave.delete_value(NOME_VOCE);
    }
    Ok(())
}

#[tauri::command]
pub fn avvio_automatico_abilitato() -> bool {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    hkcu.open_subkey_with_flags(CHIAVE_RUN, KEY_READ)
        .and_then(|chiave| chiave.get_value::<String, _>(NOME_VOCE))
        .is_ok()
}

// Punto d'ingresso stabile di TrackFlow. I collegamenti (Menu Start,
// Desktop, avvio automatico) puntano SEMPRE a questo eseguibile, mai
// direttamente a una versione specifica — così un aggiornamento (che
// scarica ed estrae la versione nuova in una cartella "versions/x.y.z"
// a fianco di quella in uso, poi scrive current.txt) non deve mai
// toccare un file che l'utente ha già in esecuzione: non c'è nulla da
// sovrascrivere, solo una nuova cartella e un puntatore da aggiornare.
//
// Deliberatamente senza dipendenze oltre alla libreria standard: deve
// avviarsi all'istante e avere pochissimo che possa rompersi, essendo
// l'UNICA cosa a cui è sempre puntato dall'esterno.
#![windows_subsystem = "windows"]

use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

const CURRENT_FILE: &str = "current.txt";
const VERSIONS_DIR: &str = "versions";
const APP_EXE: &str = "app.exe";

fn cartella_installazione() -> Option<PathBuf> {
    env::current_exe().ok()?.parent().map(Path::to_path_buf)
}

// Nessuna console disponibile (windows_subsystem = "windows") — un
// eventuale errore va scritto da qualche parte di leggibile invece che
// perso nel nulla, così un problema di avvio è comunque diagnosticabile
// senza dover ricompilare in modalità console per vederlo.
fn segnala_errore(cartella: &Path, messaggio: &str) {
    let percorso_log = cartella.join("launcher-error.log");
    if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(percorso_log) {
        let _ = writeln!(file, "{messaggio}");
    }
}

fn main() {
    let Some(cartella) = cartella_installazione() else {
        return;
    };

    let percorso_current = cartella.join(CURRENT_FILE);
    let versione = match fs::read_to_string(&percorso_current) {
        Ok(contenuto) => contenuto.trim().to_string(),
        Err(e) => {
            segnala_errore(&cartella, &format!("Impossibile leggere {CURRENT_FILE}: {e}"));
            return;
        }
    };

    if versione.is_empty() {
        segnala_errore(&cartella, &format!("{CURRENT_FILE} è vuoto — installazione incompleta?"));
        return;
    }

    let percorso_app = cartella.join(VERSIONS_DIR).join(&versione).join(APP_EXE);
    if !percorso_app.exists() {
        segnala_errore(
            &cartella,
            &format!("Versione '{versione}' indicata in {CURRENT_FILE} ma {} non esiste", percorso_app.display()),
        );
        return;
    }

    // Passa avanti eventuali argomenti (es. protocolli/deep link) senza
    // doverli capire — non è compito del launcher interpretarli.
    let argomenti: Vec<String> = env::args().skip(1).collect();

    match Command::new(&percorso_app).args(&argomenti).spawn() {
        Ok(_) => {}
        Err(e) => segnala_errore(&cartella, &format!("Avvio di {} fallito: {e}", percorso_app.display())),
    }
}

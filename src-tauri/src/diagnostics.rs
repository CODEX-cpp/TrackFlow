//! Modulo diagnostico avanzato (perf: uso CPU/RAM del processo, tempi di
//! query/rendering lato frontend) nato durante l'indagine su un
//! rallentamento segnalato dall'utente su PC senza scheda grafica
//! dedicata — vedi BLUEPRINT.md sezione 45. La causa reale (risparmio
//! energetico di Windows che limita la frequenza della CPU) è stata
//! trovata e non è risolvibile da qui, ma il modulo resta a disposizione
//! per indagini future: **disattivato di default** (richiesta esplicita
//! dell'utente, per non scrivere continuamente su disco senza motivo),
//! attivabile da Impostazioni → Sviluppatore insieme alla scelta della
//! cartella di destinazione del file.
//!
//! Scrive un file di testo (`trackflow-diagnostica.log`, una riga per
//! evento) SOLO mentre la finestra di TrackFlow è in primo piano (vedi
//! `IN_FOREGROUND`, aggiornato da `lib.rs` sull'evento
//! `WindowEvent::Focused`) E solo mentre l'utente ha attivato il log
//! (vedi `ATTIVA`, impostato da `imposta_diagnostica`). Riceve eventi
//! sia dal lato Rust (uso CPU/RAM del processo, campionato
//! periodicamente) sia dal lato frontend (tempi di query, di
//! costruzione blocchi Timeline, di rendering — vedi
//! `src/util/diagnostics.ts`), tramite il comando
//! `log_frontend_diagnostica` cablato qui sotto: un unico file con
//! tutto, non due log separati da incrociare a mano.
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use serde_json::Value;
use sysinfo::{Pid, ProcessesToUpdate, System};

/// true di default: finché la finestra non manda il suo primo evento
/// Focused/Blur (appena dopo la creazione) è meglio registrare tutto
/// piuttosto che perdere silenziosamente l'avvio dell'app per un falso
/// "non in primo piano".
pub static IN_FOREGROUND: AtomicBool = AtomicBool::new(true);

/// Disattivato di default — richiesta esplicita dell'utente. Si attiva
/// solo dalle Impostazioni → Sviluppatore, tramite `imposta_diagnostica`.
static ATTIVA: AtomicBool = AtomicBool::new(false);

/// Il thread di campionamento CPU/RAM va avviato una sola volta per
/// tutta la vita del processo — se l'utente disattiva e riattiva il log
/// più volte dalla stessa sessione, non deve accumularsi un thread per
/// ogni riattivazione. Il thread stesso controlla `ATTIVA`/`IN_FOREGROUND`
/// ad ogni giro e non fa nulla se non è il momento di scrivere.
static THREAD_CAMPIONAMENTO_AVVIATO: AtomicBool = AtomicBool::new(false);

static LOG_FILE: Mutex<Option<File>> = Mutex::new(None);

/// Cartella di default se l'utente non ne ha scelta una: il Desktop
/// (comportamento storico di questo modulo). Niente crate "dirs" tra le
/// dipendenze solo per questo — su Windows il Desktop è sempre
/// `%USERPROFILE%\Desktop`, e quella variabile d'ambiente è sempre
/// presente per un processo utente normale.
fn cartella_default() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .map(|home| PathBuf::from(home).join("Desktop"))
        .unwrap_or_else(std::env::temp_dir)
}

const NOME_FILE: &str = "trackflow-diagnostica.log";

/// Attiva (o riattiva con una cartella diversa) il log diagnostico.
/// Chiamata sia all'avvio dell'app (se l'impostazione era già attiva
/// l'ultima volta, vedi `lib.rs`) sia dal vivo quando l'utente
/// interviene dalle Impostazioni — in entrambi i casi (ri)apre il file
/// nella cartella indicata, così cambiare cartella con il log già
/// attivo funziona senza dover riavviare l'app.
pub fn avvia(cartella: &Path) -> Result<(), String> {
    std::fs::create_dir_all(cartella).map_err(|e| e.to_string())?;
    let percorso = cartella.join(NOME_FILE);
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&percorso)
        .map_err(|e| format!("Impossibile aprire il file diagnostico {percorso:?}: {e}"))?;
    *LOG_FILE.lock().unwrap() = Some(file);
    ATTIVA.store(true, Ordering::Relaxed);

    scrivi(
        "diagnostica_avviata",
        serde_json::json!({ "percorso": percorso.display().to_string() }),
    );

    if !THREAD_CAMPIONAMENTO_AVVIATO.swap(true, Ordering::Relaxed) {
        let pid = Pid::from_u32(std::process::id());
        std::thread::spawn(move || {
            let mut sys = System::new();
            loop {
                std::thread::sleep(Duration::from_secs(2));
                if !ATTIVA.load(Ordering::Relaxed) || !IN_FOREGROUND.load(Ordering::Relaxed) {
                    continue;
                }
                sys.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);
                if let Some(processo) = sys.process(pid) {
                    scrivi(
                        "risorse_processo",
                        serde_json::json!({
                            "cpu_percento": processo.cpu_usage(),
                            "memoria_mb": processo.memory() / 1024 / 1024,
                        }),
                    );
                }
            }
        });
    }

    Ok(())
}

/// Disattiva il log e chiude il file — così se l'utente cambia
/// cartella in seguito non resta aperto un handle su quella vecchia.
pub fn ferma() {
    ATTIVA.store(false, Ordering::Relaxed);
    *LOG_FILE.lock().unwrap() = None;
}

/// Scrive una riga SOLO se il log è attivo E la finestra è in primo
/// piano — il filtro sta qui, in un unico punto, così sia le chiamate
/// Rust dirette sotto sia quelle inoltrate dal frontend
/// (`log_frontend_diagnostica`) lo rispettano senza doverlo ripetere ad
/// ogni chiamante.
pub fn scrivi(evento: &str, dettagli: Value) {
    if !ATTIVA.load(Ordering::Relaxed) || !IN_FOREGROUND.load(Ordering::Relaxed) {
        return;
    }
    let riga = format!(
        "{} {} {}\n",
        chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        evento,
        dettagli
    );
    if let Ok(mut guard) = LOG_FILE.lock() {
        if let Some(file) = guard.as_mut() {
            // flush ad ogni riga: un file diagnostico che si potrebbe
            // perdere a metà se l'app si blocca (proprio il genere di
            // sintomo per cui questo modulo è nato) sarebbe inutile.
            let _ = file.write_all(riga.as_bytes());
            let _ = file.flush();
        }
    }
}

/// Da chiamare una sola volta all'avvio dell'app (vedi `lib.rs`),
/// leggendo lo stato persistito dalle Impostazioni. `cartella` vuota o
/// assente equivale a "usa la cartella di default" (Desktop).
pub fn avvia_da_impostazioni(abilitata: bool, cartella: Option<&str>) {
    if !abilitata {
        return;
    }
    let cartella = match cartella.filter(|c| !c.trim().is_empty()) {
        Some(c) => PathBuf::from(c),
        None => cartella_default(),
    };
    if let Err(e) = avvia(&cartella) {
        log::error!("Impossibile avviare la diagnostica: {e}");
    }
}

/// Punto di ingresso dal frontend — vedi `src/util/diagnostics.ts`.
/// `dettagli` è testo libero (qualunque JSON), non uno schema fisso:
/// pensato per raccogliere il più possibile durante un'indagine, non
/// per validare una struttura decisa in anticipo.
#[tauri::command]
pub fn log_frontend_diagnostica(evento: String, dettagli: Value) {
    scrivi(&evento, dettagli);
}

/// Comando invocato dal toggle/selettore cartella in Impostazioni →
/// Sviluppatore — applica il cambiamento subito, senza richiedere un
/// riavvio dell'app. La persistenza (così il riavvio successivo
/// ricorda lo stato) passa dal solito meccanismo di override file in
/// `lib.rs`'s `dispatch()`, innescato dal normale salvataggio delle
/// impostazioni lato frontend.
#[tauri::command]
pub fn imposta_diagnostica(abilitata: bool, cartella: Option<String>) -> Result<(), String> {
    if !abilitata {
        ferma();
        return Ok(());
    }
    let cartella = match cartella.filter(|c| !c.trim().is_empty()) {
        Some(c) => PathBuf::from(c),
        None => cartella_default(),
    };
    avvia(&cartella)
}

// Controllo di integrità + auto-ripristino da backup, eseguito una sola
// volta all'avvio PRIMA di aprire il database per davvero (vedi
// `verifica_e_ripristina` sotto, chiamata da src-tauri/src/lib.rs prima
// di `Datastore::new`).
//
// Perché questo file esiste: il database si è corrotto per davvero più
// volte durante lo sviluppo (`database disk image is malformed`), quasi
// certamente per un `TerminateProcess` (Task Manager "Termina attività",
// o uno script che fa `Stop-Process -Force`) arrivato nel momento
// sbagliato di un checkpoint del WAL. Un fix precedente
// (`AppServer::close()`, vedi worker.rs) aspetta davvero il worker prima
// di uscire — ma protegge SOLO l'uscita "pulita" (pulsante Esci/chiusura
// finestra): un `TerminateProcess` esterno non dà all'app NESSUNA
// possibilità di eseguire codice, quindi nessun fix lato "chiusura" può
// mai prevenire la corruzione in quel caso. L'unica difesa possibile è
// qui: assumere che possa corrompersi di nuovo, e fare in modo che
// l'app **si accorga da sola all'avvio successivo e si ripari da sola**
// invece di restare bloccata con la schermata bianca — esattamente il
// sintomo con cui l'utente ha segnalato l'incidente che ha portato a
// scrivere questo modulo.
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

// Un backup più vecchio di questo non "copre" più l'avvio corrente —
// se ne fa uno nuovo. Compromesso tra spazio su disco (ogni backup è
// l'intero file, tipicamente qualche MB) e quanta attività reale si
// rischia di perdere in caso di corruzione proprio a ridosso della
// prossima finestra di backup.
const INTERVALLO_BACKUP: Duration = Duration::from_secs(3 * 60 * 60);
// Quanti backup automatici tenere in rotazione — il più vecchio viene
// cancellato ad ogni nuovo backup oltre questa soglia.
const BACKUP_DA_TENERE: usize = 8;

fn cartella_backup(db_path: &Path) -> PathBuf {
    db_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("backups")
}

// Vero se il file a questo percorso è un database SQLite integro
// (`PRAGMA quick_check` — più veloce di `integrity_check` completo, ma
// comunque rileva la corruzione a livello di pagina che ci interessa
// qui; questa funzione gira ad ogni avvio dell'app, non deve rallentarlo
// percettibilmente). Un file mancante o illeggibile non è "integro" ma
// nemmeno un errore da segnalare qui — il chiamante distingue i due casi.
fn integro(path: &Path) -> bool {
    let conn = match Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
    {
        Ok(c) => c,
        Err(_) => return false,
    };
    match conn.pragma_query_value(None, "quick_check", |row| row.get::<_, String>(0)) {
        Ok(risultato) => risultato == "ok",
        Err(_) => false,
    }
}

// Il backup valido più recente nella cartella backups, o None se non ce
// n'è nessuno (prima installazione, o cartella backup persa insieme al
// resto). Ne verifica l'integrità prima di fidarsene — un backup preso
// mentre il database era già silenziosamente corrotto sarebbe inutile,
// meglio scartarlo e passare al successivo più vecchio piuttosto che
// ripristinare un'altra copia rotta.
fn backup_valido_piu_recente(cartella: &Path) -> Option<PathBuf> {
    let mut candidati: Vec<(std::time::SystemTime, PathBuf)> = fs::read_dir(cartella)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|e| e == "db").unwrap_or(false))
        .filter_map(|p| fs::metadata(&p).ok().and_then(|m| m.modified().ok()).map(|t| (t, p)))
        .collect();
    candidati.sort_by(|a, b| b.0.cmp(&a.0));
    candidati.into_iter().map(|(_, p)| p).find(|p| integro(p))
}

// Chiamata ad ogni avvio, PRIMA che `Datastore::new` apra il database
// per davvero. Ritorna `true` se ha dovuto ripristinare un backup
// (informazione solo per il log del chiamante, vedi lib.rs) — il
// database al percorso `db_path` è comunque garantito pronto all'uso
// quando questa funzione ritorna, tranne nel caso limite in cui non
// esiste NESSUN backup valido: in quel caso il file corrotto viene
// comunque spostato da parte e l'app riparte con un database vuoto
// (perdita totale della cronologia, ma l'app SI APRE — meglio di una
// schermata bianca permanente).
pub fn verifica_e_ripristina(db_path: &str) -> bool {
    let path = Path::new(db_path);
    if !path.exists() {
        // Prima installazione — niente da controllare o riparare.
        return false;
    }
    if integro(path) {
        backup_se_dovuto(path);
        return false;
    }

    error!(
        "Database corrotto rilevato all'avvio ({db_path}) — quasi certamente una chiusura non pulita \
         (Task Manager, kill esterno) durante un checkpoint del WAL. Provo a ripristinare da un backup automatico."
    );

    let cartella = cartella_backup(path);
    let backup = backup_valido_piu_recente(&cartella);

    // Il file corrotto NON viene mai cancellato, solo spostato da parte
    // — può sempre servire per un tentativo di recupero manuale più
    // avanti (stessa prassi già seguita a mano più volte durante lo
    // sviluppo, vedi BLUEPRINT.md).
    let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let corrotto_spostato = path.with_file_name(format!(
        "{}.corrupt-{timestamp}",
        path.file_name().and_then(|n| n.to_str()).unwrap_or("sqlite.db")
    ));
    if let Err(err) = fs::rename(path, &corrotto_spostato) {
        error!("Impossibile spostare il database corrotto da parte: {err} — riparto comunque su un file vuoto.");
        let _ = fs::remove_file(path);
    }
    // WAL/SHM del file corrotto appartengono a quel file, non hanno
    // senso per il database che sta per prenderne il posto (backup o
    // vuoto) — lasciarli lì confonderebbe la prossima apertura.
    let _ = fs::remove_file(path.with_extension("db-wal"));
    let _ = fs::remove_file(path.with_extension("db-shm"));

    match backup {
        Some(sorgente) => {
            match fs::copy(&sorgente, path) {
                Ok(_) => {
                    warn!(
                        "Database ripristinato dal backup automatico {sorgente:?} — l'attività \
                         tracciata dopo quel backup (fino a {INTERVALLO_BACKUP:?} prima della \
                         corruzione) è andata persa, il resto della cronologia è intatto."
                    );
                }
                Err(err) => {
                    error!("Impossibile copiare il backup {sorgente:?}: {err} — riparto su un database vuoto.");
                }
            }
        }
        None => {
            warn!(
                "Nessun backup automatico valido trovato in {cartella:?} — riparto su un database \
                 vuoto (l'intera cronologia precedente è andata persa, il file corrotto resta comunque \
                 salvato come {corrotto_spostato:?} per un eventuale recupero manuale)."
            );
        }
    }
    true
}

// Copia il database (già confermato integro dal chiamante) dentro
// backups/ come istantanea con timestamp, ma solo se l'ultimo backup è
// più vecchio di INTERVALLO_BACKUP (o non esiste ancora) — non ad ogni
// singolo avvio, per non riempire il disco di copie quasi identiche
// durante una sessione con molti riavvii ravvicinati. Consolida prima
// il WAL nel file principale (`wal_checkpoint(TRUNCATE)`) così la copia
// è autosufficiente: un file .db da solo, senza bisogno dei suoi -wal/
// -shm per essere valido.
fn backup_se_dovuto(path: &Path) {
    let cartella = cartella_backup(path);
    if fs::create_dir_all(&cartella).is_err() {
        return;
    }

    let ultimo = fs::read_dir(&cartella)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.metadata().ok().and_then(|m| m.modified().ok()))
        .max();
    if let Some(t) = ultimo {
        if SystemTime::now().duration_since(t).unwrap_or(Duration::ZERO) < INTERVALLO_BACKUP {
            return;
        }
    }

    // Connessione dedicata e temporanea solo per il checkpoint — chiusa
    // subito dopo (fine di questa funzione), non tenuta aperta insieme
    // a quella vera del Datastore che segue.
    if let Ok(conn) = Connection::open(path) {
        let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
    }

    let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let destinazione = cartella.join(format!("sqlite-{timestamp}.db"));
    if fs::copy(path, &destinazione).is_err() {
        return;
    }
    info!("Backup automatico del database salvato: {destinazione:?}");

    // Rotazione: tiene solo i BACKUP_DA_TENERE più recenti.
    let mut esistenti: Vec<(SystemTime, PathBuf)> = fs::read_dir(&cartella)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter_map(|p| fs::metadata(&p).ok().and_then(|m| m.modified().ok()).map(|t| (t, p)))
        .collect();
    esistenti.sort_by(|a, b| b.0.cmp(&a.0));
    for (_, vecchio) in esistenti.into_iter().skip(BACKUP_DA_TENERE) {
        let _ = fs::remove_file(vecchio);
    }
}

// Legge la lingua salvata (`settings.locale`, la stessa scritta dal
// frontend — vedi src/i18n/index.ts) con una connessione a parte, sola
// lettura — serve al menu della tray (src-tauri/src/lib.rs), costruito
// in modo sincrono durante `.setup()` ben prima che il vero AppServer/
// Datastore (costruito in un task async separato, vedi build_app_server)
// sia pronto: senza questa scorciatoia il menu della tray non avrebbe
// nessun modo di sapere la lingua scelta a quel punto. Bug reale
// segnalato da un utente esterno (issue GitHub #2, "Tray is wrong
// language"): il menu era hardcoded in italiano, ignorando del tutto
// l'impostazione lingua.
// Tollera qualunque errore restituendo `None` (installazione nuova,
// tabella non ancora creata, database corrotto non ancora riparato) —
// il chiamante ricade su un default sensato.
pub fn leggi_locale_per_tray(db_path: &str) -> Option<String> {
    let conn = Connection::open_with_flags(db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).ok()?;
    let raw: String = conn
        .query_row("SELECT value FROM key_value WHERE key = 'settings.locale'", [], |row| row.get(0))
        .ok()?;
    // Il valore è salvato come stringa JSON (es. `"it"` con le
    // virgolette incluse) — stesso formato scritto dall'endpoint POST
    // /api/0/settings/<key> (vedi aw-server/src/endpoints/settings.rs).
    serde_json::from_str::<String>(&raw).ok()
}

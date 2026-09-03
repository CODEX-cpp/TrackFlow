//! Watcher per l'attività in Excel (file aperto) — stessa tecnica di
//! aw-watcher-vscode-rust: nessuna API COM/Office, solo l'elenco delle
//! finestre aperte (EnumWindows) e il titolo, perché gira come processo
//! esterno indipendente (coerente con l'architettura "zero Python/zero
//! rete" del progetto), non come plugin/add-in Excel.
//!
//! Richiesta esplicita dell'utente: non interessa QUALE file abbia il
//! fuoco in un dato momento, né una precisione al secondo — interessa
//! solo sapere quando un file è stato aperto, quando è stato chiuso, e
//! per quanto tempo è rimasto aperto, anche con più file Excel aperti
//! insieme in parallelo. Per questo il watcher non manda heartbeat
//! continui (che nel backend si fondono in un unico evento per bucket,
//! bene per UN file alla volta ma frammenterebbero la durata di ognuno
//! se più file fossero tracciati in parallelo) — invece tiene lui stesso
//! il conto di apertura/chiusura per ogni file e, alla chiusura, manda
//! UN SOLO evento completo (timestamp di apertura + durata reale). Il
//! prezzo di questo approccio: se l'intera app viene chiusa mentre un
//! file è ancora aperto, quella sessione non viene registrata affatto
//! (non c'è mai una chiusura da rilevare) — accettabile per un dato che
//! non deve essere preciso al secondo, per data una sessione persa è
//! comunque meno grave di dati frammentati o sbagliati.
//!
//! IMPORTANTE (2026-08-12): costruito e compilato su una macchina senza
//! Excel installato — il parsing del titolo cerca la sotto-stringa
//! fissa " - Excel" e taglia lì, così qualunque testo aggiuntivo dopo
//! (" (Modalità compatibilità)", " (Sola lettura)", ...) viene scartato
//! senza bisogno di elencare ogni variante.
//!
//! Bug reale segnalato da un utente (issue GitHub #4, con log e
//! screenshot reali da Excel 2021 MSO/Microsoft 365): un titolo che NON
//! contiene " - Excel" non è affatto un file mai visto prima — è quasi
//! sempre una finestra di UTILITÀ di Excel (es. "Find and Replace"/
//! "Trova e sostituisci", ma lo stesso vale per "Formato celle",
//! "Imposta pagina", ecc.), un'altra finestra top-level separata dello
//! stesso processo excel.exe, non un documento. La versione precedente
//! di questa funzione trattava comunque quel titolo intero come nome
//! file (v. commento storico rimosso da qui) "per non perdere dati" —
//! risultato osservato dal vivo: "Find and Replace" compariva come una
//! RIGA A SÉ nella corsia Excel della Timeline, con una sessione fasulla
//! di pochi secondi, mentre il file vero continuava ad essere tracciato
//! correttamente in parallelo sulla sua riga. Con dati reali da 6 file
//! Excel diversi che rispettano tutti il pattern " - Excel" e UN solo
//! caso di fallback che si è rivelato essere proprio una finestra di
//! dialogo, il fallback permissivo fa più danni (falsi file) che
//! benefici (dati salvati) — ora un titolo che non rispetta il pattern
//! viene semplicemente ignorato, non trasformato in un file fantasma.

use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::thread;
use std::time::Duration as StdDuration;

use chrono::{DateTime, SecondsFormat, Utc};
use clap::Parser;
use serde_json::{json, Map};

const CLIENT_NAME: &str = "aw-watcher-excel";
// Tipo dedicato, DIVERSO da "app.editor.activity" (VS Code) apposta:
// la webui seleziona i bucket "Top Editor Files/Projects" per tipo, non
// per ID (vedi stores/buckets.ts's bucketsEditor()) — riusare lo stesso
// tipo avrebbe mescolato i file Excel dentro quei moduli invece di
// restare una corsia/dato separato, come richiesto esplicitamente.
const BUCKET_TYPE: &str = "app.excel.activity";

fn is_excel_exe(app: &str) -> bool {
    app.eq_ignore_ascii_case("excel.exe")
}

/// "<file> - Excel", "<file> - Excel (Modalità compatibilità)", "<file>
/// - Excel (Sola lettura)", ecc. — taglia alla sotto-stringa fissa
/// " - Excel" invece di elencare ogni variante di suffisso (vedi
/// commento in cima al file sul perché). Nessun file aperto (titolo
/// bare "Excel"), titolo vuoto, o titolo che non rispetta il pattern
/// (quasi sempre una finestra di utilità/dialogo, non un documento —
/// vedi il commento in cima al file sul bug reale che questo evita)
/// -> None.
fn interpreta_titolo(titolo: &str) -> Option<String> {
    let pulito = titolo.trim();
    if pulito.is_empty() || pulito.eq_ignore_ascii_case("excel") {
        return None;
    }
    let pos = pulito.find(" - Excel")?;
    let file = pulito[..pos].trim();
    if file.is_empty() {
        None
    } else {
        Some(file.to_string())
    }
}

#[cfg(windows)]
fn percorso_processo(pid: u32) -> Option<String> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::ProcessStatus::GetModuleFileNameExW;
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION};

    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_INFORMATION, false, pid).ok()?;
        let mut path_buf = [0u16; 1024];
        let len = GetModuleFileNameExW(Some(handle), None, &mut path_buf);
        let _ = CloseHandle(handle);
        if len == 0 {
            return None;
        }
        Some(String::from_utf16_lossy(&path_buf[..len as usize]))
    }
}

/// Versione del file (es. "16.0.20326.20072") letta dalle risorse
/// dell'eseguibile — usata solo dal log diagnostico dettagliato (vedi
/// `LogDettagliatoState`/`main`), per capire con precisione quale build
/// di Excel un utente sta usando senza doverglielo chiedere a mano ogni
/// volta (come fatto per issue GitHub #4). `None` se le informazioni di
/// versione non sono presenti nel file o la lettura fallisce per
/// qualunque motivo — mai un errore fatale per il watcher.
#[cfg(windows)]
fn versione_file(percorso: &str) -> Option<String> {
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{
        GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW, VS_FIXEDFILEINFO,
    };

    let percorso_wide: Vec<u16> = percorso.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let dimensione = GetFileVersionInfoSizeW(PCWSTR(percorso_wide.as_ptr()), None);
        if dimensione == 0 {
            return None;
        }
        let mut buffer = vec![0u8; dimensione as usize];
        GetFileVersionInfoW(
            PCWSTR(percorso_wide.as_ptr()),
            None,
            dimensione,
            buffer.as_mut_ptr() as *mut core::ffi::c_void,
        )
        .ok()?;

        let sotto_blocco: Vec<u16> = "\\".encode_utf16().chain(std::iter::once(0)).collect();
        let mut ptr_info: *mut core::ffi::c_void = std::ptr::null_mut();
        let mut lunghezza: u32 = 0;
        let ok = VerQueryValueW(
            buffer.as_ptr() as *const core::ffi::c_void,
            PCWSTR(sotto_blocco.as_ptr()),
            &mut ptr_info,
            &mut lunghezza,
        );
        if !ok.as_bool() || ptr_info.is_null() {
            return None;
        }
        let info = &*(ptr_info as *const VS_FIXEDFILEINFO);
        let major = (info.dwFileVersionMS >> 16) & 0xffff;
        let minor = info.dwFileVersionMS & 0xffff;
        let build = (info.dwFileVersionLS >> 16) & 0xffff;
        let revisione = info.dwFileVersionLS & 0xffff;
        Some(format!("{major}.{minor}.{build}.{revisione}"))
    }
}

/// Tutto quello che si può leggere di una finestra top-level di
/// excel.exe via Win32 puro (nessuna API COM/Object Model — vedi il
/// commento in cima al file sul perché) — sia che il suo titolo
/// rappresenti un documento vero sia una finestra di utilità/dialogo.
/// Usata per il tracciamento normale (via `file_interpretato`) E, per
/// intero, dal log diagnostico dettagliato quando attivo (vedi `main`) —
/// richiesta esplicita dell'utente dopo il bug "Trova e sostituisci"
/// dell'issue #4: non fidarsi più di un singolo screenshot isolato, poter
/// vedere riga per riga, durante un uso reale prolungato, esattamente
/// cosa il watcher vede ad ogni controllo.
#[derive(Debug, Clone)]
struct InfoFinestraExcel {
    hwnd: isize,
    pid: u32,
    titolo: String,
    classe: String,
    percorso_processo: Option<String>,
    versione_file: Option<String>,
    rect: Option<(i32, i32, i32, i32)>,
    focalizzata: bool,
    minimizzata: bool,
    abilitata: bool,
    owner_hwnd: Option<isize>,
    /// Risultato di `interpreta_titolo(titolo)` — già calcolato qui
    /// così sia il tracciamento normale sia il log diagnostico vedono
    /// esattamente la stessa interpretazione, senza ricalcolarla in due
    /// posti che potrebbero disallinearsi.
    file_interpretato: Option<String>,
}

/// Elenca TUTTE le finestre top-level visibili di processi excel.exe in
/// questo momento — documenti E finestre di utilità/dialogo insieme,
/// indipendentemente da quale abbia il fuoco (un file resta "aperto"
/// agli occhi del watcher finché la sua finestra esiste, non solo mentre
/// lo si guarda). Il tracciamento apertura/chiusura in `main()` filtra
/// da questo elenco solo quelle con `file_interpretato` valorizzato.
#[cfg(windows)]
fn elenca_finestre_excel() -> Vec<InfoFinestraExcel> {
    use windows::Win32::Foundation::{HWND, LPARAM, RECT};
    use windows::core::BOOL;
    use windows::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled;
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetClassNameW, GetForegroundWindow, GetWindow, GetWindowRect,
        GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindowVisible, GW_OWNER,
    };

    unsafe extern "system" fn callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        // Continua l'enumerazione anche negli "early return": la firma di
        // EnumWindows tratta FALSE come "interrompi tutto", non voluto qui.
        if !unsafe { IsWindowVisible(hwnd) }.as_bool() {
            return BOOL(1);
        }

        let mut title_buf = [0u16; 1024];
        let len = unsafe { GetWindowTextW(hwnd, &mut title_buf) };
        if len <= 0 {
            return BOOL(1);
        }
        let titolo = String::from_utf16_lossy(&title_buf[..len as usize]);

        let mut pid: u32 = 0;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        let Some(percorso) = percorso_processo(pid) else {
            return BOOL(1);
        };
        let app = Path::new(&percorso).file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
        if !is_excel_exe(&app) {
            return BOOL(1);
        }

        let mut class_buf = [0u16; 256];
        let class_len = unsafe { GetClassNameW(hwnd, &mut class_buf) };
        let classe = if class_len > 0 {
            String::from_utf16_lossy(&class_buf[..class_len as usize])
        } else {
            String::new()
        };

        let mut rect = RECT::default();
        let rect = unsafe { GetWindowRect(hwnd, &mut rect) }
            .ok()
            .map(|_| (rect.left, rect.top, rect.right, rect.bottom));

        let minimizzata = unsafe { IsIconic(hwnd) }.as_bool();
        let abilitata = unsafe { IsWindowEnabled(hwnd) }.as_bool();
        let focalizzata = unsafe { GetForegroundWindow() } == hwnd;
        let owner_hwnd = unsafe { GetWindow(hwnd, GW_OWNER) }.ok().map(|h| h.0 as isize);
        let versione_file = versione_file(&percorso);
        let file_interpretato = interpreta_titolo(&titolo);

        let risultati = unsafe { &mut *(lparam.0 as *mut Vec<InfoFinestraExcel>) };
        risultati.push(InfoFinestraExcel {
            hwnd: hwnd.0 as isize,
            pid,
            titolo,
            classe,
            percorso_processo: Some(percorso),
            versione_file,
            rect,
            focalizzata,
            minimizzata,
            abilitata,
            owner_hwnd,
            file_interpretato,
        });
        BOOL(1)
    }

    let mut risultati: Vec<InfoFinestraExcel> = Vec::new();
    let lparam = LPARAM(std::ptr::addr_of_mut!(risultati) as isize);
    let _ = unsafe { EnumWindows(Some(callback), lparam) };
    risultati
}

#[cfg(not(windows))]
fn elenca_finestre_excel() -> Vec<InfoFinestraExcel> {
    compile_error!("aw-watcher-excel supporta solo Windows");
}

/// Elenco dei nomi file "aperti", derivato da `elenca_finestre_excel()`
/// filtrando solo le finestre con `file_interpretato` valorizzato —
/// stesso identico elenco che produceva la vecchia `file_excel_aperti()`,
/// ora fattorizzato per essere calcolato una sola volta per giro e
/// riusato anche dal log diagnostico (vedi `main`).
fn file_aperti_da_finestre(finestre: &[InfoFinestraExcel]) -> Vec<String> {
    let mut risultati: Vec<String> = Vec::new();
    for f in finestre {
        if let Some(file) = &f.file_interpretato {
            if !risultati.contains(file) {
                risultati.push(file.clone());
            }
        }
    }
    risultati
}

/// Separa un descrittore file come lo produce `interpreta_titolo` (es.
/// "RipeilogoDati_Renergy.xlsx [Sola lettura]") nel nome file pulito e,
/// se presente, il testo tra le parentesi quadre finali. Bug reale
/// segnalato dall'utente: il titolo REALE di Excel mette l'indicatore
/// di modalità (Sola lettura, Modalità protetta, ...) PRIMA di
/// " - Excel" e tra QUADRE, non dopo e tra tonde come ipotizzato in
/// `interpreta_titolo` (mai verificato empiricamente finché l'utente
/// non ha testato su un'installazione Excel vera, vedi commento in cima
/// al file) — quel testo restava quindi dentro il nome file stesso,
/// facendo trattare lo stesso identico file aperto in sola lettura e in
/// modifica come due file COMPLETAMENTE DIVERSI nella Timeline (due
/// blocchi separati invece di uno solo con la modalità come dettaglio).
///
/// Usato solo al momento di mandare l'evento — il tracciamento
/// apertura/chiusura in `main()` continua a usare il descrittore intero
/// invariato come chiave, così un cambio di modalità a runtime (es.
/// sblocco di un file in sola lettura) resta un confine di sessione
/// come già osservato nei dati reali, invece di dover gestire un
/// cambio di modalità a metà sessione.
fn dividi_file_e_modalita(descrittore: &str) -> (String, Option<String>) {
    if descrittore.ends_with(']') {
        if let Some(apertura_parentesi) = descrittore.rfind('[') {
            if apertura_parentesi > 0 {
                let file = descrittore[..apertura_parentesi].trim();
                let modalita = descrittore[apertura_parentesi + 1..descrittore.len() - 1].trim();
                if !file.is_empty() && !modalita.is_empty() {
                    return (file.to_string(), Some(modalita.to_string()));
                }
            }
        }
    }
    (descrittore.to_string(), None)
}

/// Manda UN evento completo (non un heartbeat) per una sessione appena
/// chiusa: `apertura` è quando il file è comparso per la prima volta tra
/// le finestre aperte, `chiusura` è adesso (non più trovato). Op
/// "event", non "heartbeat" — inserisce la riga così com'è, senza
/// passare dal meccanismo di fusione heartbeat del backend (che tiene un
/// solo "ultimo evento" per bucket: andrebbe benissimo per un file alla
/// volta ma frammenterebbe la durata se più file venissero tracciati in
/// parallelo, vedi commento in cima al file).
fn emit_sessione_chiusa(bucket_id: &str, descrittore: &str, apertura: DateTime<Utc>, chiusura: DateTime<Utc>) {
    let (file, modalita) = dividi_file_e_modalita(descrittore);
    let mut data = Map::new();
    data.insert("file".to_string(), file.into());
    if let Some(modalita) = modalita {
        data.insert("modalita".to_string(), modalita.into());
    }
    let durata_secondi = (chiusura - apertura).num_milliseconds() as f64 / 1000.0;
    let envelope = json!({
        "bucket_id": bucket_id,
        "bucket_type": BUCKET_TYPE,
        "client": CLIENT_NAME,
        "op": "event",
        "event": {
            "timestamp": apertura.to_rfc3339_opts(SecondsFormat::Millis, true),
            "duration": durata_secondi,
            "data": data,
        },
    });
    let mut stdout = std::io::stdout();
    let _ = writeln!(stdout, "{envelope}");
    let _ = stdout.flush();
}

#[derive(Parser)]
#[command(about = "Watcher per l'attività in Excel (file aperto), letta dal titolo della finestra")]
struct Args {
    /// Gira in modalità test: dati separati da quelli reali (accettato
    /// per uniformità con gli altri watcher, non cambia comportamento:
    /// questo watcher non parla mai in rete).
    #[arg(long)]
    testing: bool,

    /// Ogni quanti secondi controllare quali file Excel sono aperti —
    /// richiesta esplicita: non serve precisione al secondo, solo sapere
    /// apertura/chiusura di ogni file, quindi un intervallo più largo
    /// (di sicuro non 2s) va benissimo e pesa meno sul sistema. Usato
    /// solo quando il log dettagliato (vedi sotto) è spento — con quello
    /// attivo l'intervallo scende a 5s, richiesta esplicita dell'utente
    /// per un'indagine più fine durante un uso reale.
    #[arg(long, default_value_t = 20.0)]
    poll_interval: f64,

    /// Cartella dati scrivibile condivisa — usata per rileggere ad ogni
    /// giro il file di override del log dettagliato (vedi
    /// `leggi_log_dettagliato`), scritto dalle Impostazioni → Sviluppatore
    /// quando l'utente accende il toggle "Log dettagliato Excel".
    #[arg(long)]
    app_data_dir: Option<std::path::PathBuf>,
}

/// Nome del file di override scritto da `imposta_log_dettagliato_watcher`
/// (src-tauri/src/watcher_status.rs) quando l'utente accende/spegne il
/// toggle "Log dettagliato" per questo watcher dalle Impostazioni →
/// Sviluppatore → Stato watcher. Stessa convenzione "true"/qualunque
/// altra cosa = false già usata per gli altri override (vedi
/// aw-watcher-screenshot-rust).
const LOG_DETTAGLIATO_OVERRIDE_FILE: &str = "detailed-log-aw-watcher-excel-override.txt";

fn leggi_log_dettagliato(app_data_dir: Option<&Path>) -> bool {
    let Some(dir) = app_data_dir else { return false };
    std::fs::read_to_string(dir.join(LOG_DETTAGLIATO_OVERRIDE_FILE))
        .map(|s| s.trim() == "true")
        .unwrap_or(false)
}

/// Intervallo usato quando il log dettagliato è acceso — più breve del
/// default (20s) per un'indagine più fine, richiesta esplicita
/// dell'utente ("non ogni 20 secondi, ogni 5").
const INTERVALLO_LOG_DETTAGLIATO_SECONDI: f64 = 5.0;

/// Nome del file dedicato al log dettagliato — DELIBERATAMENTE separato
/// da TrackFlow.log (dove finiscono tutte le righe di tutti i watcher
/// insieme, vedi spawn_stdout_drain in lib.rs): richiesta esplicita
/// dell'utente ("intendo il log di Excel solo"), un file dedicato invece
/// di dover filtrare a mano tra migliaia di righe di screenshot/AFK/altro
/// per trovare quelle di Excel. Stesso prefisso "detailed-log-<nome>"
/// del file di override (vedi LOG_DETTAGLIATO_OVERRIDE_FILE) per
/// restare facilmente associabili a colpo d'occhio nella cartella dati.
const LOG_DETTAGLIATO_FILE: &str = "detailed-log-aw-watcher-excel.log";

/// Scrive (append, un file testuale dedicato — MAI stdout: sia per
/// tenerlo separato dal log generale sia perché una riga che per
/// coincidenza avesse la forma di un envelope evento finirebbe inserita
/// nel database dal meccanismo di forwarding di lib.rs) tutto il
/// possibile su ogni finestra excel.exe vista in questo giro — richiesta
/// esplicita dell'utente dopo il bug "Trova e sostituisci" dell'issue
/// GitHub #4: poter vedere, riga per riga durante un uso reale
/// prolungato, esattamente cosa il watcher vede ad ogni controllo, non
/// dedurlo da un singolo screenshot isolato. Nessun errore fatale se la
/// scrittura fallisce (es. cartella non ancora pronta): il watcher deve
/// continuare a funzionare comunque, la diagnostica è un extra.
fn log_dettagliato(app_data_dir: &Path, finestre: &[InfoFinestraExcel], aperti_ora: &[String]) {
    use std::io::Write as _;
    let Ok(mut file) =
        std::fs::OpenOptions::new().create(true).append(true).open(app_data_dir.join(LOG_DETTAGLIATO_FILE))
    else {
        return;
    };
    let _ = writeln!(
        file,
        "[{}] === giro: {} finestre excel.exe trovate ===",
        Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        finestre.len()
    );
    for f in finestre {
        let _ = writeln!(
            file,
            "  hwnd={:#x} pid={} titolo={:?} classe={:?} interpretato={:?} focalizzata={} minimizzata={} abilitata={} owner_hwnd={:?} rect={:?} exe={:?} versione_exe={:?}",
            f.hwnd,
            f.pid,
            f.titolo,
            f.classe,
            f.file_interpretato,
            f.focalizzata,
            f.minimizzata,
            f.abilitata,
            f.owner_hwnd,
            f.rect,
            f.percorso_processo,
            f.versione_file,
        );
    }
    let _ = writeln!(file, "  file considerati \"aperti\" questo giro: {aperti_ora:?}");
}

fn main() {
    let args = Args::parse();
    // Niente più suffisso "_<hostname>" — vedi lo stesso commento in
    // aw-watcher-afk-rust/src/main.rs.
    let bucket_id = "aw-watcher-excel".to_string();

    println!(
        "Modalità: {} - poll_interval: {}s",
        if args.testing { "testing" } else { "normale" },
        args.poll_interval
    );
    println!("Bucket: {bucket_id}");

    // Indirizzo di apertura di ogni file attualmente aperto — un file
    // "chiude" (e manda il suo evento) nel primo giro in cui non compare
    // più tra le finestre trovate.
    let mut aperti_da: HashMap<String, DateTime<Utc>> = HashMap::new();

    loop {
        let log_dettagliato_attivo = leggi_log_dettagliato(args.app_data_dir.as_deref());
        let finestre = elenca_finestre_excel();
        let aperti_ora = file_aperti_da_finestre(&finestre);
        let ora = Utc::now();

        if log_dettagliato_attivo {
            // Sicuro chiamare .unwrap() qui: leggi_log_dettagliato torna
            // già false se args.app_data_dir è None, quindi
            // log_dettagliato_attivo=true implica che la cartella esiste.
            log_dettagliato(args.app_data_dir.as_deref().unwrap(), &finestre, &aperti_ora);
        }

        for file in &aperti_ora {
            aperti_da.entry(file.clone()).or_insert(ora);
        }

        aperti_da.retain(|file, apertura| {
            if aperti_ora.contains(file) {
                true
            } else {
                emit_sessione_chiusa(&bucket_id, file, *apertura, ora);
                false
            }
        });

        let intervallo =
            if log_dettagliato_attivo { INTERVALLO_LOG_DETTAGLIATO_SECONDI } else { args.poll_interval };
        thread::sleep(StdDuration::from_secs_f64(intervallo));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finestra_di_prova(titolo: &str) -> InfoFinestraExcel {
        InfoFinestraExcel {
            hwnd: 0,
            pid: 1234,
            titolo: titolo.to_string(),
            classe: "XLMAIN".to_string(),
            percorso_processo: Some("C:\\Program Files\\Microsoft Office\\EXCEL.EXE".to_string()),
            versione_file: None,
            rect: None,
            focalizzata: false,
            minimizzata: false,
            abilitata: true,
            owner_hwnd: None,
            file_interpretato: interpreta_titolo(titolo),
        }
    }

    #[test]
    fn file_aperti_da_finestre_filtra_solo_i_documenti_veri() {
        let finestre = vec![
            finestra_di_prova("Report.xlsx - Excel"),
            finestra_di_prova("Find and Replace"),
            finestra_di_prova("Cash Ledger.xlsx - Excel"),
        ];
        let aperti = file_aperti_da_finestre(&finestre);
        assert_eq!(aperti, vec!["Report.xlsx".to_string(), "Cash Ledger.xlsx".to_string()]);
    }

    #[test]
    fn file_aperti_da_finestre_deduplica_lo_stesso_file() {
        // Es. "Visualizza affiancate" — due finestre per lo stesso file.
        let finestre = vec![finestra_di_prova("Report.xlsx - Excel"), finestra_di_prova("Report.xlsx - Excel")];
        let aperti = file_aperti_da_finestre(&finestre);
        assert_eq!(aperti, vec!["Report.xlsx".to_string()]);
    }

    #[test]
    fn leggi_log_dettagliato_false_senza_cartella_dati() {
        assert!(!leggi_log_dettagliato(None));
    }

    #[test]
    fn leggi_log_dettagliato_false_quando_il_file_manca() {
        let dir = std::env::temp_dir().join(format!("aw-excel-diag-missing-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert!(!leggi_log_dettagliato(Some(&dir)));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn leggi_log_dettagliato_true_quando_il_file_dice_true() {
        let dir = std::env::temp_dir().join(format!("aw-excel-diag-true-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(LOG_DETTAGLIATO_OVERRIDE_FILE), "true\n").unwrap();
        assert!(leggi_log_dettagliato(Some(&dir)));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn interpreta_titolo_file_salvato() {
        let r = interpreta_titolo("Report_ClienteX.xlsx - Excel");
        assert_eq!(r, Some("Report_ClienteX.xlsx".to_string()));
    }

    #[test]
    fn interpreta_titolo_modalita_compatibilita() {
        let r = interpreta_titolo("Report_ClienteX.xls - Excel (Modalità compatibilità)");
        assert_eq!(r, Some("Report_ClienteX.xls".to_string()));
    }

    #[test]
    fn interpreta_titolo_sola_lettura() {
        let r = interpreta_titolo("Report.xlsx - Excel (Sola lettura)");
        assert_eq!(r, Some("Report.xlsx".to_string()));
    }

    #[test]
    fn interpreta_titolo_non_salvato() {
        let r = interpreta_titolo("Cartel1 - Excel");
        assert_eq!(r, Some("Cartel1".to_string()));
    }

    #[test]
    fn interpreta_titolo_bare_nessun_file() {
        let r = interpreta_titolo("Excel");
        assert_eq!(r, None);
    }

    #[test]
    fn interpreta_titolo_vuoto() {
        let r = interpreta_titolo("");
        assert_eq!(r, None);
    }

    #[test]
    fn interpreta_titolo_pattern_sconosciuto_viene_ignorato() {
        // Bug reale (issue GitHub #4): un titolo senza " - Excel" è
        // quasi sempre una finestra di utilità/dialogo (es. "Find and
        // Replace"), non un documento — va ignorato, non trattato come
        // un file fantasma (vedi il commento in cima al file).
        let r = interpreta_titolo("Qualcosa di inatteso");
        assert_eq!(r, None);
    }

    #[test]
    fn interpreta_titolo_finestra_trova_e_sostituisci_viene_ignorata() {
        // Caso reale osservato dal vivo nell'issue #4: la finestra
        // "Find and Replace" compariva come una riga fasulla a sé nella
        // corsia Excel della Timeline.
        let r = interpreta_titolo("Find and Replace");
        assert_eq!(r, None);
    }

    #[test]
    fn is_excel_exe_riconosce_maiuscole_minuscole() {
        assert!(is_excel_exe("EXCEL.EXE"));
        assert!(is_excel_exe("excel.exe"));
        assert!(!is_excel_exe("winword.exe"));
    }

    // dividi_file_e_modalita: bug reale segnalato dall'utente — il
    // titolo REALE di Excel per un file in sola lettura è tipo
    // "RipeilogoDati_Renergy.xlsx [Sola lettura] - Excel" (indicatore
    // PRIMA di " - Excel", tra QUADRE), non come ipotizzato in
    // interpreta_titolo senza un'installazione Excel vera a
    // disposizione — quindi interpreta_titolo restituisce il
    // descrittore combinato "RipeilogoDati_Renergy.xlsx [Sola
    // lettura]", che va poi separato qui prima di finire nell'evento.
    #[test]
    fn dividi_file_e_modalita_sola_lettura() {
        let (file, modalita) = dividi_file_e_modalita("RipeilogoDati_Renergy.xlsx [Sola lettura]");
        assert_eq!(file, "RipeilogoDati_Renergy.xlsx");
        assert_eq!(modalita, Some("Sola lettura".to_string()));
    }

    #[test]
    fn dividi_file_e_modalita_normale() {
        let (file, modalita) = dividi_file_e_modalita("RipeilogoDati_Renergy.xlsx");
        assert_eq!(file, "RipeilogoDati_Renergy.xlsx");
        assert_eq!(modalita, None);
    }

    #[test]
    fn dividi_file_e_modalita_modalita_protetta() {
        let (file, modalita) = dividi_file_e_modalita("Report.xlsx [Modalità protetta]");
        assert_eq!(file, "Report.xlsx");
        assert_eq!(modalita, Some("Modalità protetta".to_string()));
    }

    // Un nome file che finisce per coincidenza con "]" ma senza una "["
    // corrispondente (o con parentesi vuote) non deve essere spezzato a
    // metà — resta il descrittore intero, nessuna modalità.
    #[test]
    fn dividi_file_e_modalita_niente_parentesi_quadre() {
        let (file, modalita) = dividi_file_e_modalita("Cartel1");
        assert_eq!(file, "Cartel1");
        assert_eq!(modalita, None);
    }

    #[test]
    fn dividi_file_e_modalita_integrazione_con_interpreta_titolo() {
        let descrittore =
            interpreta_titolo("RipeilogoDati_Renergy.xlsx [Sola lettura] - Excel").unwrap();
        let (file, modalita) = dividi_file_e_modalita(&descrittore);
        assert_eq!(file, "RipeilogoDati_Renergy.xlsx");
        assert_eq!(modalita, Some("Sola lettura".to_string()));
    }
}

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
//! Excel installato — l'utente lo testerà sul PC di lavoro. Il parsing
//! del titolo è quindi deliberatamente permissivo invece di provare a
//! coprire ogni variante esatta di formato titolo (modalità
//! compatibilità, sola lettura, cloud/OneDrive, ecc.), che non è stato
//! possibile verificare empiricamente come già fatto per VS Code: cerca
//! la sotto-stringa fissa " - Excel" e taglia lì, così qualunque testo
//! aggiuntivo dopo (" (Modalità compatibilità)", " (Sola lettura)", ...)
//! viene scartato senza bisogno di elencare ogni variante. Se il
//! pattern non si trova (titolo mai visto), usa comunque il titolo
//! intero come nome file invece di scartare l'evento — meglio un
//! raggruppamento imperfetto che perdere dati. Da rivedere con dati
//! reali alla prima sessione di test dell'utente.

use std::collections::HashMap;
use std::io::Write;
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
/// bare "Excel") o titolo vuoto -> None.
fn interpreta_titolo(titolo: &str) -> Option<String> {
    let pulito = titolo.trim();
    if pulito.is_empty() || pulito.eq_ignore_ascii_case("excel") {
        return None;
    }
    match pulito.find(" - Excel") {
        Some(pos) => {
            let file = pulito[..pos].trim();
            if file.is_empty() {
                None
            } else {
                Some(file.to_string())
            }
        }
        // Pattern non riconosciuto: usa comunque il titolo intero
        // piuttosto che scartare l'evento.
        None => Some(pulito.to_string()),
    }
}

#[cfg(windows)]
fn nome_processo(pid: u32) -> Option<String> {
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
        let path = String::from_utf16_lossy(&path_buf[..len as usize]);
        std::path::Path::new(&path)
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
    }
}

/// Elenca i nomi file di TUTTE le finestre Excel aperte in questo momento
/// (indipendentemente da quale abbia il fuoco) — così un file resta
/// "aperto" agli occhi del watcher finché la sua finestra esiste,
/// esattamente come lo intende l'utente, non solo mentre lo si guarda.
#[cfg(windows)]
fn file_excel_aperti() -> Vec<String> {
    use windows::Win32::Foundation::{HWND, LPARAM};
    use windows::core::BOOL;
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
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
        let Some(app) = nome_processo(pid) else {
            return BOOL(1);
        };
        if !is_excel_exe(&app) {
            return BOOL(1);
        }

        if let Some(file) = interpreta_titolo(&titolo) {
            let risultati = unsafe { &mut *(lparam.0 as *mut Vec<String>) };
            if !risultati.contains(&file) {
                risultati.push(file);
            }
        }
        BOOL(1)
    }

    let mut risultati: Vec<String> = Vec::new();
    let lparam = LPARAM(std::ptr::addr_of_mut!(risultati) as isize);
    let _ = unsafe { EnumWindows(Some(callback), lparam) };
    risultati
}

#[cfg(not(windows))]
fn file_excel_aperti() -> Vec<String> {
    compile_error!("aw-watcher-excel supporta solo Windows");
}

/// Manda UN evento completo (non un heartbeat) per una sessione appena
/// chiusa: `apertura` è quando il file è comparso per la prima volta tra
/// le finestre aperte, `chiusura` è adesso (non più trovato). Op
/// "event", non "heartbeat" — inserisce la riga così com'è, senza
/// passare dal meccanismo di fusione heartbeat del backend (che tiene un
/// solo "ultimo evento" per bucket: andrebbe benissimo per un file alla
/// volta ma frammenterebbe la durata se più file venissero tracciati in
/// parallelo, vedi commento in cima al file).
fn emit_sessione_chiusa(bucket_id: &str, file: &str, apertura: DateTime<Utc>, chiusura: DateTime<Utc>) {
    let mut data = Map::new();
    data.insert("file".to_string(), file.into());
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
    /// (di sicuro non 2s) va benissimo e pesa meno sul sistema.
    #[arg(long, default_value_t = 20.0)]
    poll_interval: f64,

    /// Non usato da questo watcher (nessuno stato/file da leggere) —
    /// accettato comunque per uniformità con gli altri watcher, che
    /// vengono tutti lanciati con lo stesso set di argomenti da
    /// src-tauri/src/lib.rs.
    #[arg(long)]
    app_data_dir: Option<std::path::PathBuf>,
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
        let aperti_ora = file_excel_aperti();
        let ora = Utc::now();

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

        thread::sleep(StdDuration::from_secs_f64(args.poll_interval));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn interpreta_titolo_pattern_sconosciuto_usa_titolo_intero() {
        // Formato titolo mai visto (non testabile su questa macchina,
        // niente Excel installato) — fallback: meglio un raggruppamento
        // imperfetto che perdere l'evento.
        let r = interpreta_titolo("Qualcosa di inatteso");
        assert_eq!(r, Some("Qualcosa di inatteso".to_string()));
    }

    #[test]
    fn is_excel_exe_riconosce_maiuscole_minuscole() {
        assert!(is_excel_exe("EXCEL.EXE"));
        assert!(is_excel_exe("excel.exe"));
        assert!(!is_excel_exe("winword.exe"));
    }
}

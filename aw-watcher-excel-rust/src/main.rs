//! Watcher per l'attività in Excel (file aperto) — stessa tecnica di
//! aw-watcher-vscode-rust: nessuna API COM/Office, solo la finestra in
//! primo piano (GetForegroundWindow) e il titolo, perché gira come
//! processo esterno indipendente (coerente con l'architettura "zero
//! Python/zero rete" del progetto), non come plugin/add-in Excel.
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

use std::io::Write;
use std::thread;
use std::time::Duration as StdDuration;

use chrono::{SecondsFormat, Utc};
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
fn finestra_in_primo_piano() -> Option<(String, String)> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId,
    };

    let hwnd: HWND = unsafe { GetForegroundWindow() };
    if hwnd.0.is_null() {
        return None;
    }

    let mut title_buf = [0u16; 1024];
    let len = unsafe { GetWindowTextW(hwnd, &mut title_buf) };
    let titolo = String::from_utf16_lossy(&title_buf[..len.max(0) as usize]);

    let mut pid: u32 = 0;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };

    let app = nome_processo(pid)?;
    Some((app, titolo))
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

#[cfg(not(windows))]
fn finestra_in_primo_piano() -> Option<(String, String)> {
    compile_error!("aw-watcher-excel supporta solo Windows");
}

fn emit(bucket_id: &str, pulsetime: f64, data: Map<String, serde_json::Value>) {
    let envelope = json!({
        "bucket_id": bucket_id,
        "bucket_type": BUCKET_TYPE,
        "client": CLIENT_NAME,
        "op": "heartbeat",
        "pulsetime": pulsetime,
        "event": {
            "timestamp": Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            "duration": 0.0,
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

    /// Ogni quanti secondi controllare la finestra in primo piano.
    #[arg(long, default_value_t = 2.0)]
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
    let hostname = gethostname::gethostname().to_string_lossy().to_string();
    let bucket_id = format!("aw-watcher-excel_{hostname}");
    let pulsetime = (args.poll_interval * 1.5).max(args.poll_interval + 1.0);

    println!(
        "Modalità: {} - poll_interval: {}s",
        if args.testing { "testing" } else { "normale" },
        args.poll_interval
    );
    println!("Bucket: {bucket_id}");

    loop {
        if let Some((app, titolo)) = finestra_in_primo_piano() {
            if is_excel_exe(&app) {
                if let Some(file) = interpreta_titolo(&titolo) {
                    let mut data = Map::new();
                    data.insert("file".to_string(), file.into());
                    emit(&bucket_id, pulsetime, data);
                }
            }
        }
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

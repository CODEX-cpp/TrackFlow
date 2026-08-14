//! Watcher per l'attività in VS Code (file/progetto aperti) — sostituisce
//! l'estensione ufficiale "ActivityWatch for VSCode", che non funziona
//! più da quando il server integrato non apre nessuna porta TCP reale
//! (Fase 5, "zero esposizione di rete": l'estensione parlava via HTTP a
//! localhost:5600, che non esiste più). Trovato il 2026-08-12: il
//! bucket `aw-watcher-vscode_*` non riceveva un evento nuovo dall'8
//! agosto, il giorno del passaggio a quell'architettura — un buco reale
//! della migrazione, non qualcosa di già gestito.
//!
//! Non essendo un'estensione VS Code vera (gira come processo esterno,
//! come tutti gli altri watcher di questo progetto), non ha accesso
//! diretto all'editor per sapere quale file/cartella sia aperto: usa
//! invece lo stesso identico meccanismo di aw-watcher-window-rust
//! (finestra in primo piano) e legge il titolo — VS Code lo compone
//! sempre come "<file> - <progetto> - Visual Studio Code" (o solo
//! "<file> - Visual Studio Code" senza cartella aperta), stessa
//! convenzione già usata lato webui in appNames.ts's
//! vscodeTitleDisplayName(). Emette solo quando la finestra in primo
//! piano è davvero VS Code — nessun heartbeat altrimenti.
//!
//! Compromesso consapevole rispetto all'estensione vera: il titolo dà
//! solo il nome del file (non il percorso completo) e il nome della
//! cartella (non il percorso completo del progetto) — comunque
//! sufficiente per raggruppare/classificare correttamente in "Top
//! Editor Projects"/"Top Editor Files" (la query webui raggruppa per
//! file+language+project insieme, quindi due file omonimi in progetti
//! diversi restano distinti). "language" è dedotto dall'estensione del
//! file con una mappa fissa, non letto dal vero language mode di VS
//! Code (che un processo esterno non può vedere).

use std::io::Write;
use std::thread;
use std::time::Duration as StdDuration;

use chrono::{SecondsFormat, Utc};
use clap::Parser;
use serde_json::{json, Map};

const CLIENT_NAME: &str = "aw-watcher-vscode";
const BUCKET_TYPE: &str = "app.editor.activity";

/// Nomi exe delle varianti di VS Code che riconosciamo — Stable e
/// Insiders (stesso schema exe->titolo, solo il nome del processo e la
/// coda "- Insiders" nel titolo cambiano).
fn is_vscode_exe(app: &str) -> bool {
    let normalizzato = app.to_ascii_lowercase();
    normalizzato == "code.exe" || normalizzato == "code - insiders.exe"
}

/// "<file> - <progetto> - Visual Studio Code[ - Insiders]" (progetto
/// aperto), "<file> - Visual Studio Code[ - Insiders]" (solo file,
/// nessuna cartella), o titolo bare (niente aperto) — stessa euristica
/// di vscodeTitleDisplayName() in src/util/appNames.ts, va tenuta
/// identica se quella cambia.
struct TitoloVSCode {
    file: Option<String>,
    progetto: Option<String>,
}

fn interpreta_titolo(titolo: &str) -> TitoloVSCode {
    // Toglie la coda fissa "Visual Studio Code[ - Insiders]" dal titolo
    // GREZZO prima di spezzare per " - ": Insiders aggiunge un ulteriore
    // segmento "Insiders" in fondo, che altrimenti verrebbe scambiato
    // per il nome del progetto.
    let senza_suffisso = titolo
        .strip_suffix(" - Visual Studio Code - Insiders")
        .or_else(|| titolo.strip_suffix(" - Visual Studio Code"))
        .unwrap_or(titolo);

    let utili: Vec<&str> = if senza_suffisso.is_empty()
        || senza_suffisso == "Visual Studio Code"
        || senza_suffisso == "Visual Studio Code - Insiders"
    {
        Vec::new()
    } else {
        senza_suffisso.split(" - ").map(|p| p.trim()).collect()
    };

    match utili.len() {
        0 => TitoloVSCode { file: None, progetto: None },
        1 => TitoloVSCode { file: Some(utili[0].to_string()), progetto: None },
        _ => TitoloVSCode {
            file: Some(utili[0].to_string()),
            progetto: Some(utili[utili.len() - 1].to_string()),
        },
    }
}

/// Mappa best-effort estensione->linguaggio — un processo esterno non
/// può leggere il vero "language mode" scelto in VS Code (che può
/// differire dall'estensione, es. un file .txt impostato manualmente
/// come "python"), solo approssimarlo dal nome del file.
fn linguaggio_da_estensione(file: &str) -> String {
    let estensione = std::path::Path::new(file)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match estensione.as_str() {
        "vue" => "vue",
        "ts" => "typescript",
        "tsx" => "typescriptreact",
        "js" => "javascript",
        "jsx" => "javascriptreact",
        "rs" => "rust",
        "py" => "python",
        "md" => "markdown",
        "json" => "json",
        "css" => "css",
        "scss" => "scss",
        "html" => "html",
        "toml" => "toml",
        "yml" | "yaml" => "yaml",
        "sh" => "shellscript",
        "sql" => "sql",
        "c" => "c",
        "cpp" | "cc" => "cpp",
        "cs" => "csharp",
        "go" => "go",
        "java" => "java",
        "" => "unknown",
        other => other,
    }
    .to_string()
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
    compile_error!("aw-watcher-vscode supporta solo Windows");
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
#[command(about = "Watcher per l'attività in VS Code (file/progetto), letta dal titolo della finestra")]
struct Args {
    /// Gira in modalità test: dati separati da quelli reali (accettato
    /// per uniformità con gli altri watcher, non cambia comportamento:
    /// questo watcher non parla mai in rete).
    #[arg(long)]
    testing: bool,

    /// Ogni quanti secondi controllare la finestra in primo piano
    /// (default: 2 — stesso ordine di grandezza del watcher finestra
    /// generico, l'attività di editing cambia comunque più lentamente
    /// dei singoli secondi).
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
    let bucket_id = format!("aw-watcher-vscode_{hostname}");
    let pulsetime = (args.poll_interval * 1.5).max(args.poll_interval + 1.0);

    println!(
        "Modalità: {} - poll_interval: {}s",
        if args.testing { "testing" } else { "normale" },
        args.poll_interval
    );
    println!("Bucket: {bucket_id}");

    loop {
        if let Some((app, titolo)) = finestra_in_primo_piano() {
            if is_vscode_exe(&app) {
                let interpretato = interpreta_titolo(&titolo);
                if let Some(file) = interpretato.file {
                    let mut data = Map::new();
                    data.insert("file".to_string(), file.clone().into());
                    data.insert("language".to_string(), linguaggio_da_estensione(&file).into());
                    data.insert(
                        "project".to_string(),
                        interpretato.progetto.unwrap_or_else(|| "unknown".to_string()).into(),
                    );
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
    fn interpreta_titolo_con_progetto() {
        let r = interpreta_titolo("App.vue - aw-webui - Visual Studio Code");
        assert_eq!(r.file, Some("App.vue".to_string()));
        assert_eq!(r.progetto, Some("aw-webui".to_string()));
    }

    #[test]
    fn interpreta_titolo_con_progetto_insiders() {
        let r = interpreta_titolo("App.vue - aw-webui - Visual Studio Code - Insiders");
        assert_eq!(r.file, Some("App.vue".to_string()));
        assert_eq!(r.progetto, Some("aw-webui".to_string()));
    }

    #[test]
    fn interpreta_titolo_solo_file() {
        let r = interpreta_titolo("App.vue - Visual Studio Code");
        assert_eq!(r.file, Some("App.vue".to_string()));
        assert_eq!(r.progetto, None);
    }

    #[test]
    fn interpreta_titolo_vuoto() {
        let r = interpreta_titolo("Visual Studio Code");
        assert_eq!(r.file, None);
        assert_eq!(r.progetto, None);
    }

    #[test]
    fn interpreta_titolo_con_file_non_salvato_asterisco() {
        // VS Code antepone "● " al nome file quando ci sono modifiche
        // non salvate — resta parte del segmento "file" così com'è,
        // nessuna pulizia speciale necessaria per l'aggregazione (lo
        // stesso file con/senza asterisco creerebbe comunque due
        // chiavi diverse, comportamento accettato per ora).
        let r = interpreta_titolo("● App.vue - aw-webui - Visual Studio Code");
        assert_eq!(r.file, Some("● App.vue".to_string()));
    }

    #[test]
    fn linguaggio_da_estensione_vue() {
        assert_eq!(linguaggio_da_estensione("App.vue"), "vue");
    }

    #[test]
    fn linguaggio_da_estensione_sconosciuta_ma_presente() {
        assert_eq!(linguaggio_da_estensione("Makefile.xyz123"), "xyz123");
    }

    #[test]
    fn linguaggio_da_estensione_assente() {
        assert_eq!(linguaggio_da_estensione("Dockerfile"), "unknown");
    }

    #[test]
    fn is_vscode_exe_riconosce_stable_e_insiders() {
        assert!(is_vscode_exe("Code.exe"));
        assert!(is_vscode_exe("code.exe"));
        assert!(is_vscode_exe("Code - Insiders.exe"));
        assert!(!is_vscode_exe("chrome.exe"));
    }
}

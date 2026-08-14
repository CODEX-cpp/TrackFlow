//! Traccia quali app di terze parti hanno un'icona nella system tray
//! (area notifiche vicino all'orologio) in un dato momento — non i
//! controlli di sistema (volume, rete, orologio, ecc.) e non i pulsanti
//! della taskbar per le finestre aperte (già coperti da
//! aw-watcher-window).
//!
//! Windows non offre un'API pubblica per elencare le icone tray di
//! *altri* processi. La tecnica usata qui è la stessa impiegata da
//! Narrator e dalle utility che elencano le icone tray: UI Automation
//! sull'elemento Shell_TrayWnd (la taskbar), filtrando i bottoni per
//! AutomationId. Verificato empiricamente (dump completo dell'albero
//! UIA reale su questo PC) che Explorer distingue già da sé, tramite
//! l'AutomationId:
//!   - "NotifyItemIcon"  -> icona di un'app di terze parti (quello che
//!                          vogliamo)
//!   - "SystemTrayIcon"  -> controllo di sistema integrato in Windows
//!                          (volume, rete, orologio, "mostra icone
//!                          nascoste", "mostra desktop", indicatori
//!                          privacy) -> escluso
//!   - class
//!     "Taskbar.TaskListButtonAutomationPeer" -> pulsante taskbar per
//!                          una finestra aperta -> escluso (dominio di
//!                          aw-watcher-window/Top Applications)
#![cfg(windows)]

use std::collections::HashMap;
use std::thread;
use std::time::Duration as StdDuration;

use chrono::{DateTime, SecondsFormat, Utc};
use clap::Parser;
use serde_json::{json, Map};
use windows::core::w;
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::Accessibility::{
    IUIAutomation, IUIAutomationElement, CUIAutomation, TreeScope_Descendants,
};
use windows::Win32::UI::WindowsAndMessaging::FindWindowW;

const CLIENT_NAME: &str = "aw-watcher-tray";
const BUCKET_ID: &str = "tray-apps";
const BUCKET_TYPE: &str = "app.background.presence";

/// Stampa una riga JSON su stdout — il processo Tauri che ci ha lanciato
/// legge questa pipe e inoltra l'evento al server in-process (stesso
/// contratto di tutti gli altri watcher, vedi BLUEPRINT.md Fase 5).
fn emit(op: &str, pulsetime: Option<f64>, timestamp: DateTime<Utc>, duration_seconds: f64, data: Map<String, serde_json::Value>) {
    let mut envelope = json!({
        "bucket_id": BUCKET_ID,
        "bucket_type": BUCKET_TYPE,
        "client": CLIENT_NAME,
        "op": op,
        "event": {
            "timestamp": timestamp.to_rfc3339_opts(SecondsFormat::Millis, true),
            "duration": duration_seconds,
            "data": data,
        },
    });
    if let Some(p) = pulsetime {
        envelope["pulsetime"] = json!(p);
    }
    use std::io::Write;
    let mut stdout = std::io::stdout();
    let _ = writeln!(stdout, "{envelope}");
    let _ = stdout.flush();
}

/// Deve restare più alto dell'intervallo di poll, altrimenti due
/// controlli consecutivi con la stessa icona presente verrebbero
/// considerati sessioni diverse invece che una che continua.
const HEARTBEAT_MARGIN_SECONDS: i64 = 15;

/// Molte icone di terze parti (monitor hardware, stato sync, ecc.)
/// scrivono un tooltip tipo "Etichetta: valore che cambia" — usarlo
/// così com'è come identità dell'app farebbe sembrare una sessione
/// diversa ad ogni giro di poll (verificato empiricamente: un monitor
/// di temperatura CPU cambia il tooltip letteralmente ogni pochi
/// secondi). Taglia alla prima ":" e tiene solo l'etichetta, stabile
/// nel tempo — euristica, non perfetta per ogni icona possibile, ma
/// copre il pattern più comune. Da rifinire in futuro se emergono altri
/// pattern instabili.
fn normalizza_nome_icona(nome: &str) -> String {
    let etichetta = nome.split(':').next().unwrap_or(nome);
    etichetta.trim().to_string()
}

unsafe fn nomi_icone_tray(automation: &IUIAutomation, root: &IUIAutomationElement, out: &mut Vec<String>) {
    let Ok(condizione_vera) = automation.CreateTrueCondition() else { return };
    let Ok(tutti) = root.FindAll(TreeScope_Descendants, &condizione_vera) else { return };
    let count = tutti.Length().unwrap_or(0);
    for i in 0..count {
        let Ok(el) = tutti.GetElement(i) else { continue };
        let autoid = el.CurrentAutomationId().map(|s| s.to_string()).unwrap_or_default();
        if autoid != "NotifyItemIcon" {
            continue;
        }
        if let Ok(nome) = el.CurrentName() {
            let nome = normalizza_nome_icona(&nome.to_string());
            if !nome.is_empty() {
                out.push(nome);
            }
        }
    }
}

/// Un giro di scansione: nomi delle icone attualmente presenti (visibili
/// + nascoste dietro "Mostra icone nascoste"), deduplicati.
unsafe fn scansiona(automation: &IUIAutomation) -> Vec<String> {
    let mut nomi = Vec::new();

    if let Ok(hwnd) = FindWindowW(w!("Shell_TrayWnd"), None) {
        if !hwnd.is_invalid() {
            if let Ok(root) = automation.ElementFromHandle(hwnd) {
                nomi_icone_tray(automation, &root, &mut nomi);
            }
        }
    }
    // Icone nascoste nel flyout overflow: finestra separata, esiste già
    // (anche se non visibile) senza bisogno di aprire il "^".
    if let Ok(hwnd) = FindWindowW(w!("NotifyIconOverflowWindow"), None) {
        if !hwnd.is_invalid() {
            if let Ok(root) = automation.ElementFromHandle(hwnd) {
                nomi_icone_tray(automation, &root, &mut nomi);
            }
        }
    }

    nomi.sort();
    nomi.dedup();
    nomi
}

struct TrayWatcher {
    heartbeat_pulsetime: f64,
    /// app -> quando è comparsa la sessione ancora aperta (icona ancora
    /// presente all'ultimo giro di scansione).
    sessioni_aperte: HashMap<String, DateTime<Utc>>,
}

impl TrayWatcher {
    fn new(poll_interval_seconds: i64) -> Self {
        TrayWatcher {
            heartbeat_pulsetime: (poll_interval_seconds + HEARTBEAT_MARGIN_SECONDS) as f64,
            sessioni_aperte: HashMap::new(),
        }
    }

    fn aggiorna(&mut self, presenti_ora: Vec<String>) {
        let adesso = Utc::now();
        let presenti_ora: std::collections::HashSet<String> = presenti_ora.into_iter().collect();

        // Nuove: comparse da questo giro, sessione aperta ora.
        for app in &presenti_ora {
            if !self.sessioni_aperte.contains_key(app) {
                self.sessioni_aperte.insert(app.clone(), adesso);
            }
        }

        // Ancora presenti: heartbeat per tenerle "vive" in dashboard.
        for app in &presenti_ora {
            let mut data = Map::new();
            data.insert("app".to_string(), app.clone().into());
            emit("heartbeat", Some(self.heartbeat_pulsetime), adesso, 0.0, data);
        }

        // Sparite da questo giro: chiudi la sessione con la durata reale.
        let sparite: Vec<String> = self
            .sessioni_aperte
            .keys()
            .filter(|app| !presenti_ora.contains(*app))
            .cloned()
            .collect();
        for app in sparite {
            if let Some(inizio) = self.sessioni_aperte.remove(&app) {
                let durata = adesso - inizio;
                if durata.num_milliseconds() > 0 {
                    let mut data = Map::new();
                    data.insert("app".to_string(), app.into());
                    let durata_secondi = durata.num_milliseconds() as f64 / 1000.0;
                    emit("event", None, inizio, durata_secondi, data);
                }
            }
        }
    }
}

#[derive(Parser)]
#[command(about = "Watcher per le icone di app di terze parti nella system tray")]
struct Args {
    /// Gira in modalità test: porta 5666, dati separati da quelli reali
    #[arg(long)]
    testing: bool,

    /// Ogni quanti secondi controllare le icone presenti (default: 10)
    #[arg(long, default_value_t = 10)]
    poll_interval: i64,

    /// Non usato da questo watcher (nessuno stato/file da scrivere per
    /// ora) — accettato comunque per uniformità con gli altri watcher,
    /// che vengono tutti lanciati con lo stesso set di argomenti da
    /// src-tauri/src/lib.rs.
    #[arg(long)]
    app_data_dir: Option<std::path::PathBuf>,
}

fn main() {
    let args = Args::parse();

    println!(
        "Modalità: {}",
        if args.testing { "testing (porta 5666)" } else { "normale (porta 5600)" }
    );
    println!("Controllo le icone tray ogni {}s", args.poll_interval);

    unsafe {
        if CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_err() {
            eprintln!("CoInitializeEx fallita, esco");
            return;
        }
        let automation: IUIAutomation = match CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER) {
            Ok(a) => a,
            Err(e) => {
                eprintln!("CoCreateInstance IUIAutomation fallita: {e:?}, esco");
                return;
            }
        };

        let mut watcher = TrayWatcher::new(args.poll_interval);
        loop {
            let presenti = scansiona(&automation);
            watcher.aggiorna(presenti);
            thread::sleep(StdDuration::from_secs(args.poll_interval as u64));
        }
    }
}

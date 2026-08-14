//! Esplorativo: apre il pannello "icone nascoste" via UI Automation
//! (Invoke sul bottone), legge le icone mentre è aperto, poi lo
//! richiude — per verificare se il contenuto è disponibile solo a
//! popup aperto (ipotesi: sì, dato che NotifyIconOverflowWindow non
//! esiste nemmeno finché non lo si apre almeno una volta).
#![cfg(windows)]
use std::thread;
use std::time::Duration;

use windows::core::w;
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationElement, IUIAutomationInvokePattern,
    TreeScope_Descendants, UIA_InvokePatternId,
};
use windows::Win32::UI::WindowsAndMessaging::FindWindowW;

unsafe fn trova_bottone_mostra_nascoste(automation: &IUIAutomation) -> Option<IUIAutomationElement> {
    let shell_tray = FindWindowW(w!("Shell_TrayWnd"), None).ok()?;
    if shell_tray.is_invalid() {
        return None;
    }
    let root = automation.ElementFromHandle(shell_tray).ok()?;
    let condizione_vera = automation.CreateTrueCondition().ok()?;
    let tutti = root.FindAll(TreeScope_Descendants, &condizione_vera).ok()?;
    let count = tutti.Length().unwrap_or(0);
    for i in 0..count {
        let Ok(el) = tutti.GetElement(i) else { continue };
        if let Ok(nome) = el.CurrentName() {
            if nome.to_string() == "Mostra icone nascoste" {
                return Some(el);
            }
        }
    }
    None
}

unsafe fn conta_e_stampa(automation: &IUIAutomation, hwnd_class: &windows::core::PCWSTR) {
    match FindWindowW(*hwnd_class, None) {
        Ok(hwnd) if !hwnd.is_invalid() => {
            println!("finestra trovata: {hwnd:?}");
            if let Ok(root) = automation.ElementFromHandle(hwnd) {
                if let Ok(condizione_vera) = automation.CreateTrueCondition() {
                    if let Ok(tutti) = root.FindAll(TreeScope_Descendants, &condizione_vera) {
                        let count = tutti.Length().unwrap_or(0);
                        println!("elementi trovati: {count}");
                        for i in 0..count {
                            if let Ok(el) = tutti.GetElement(i) {
                                let autoid = el.CurrentAutomationId().map(|s| s.to_string()).unwrap_or_default();
                                let nome = el.CurrentName().map(|s| s.to_string()).unwrap_or_default();
                                if autoid == "NotifyItemIcon" && !nome.is_empty() {
                                    println!("  ICONA: {nome:?}");
                                }
                            }
                        }
                    }
                }
            }
        }
        other => println!("finestra non trovata: {other:?}"),
    }
}

fn main() {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let automation: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).unwrap();

        println!("--- PRIMA di aprire il pannello ---");
        conta_e_stampa(&automation, &w!("NotifyIconOverflowWindow"));

        println!("--- Apro il pannello 'Mostra icone nascoste' ---");
        if let Some(bottone) = trova_bottone_mostra_nascoste(&automation) {
            if let Ok(invoke) = bottone.GetCurrentPatternAs::<IUIAutomationInvokePattern>(UIA_InvokePatternId) {
                let _ = invoke.Invoke();
                println!("Invoke() chiamato");
            } else {
                println!("Nessun pattern Invoke sul bottone");
            }
        } else {
            println!("Bottone 'Mostra icone nascoste' non trovato");
        }

        thread::sleep(Duration::from_millis(600));

        println!("--- DOPO aver aperto il pannello ---");
        conta_e_stampa(&automation, &w!("NotifyIconOverflowWindow"));

        // Richiudi: un secondo Invoke sullo stesso bottone di solito
        // fa da toggle (comportamento standard di questi controlli).
        if let Some(bottone) = trova_bottone_mostra_nascoste(&automation) {
            if let Ok(invoke) = bottone.GetCurrentPatternAs::<IUIAutomationInvokePattern>(UIA_InvokePatternId) {
                let _ = invoke.Invoke();
                println!("--- Richiuso ---");
            }
        }
    }
}

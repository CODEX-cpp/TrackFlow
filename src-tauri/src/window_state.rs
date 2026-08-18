//! Ricorda posizione, dimensione e schermo della finestra principale tra
//! un avvio e l'altro — richiesta esplicita: creare la finestra a mano
//! via `WebviewWindowBuilder` (invece che dichiararla in tauri.conf.json)
//! significa che Windows non lo fa da solo, quindi riapriva sempre sullo
//! schermo principale invece che dove l'utente l'aveva lasciata l'ultima
//! volta, anche spostandola su un secondo monitor.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::WebviewWindow;

const NOME_FILE: &str = "window-state.json";

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct StatoFinestra {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

fn percorso_file(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(NOME_FILE)
}

pub fn carica(app_data_dir: &Path) -> Option<StatoFinestra> {
    let contenuto = std::fs::read_to_string(percorso_file(app_data_dir)).ok()?;
    serde_json::from_str(&contenuto).ok()
}

pub fn salva(app_data_dir: &Path, stato: &StatoFinestra) {
    if let Ok(testo) = serde_json::to_string(stato) {
        let _ = std::fs::write(percorso_file(app_data_dir), testo);
    }
}

/// True se il rettangolo salvato ricade (anche solo in parte) dentro uno
/// degli schermi attualmente collegati — protezione contro un monitor
/// secondario scollegato dall'ultima sessione, che altrimenti lascerebbe
/// la finestra fuori da qualunque schermo visibile e irraggiungibile
/// (niente barra del titolo nativa da trascinare per recuperarla, vedi
/// `.decorations(false)` in lib.rs).
pub fn dentro_uno_schermo(window: &WebviewWindow, stato: &StatoFinestra) -> bool {
    let Ok(monitors) = window.available_monitors() else {
        return false;
    };
    monitors.iter().any(|m| {
        let mp = m.position();
        let ms = m.size();
        let sx = stato.x;
        let sy = stato.y;
        let ex = stato.x + stato.width as i32;
        let ey = stato.y + stato.height as i32;
        let mx = mp.x;
        let my = mp.y;
        let mex = mp.x + ms.width as i32;
        let mey = mp.y + ms.height as i32;
        // Sovrapposizione, non contenimento totale — basta un angolo
        // visibile su uno schermo collegato per poter riprendere in mano
        // la finestra da lì.
        sx < mex && ex > mx && sy < mey && ey > my
    })
}

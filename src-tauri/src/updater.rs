//! Controllo/download/verifica/installazione degli aggiornamenti — vedi
//! `UpdatePopup.vue` e `stores/updater.ts` lato frontend, `launcher/src/
//! main.rs` e `installer/trackflow-installer.nsi` per come questo modulo
//! si incastra nel resto del sistema di aggiornamento in stile Squirrel
//! (cartelle `versions/<versione>/`, `current.txt`, `launcher.exe`
//! stabile).
//!
//! Una release va pubblicata su GitHub con ESATTAMENTE questi due asset
//! (oltre al sorgente automatico e all'installer `trackflow-setup-
//! <versione>.exe`, quest'ultimo per chi installa da zero, non coinvolto
//! nell'auto-update):
//!   trackflow-<versione>-update-package.zip      — stesso contenuto che
//!                                    l'installer mette in
//!                                    versions/<versione>/: app.exe, i
//!                                    watcher, dist/, icons/ (root dello
//!                                    zip, non dentro una sottocartella).
//!                                    Nome scelto apposta ("-update-
//!                                    package", non solo "trackflow-
//!                                    <versione>.zip") per non farlo
//!                                    sembrare una versione "portable"
//!                                    dell'app pronta all'uso a chi
//!                                    scorre gli asset della release —
//!                                    è un pacchetto interno per
//!                                    l'updater, non va estratto ed
//!                                    eseguito a mano.
//!   trackflow-<versione>-update-package.zip.sig  — firma, generata con
//!                                    `tauri signer sign -f
//!                                    ~/.trackflow-updater.key
//!                                    <file>.zip`. ATTENZIONE: il file
//!                                    `.sig` prodotto da `tauri signer`
//!                                    non è il testo minisign in chiaro —
//!                                    è quel testo ULTERIORMENTE
//!                                    codificato in base64 (verificato
//!                                    empiricamente: è lo stesso
//!                                    contenuto stampato a schermo come
//!                                    "Public signature" dal comando di
//!                                    firma) — va decodificato prima di
//!                                    passarlo a `Signature::decode`,
//!                                    vedi `scarica_e_prepara_bloccante`.
//! Il tag della release deve essere "v<versione>" (es. "v0.2.0").

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use minisign_verify::{PublicKey, Signature};
use serde::Serialize;
use tauri::AppHandle;

const GITHUB_OWNER: &str = "CODEX-cpp";
const GITHUB_REPO: &str = "TrackFlow";

/// Chiave pubblica minisign (~/.trackflow-updater.key.pub) — generata una
/// tantum con `tauri signer generate`, la privata resta SOLO su questo
/// computer. Pubblica per definizione: incorporarla nel codice è corretto
/// e necessario, è solo quella privata a dover restare segreta.
const CHIAVE_PUBBLICA_B64: &str = "RWTCXQAkrmctYNP6ePqX0/R0kxdTUmgftsy38h614k9SrjP7IPEFUWW/";

#[derive(Debug, Clone, Serialize)]
pub struct InfoAggiornamento {
    pub versione: String,
    pub url_asset: String,
    pub url_firma: String,
}

/// `app.exe` vive in `<INSTDIR>\versions\<versione>\app.exe` (vedi
/// `trackflow-installer.nsi`) — tre `parent()` per risalire dalla sua
/// posizione alla radice dell'installazione, la stessa cartella dove
/// `launcher.exe` legge `current.txt`.
pub(crate) fn cartella_installazione() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    exe.parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| "Impossibile risalire alla cartella di installazione".to_string())
}

fn versione_da_tag(tag: &str) -> &str {
    tag.strip_prefix('v').unwrap_or(tag)
}

fn controlla_aggiornamento_bloccante() -> Result<Option<InfoAggiornamento>, String> {
    let url = format!("https://api.github.com/repos/{GITHUB_OWNER}/{GITHUB_REPO}/releases/latest");
    let risposta = ureq::get(&url)
        // L'API di GitHub rifiuta le richieste senza uno User-Agent.
        .set("User-Agent", "TrackFlow-Updater")
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| format!("controllo aggiornamenti fallito: {e}"))?;
    let corpo: serde_json::Value =
        risposta.into_json().map_err(|e| format!("risposta GitHub non valida: {e}"))?;

    let tag = corpo["tag_name"].as_str().ok_or_else(|| "release senza tag_name".to_string())?;
    let versione_remota_str = versione_da_tag(tag);
    let versione_remota = semver::Version::parse(versione_remota_str)
        .map_err(|e| format!("versione remota non valida ({versione_remota_str}): {e}"))?;
    let versione_locale = semver::Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|e| format!("versione locale non valida: {e}"))?;

    if versione_remota <= versione_locale {
        return Ok(None);
    }

    let nome_zip = format!("trackflow-{versione_remota_str}-update-package.zip");
    let nome_firma = format!("{nome_zip}.sig");
    let assets = corpo["assets"].as_array().cloned().unwrap_or_default();
    let trova_asset = |nome: &str| -> Option<String> {
        assets
            .iter()
            .find(|a| a["name"].as_str() == Some(nome))
            .and_then(|a| a["browser_download_url"].as_str())
            .map(str::to_string)
    };
    let url_asset = trova_asset(&nome_zip)
        .ok_or_else(|| format!("release {tag} non contiene l'asset atteso {nome_zip}"))?;
    let url_firma = trova_asset(&nome_firma)
        .ok_or_else(|| format!("release {tag} non contiene la firma attesa {nome_firma}"))?;

    Ok(Some(InfoAggiornamento { versione: versione_remota_str.to_string(), url_asset, url_firma }))
}

#[tauri::command]
pub async fn controlla_aggiornamento() -> Result<Option<InfoAggiornamento>, String> {
    tokio::task::spawn_blocking(controlla_aggiornamento_bloccante)
        .await
        .map_err(|e| format!("errore interno: {e}"))?
}

fn scarica_bytes_bloccante(url: &str) -> Result<Vec<u8>, String> {
    let risposta = ureq::get(url)
        .set("User-Agent", "TrackFlow-Updater")
        .call()
        .map_err(|e| format!("download fallito ({url}): {e}"))?;
    let mut buf = Vec::new();
    risposta
        .into_reader()
        .read_to_end(&mut buf)
        .map_err(|e| format!("lettura del download fallita: {e}"))?;
    Ok(buf)
}

fn verifica_firma(dati: &[u8], firma_testo: &str) -> Result<(), String> {
    let chiave = PublicKey::from_base64(CHIAVE_PUBBLICA_B64)
        .map_err(|e| format!("chiave pubblica dell'updater non valida: {e}"))?;
    let firma = Signature::decode(firma_testo).map_err(|e| format!("firma non valida: {e}"))?;
    chiave
        // false = rifiuta firme "legacy" (pre-hashed) — le release firmate
        // con `tauri signer sign` moderno usano sempre il formato attuale.
        .verify(dati, &firma, false)
        .map_err(|_| "firma dell'aggiornamento non valida — download scartato".to_string())
}

/// `enclosed_name()` (non `name()`/`mangled_name()`) rifiuta di suo path
/// assoluti o con `..` — protezione zip-slip già inclusa nella libreria,
/// non serve reimplementarla qui.
fn estrai_zip_in(dati: &[u8], destinazione: &Path) -> Result<(), String> {
    let mut archivio =
        zip::ZipArchive::new(Cursor::new(dati)).map_err(|e| format!("archivio non valido: {e}"))?;
    std::fs::create_dir_all(destinazione).map_err(|e| e.to_string())?;
    for i in 0..archivio.len() {
        let mut voce = archivio.by_index(i).map_err(|e| e.to_string())?;
        let Some(percorso_relativo) = voce.enclosed_name() else { continue };
        let percorso = destinazione.join(percorso_relativo);
        if voce.is_dir() {
            std::fs::create_dir_all(&percorso).map_err(|e| e.to_string())?;
        } else {
            if let Some(genitore) = percorso.parent() {
                std::fs::create_dir_all(genitore).map_err(|e| e.to_string())?;
            }
            let mut file = std::fs::File::create(&percorso).map_err(|e| e.to_string())?;
            std::io::copy(&mut voce, &mut file).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Scarica+verifica+estrae in `versions/<versione>/` — NON tocca
/// `current.txt`: quello resta invariato finché l'utente non conferma
/// esplicitamente il riavvio (vedi `installa_aggiornamento_e_riavvia`),
/// così la versione in esecuzione non cambia mai sotto i piedi
/// dell'utente solo perché il download è finito in background.
fn scarica_e_prepara_bloccante(versione: &str, url_asset: &str, url_firma: &str) -> Result<(), String> {
    use base64::Engine;
    let dati_zip = scarica_bytes_bloccante(url_asset)?;
    // Il file .sig prodotto da `tauri signer sign` è il testo minisign
    // codificato in base64 (non il testo in chiaro) — vedi commento in
    // testa al file.
    let firma_b64 = scarica_bytes_bloccante(url_firma)?;
    let firma_bytes = base64::engine::general_purpose::STANDARD
        .decode(&firma_b64)
        .map_err(|e| format!("firma non decodificabile (base64): {e}"))?;
    let firma_testo = String::from_utf8(firma_bytes).map_err(|e| format!("firma non leggibile: {e}"))?;
    verifica_firma(&dati_zip, &firma_testo)?;

    let installazione = cartella_installazione()?;
    let destinazione = installazione.join("versions").join(versione);
    estrai_zip_in(&dati_zip, &destinazione)
}

#[tauri::command]
pub async fn scarica_e_prepara_aggiornamento(
    versione: String,
    url_asset: String,
    url_firma: String,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || scarica_e_prepara_bloccante(&versione, &url_asset, &url_firma))
        .await
        .map_err(|e| format!("errore interno: {e}"))?
}

/// Click su "Riavvia per aggiornare" — sposta il puntatore su una
/// versione GIÀ scaricata e verificata (da `scarica_e_prepara_aggiornamento`,
/// mai su una a caso), poi rilancia tramite `launcher.exe` (mai
/// direttamente `app.exe`: è l'unico punto d'ingresso a cui restano
/// puntati i collegamenti Menu Start/Desktop, vedi launcher/src/main.rs) e
/// chiude il processo corrente.
#[tauri::command]
pub fn installa_aggiornamento_e_riavvia(app_handle: AppHandle, versione: String) -> Result<(), String> {
    let installazione = cartella_installazione()?;
    let percorso_app_nuova = installazione.join("versions").join(&versione).join("app.exe");
    if !percorso_app_nuova.exists() {
        return Err(format!(
            "versione {versione} non trovata in {} — scaricala prima con scarica_e_prepara_aggiornamento",
            percorso_app_nuova.display()
        ));
    }
    std::fs::write(installazione.join("current.txt"), &versione).map_err(|e| e.to_string())?;

    let percorso_launcher = installazione.join("launcher.exe");
    #[cfg(windows)]
    {
        // CREATE_BREAKAWAY_FROM_JOB — senza, launcher.exe entrerebbe nel
        // Job Object KILL_ON_JOB_CLOSE di questo stesso processo (vedi
        // setup_kill_on_exit_job() in lib.rs, pensato per uccidere i
        // sidecar watcher orfani) e verrebbe ucciso insieme a noi
        // nell'istante di app_handle.exit() qui sotto — un attimo prima
        // di riuscire ad avviare la versione nuova. Bug reale: il
        // riavvio automatico non appariva mai, l'utente doveva riaprire
        // l'app a mano (funzionava già correttamente una volta riaperta,
        // current.txt era già aggiornato — solo il rilancio automatico
        // falliva silenziosamente).
        use std::os::windows::process::CommandExt;
        const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;
        std::process::Command::new(&percorso_launcher)
            .creation_flags(CREATE_BREAKAWAY_FROM_JOB)
            .spawn()
            .map_err(|e| format!("avvio del launcher fallito: {e}"))?;
    }
    #[cfg(not(windows))]
    std::process::Command::new(&percorso_launcher)
        .spawn()
        .map_err(|e| format!("avvio del launcher fallito: {e}"))?;

    app_handle.exit(0);
    Ok(())
}

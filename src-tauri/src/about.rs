//! Impostazioni → Info: scarica il changelog dal vivo da GitHub Pages
//! (docs/CHANGELOG.md nel repository, pubblicato su
//! https://codex-cpp.github.io/TrackFlow/) invece di impacchettarlo in
//! ogni installer/pacchetto di aggiornamento — aggiornare il changelog
//! non richiede quindi una nuova release dell'app, solo un push sul
//! branch principale.
//!
//! Formato atteso (vedi docs/CHANGELOG.md): un titolo `## <versione>`
//! per ogni release, con dentro un `### it` e un `### en` — mostriamo
//! solo la sezione della versione e lingua richieste, non l'intero
//! file (richiesta esplicita: non serve tutta la cronologia in
//! Impostazioni, solo le novità della release corrente).

const URL_CHANGELOG: &str = "https://codex-cpp.github.io/TrackFlow/CHANGELOG.md";

fn estrai_sezione<'a>(testo: &'a str, prefisso: &str, nome: &str) -> Option<&'a str> {
    let intestazione = format!("{prefisso}{nome}");
    let inizio = testo.find(&intestazione)? + intestazione.len();
    let resto = &testo[inizio..];
    let fine = resto.find(&format!("\n{prefisso}")).unwrap_or(resto.len());
    Some(resto[..fine].trim())
}

fn scarica_changelog_bloccante(versione: &str, lingua: &str) -> Result<String, String> {
    let risposta = ureq::get(URL_CHANGELOG)
        .call()
        .map_err(|e| format!("download del changelog fallito: {e}"))?;
    let contenuto = risposta
        .into_string()
        .map_err(|e| format!("risposta del changelog non leggibile: {e}"))?;

    let sezione_versione = estrai_sezione(&contenuto, "## ", versione)
        .ok_or_else(|| format!("nessuna voce di changelog per la versione {versione}"))?;

    // "it" resta il fallback se la lingua richiesta non ha una sua
    // sottosezione (es. nuova lingua aggiunta all'app ma non ancora
    // tradotta qui) — meglio mostrare l'italiano che niente.
    estrai_sezione(sezione_versione, "### ", lingua)
        .or_else(|| estrai_sezione(sezione_versione, "### ", "it"))
        .map(str::to_string)
        .ok_or_else(|| format!("nessun changelog leggibile per la versione {versione}"))
}

#[tauri::command]
pub async fn leggi_changelog(versione: String, lingua: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || scarica_changelog_bloccante(&versione, &lingua))
        .await
        .map_err(|e| format!("errore interno: {e}"))?
}

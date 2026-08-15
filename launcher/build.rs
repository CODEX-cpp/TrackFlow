fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = tauri_winres::WindowsResource::new();
        res.set_icon("../src-tauri/icons/icon.ico");
        // Metadati Windows espliciti (Aggiornati, Firma digitale, Proprietà
        // del file) — senza, restavano tutti vuoti (verificato con
        // Get-Item .VersionInfo), un segnale che i classificatori
        // euristici/ML di Windows Defender associano a software sospetto.
        // Da soli non garantiscono di evitare falsi positivi, ma è un
        // passo gratuito e corretto in quella direzione.
        res.set("CompanyName", "TrackFlow");
        res.set("ProductName", "TrackFlow");
        res.set("FileDescription", "TrackFlow launcher");
        res.set("LegalCopyright", "Copyright © 2026 CODEX-cpp");
        res.set("OriginalFilename", "launcher.exe");
        res.set("InternalName", "launcher");
        res.set("Comments", "https://github.com/CODEX-cpp/TrackFlow");
        res.compile().expect("impossibile incorporare l'icona in launcher.exe");
    }
}

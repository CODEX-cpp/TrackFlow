//! Cifra/decifra segreti (per ora solo la chiave API Claude, vedi
//! agent.rs) tramite la DPAPI di Windows (`CryptProtectData`/
//! `CryptUnprotectData`) — lega il segreto all'utente Windows corrente
//! senza dover inventare/gestire noi stessi una password o una chiave
//! di cifratura separata da conservare da qualche parte. Un altro
//! utente Windows sulla stessa macchina, o chiunque copi il file
//! ai-agent-config.json altrove, non riesce a decifrarlo.
//!
//! Richiesta esplicita dell'utente, 2026-08-15: prima la chiave veniva
//! salvata in chiaro (vedi commento storico in agent.rs).

#[cfg(target_os = "windows")]
mod imp {
    use windows::Win32::Foundation::{LocalFree, HLOCAL};
    use windows::Win32::Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    fn blob_da(bytes: &[u8]) -> CRYPT_INTEGER_BLOB {
        CRYPT_INTEGER_BLOB {
            cbData: bytes.len() as u32,
            pbData: bytes.as_ptr() as *mut u8,
        }
    }

    // I byte prodotti da CryptProtectData/CryptUnprotectData vengono
    // allocati dalla funzione stessa (LocalAlloc sotto il cofano) — vanno
    // copiati in un Vec nostro e poi liberati con LocalFree, altrimenti
    // perdita di memoria ad ogni chiamata.
    unsafe fn copia_e_libera(blob: CRYPT_INTEGER_BLOB) -> Vec<u8> {
        let bytes = std::slice::from_raw_parts(blob.pbData, blob.cbData as usize).to_vec();
        let _ = LocalFree(Some(HLOCAL(blob.pbData as *mut core::ffi::c_void)));
        bytes
    }

    pub fn protect(chiaro: &str) -> Option<String> {
        let input = blob_da(chiaro.as_bytes());
        let mut output = CRYPT_INTEGER_BLOB::default();
        unsafe {
            CryptProtectData(&input, None, None, None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut output).ok()?;
            let bytes = copia_e_libera(output);
            Some(base64::Engine::encode(&base64::engine::general_purpose::STANDARD, bytes))
        }
    }

    pub fn unprotect(cifrato_b64: &str) -> Option<String> {
        let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, cifrato_b64).ok()?;
        let input = blob_da(&bytes);
        let mut output = CRYPT_INTEGER_BLOB::default();
        unsafe {
            CryptUnprotectData(&input, None, None, None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut output).ok()?;
            let bytes = copia_e_libera(output);
            String::from_utf8(bytes).ok()
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod imp {
    pub fn protect(_chiaro: &str) -> Option<String> {
        None
    }
    pub fn unprotect(_cifrato_b64: &str) -> Option<String> {
        None
    }
}

/// Cifra una stringa — `None` se DPAPI non è disponibile (piattaforma
/// non Windows) o fallisce per qualunque motivo, nel qual caso il
/// chiamante deve decidere come comportarsi (vedi agent.rs: ripiega sul
/// salvare in chiaro piuttosto che perdere la chiave dell'utente).
pub fn protect(chiaro: &str) -> Option<String> {
    imp::protect(chiaro)
}

/// Decifra una stringa prodotta da `protect()`. `None` se non è un
/// blob DPAPI valido (piattaforma diversa, dati corrotti) — usato anche
/// per riconoscere una chiave salvata PRIMA che questa cifratura
/// esistesse: se `unprotect()` fallisce, il chiamante ripiega sul
/// trattare il valore come già in chiaro (retrocompatibilità).
pub fn unprotect(cifrato_b64: &str) -> Option<String> {
    imp::unprotect(cifrato_b64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_preserves_original_string() {
        let originale = "sk-ant-api03-esempio-di-chiave-non-reale-1234567890";
        let cifrato = protect(originale).expect("protect deve riuscire su Windows");
        assert_ne!(cifrato, originale, "il testo cifrato non deve coincidere col chiaro");
        let decifrato = unprotect(&cifrato).expect("unprotect deve riuscire sullo stesso blob");
        assert_eq!(decifrato, originale);
    }

    #[test]
    fn unprotect_rejects_plaintext_gracefully() {
        // Simula una chiave salvata da una versione precedente, mai
        // cifrata — non deve andare in panic, solo tornare None.
        assert!(unprotect("questo-non-e-un-blob-dpapi").is_none());
    }
}

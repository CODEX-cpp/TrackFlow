use serde::{Deserialize, Serialize};

// Max duration of a i64 nanosecond is 2562047.7880152157 hours
// ((2**64)/2)/1000000000/60/60

fn get_nanos(duration: &chrono::Duration) -> f64 {
    (duration.num_nanoseconds().unwrap() as f64) / 1_000_000_000.0
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "chrono::Duration")]
pub struct DurationSerialization(#[serde(getter = "get_nanos")] f64);

// Provide a conversion to construct the remote type.
impl From<DurationSerialization> for chrono::Duration {
    fn from(def: DurationSerialization) -> chrono::Duration {
        // .round(), non un semplice cast: un valore memorizzato con
        // precisione al nanosecondo, esportato come secondi in virgola
        // mobile (get_nanos sopra) e poi reimportato, non torna sempre
        // esattamente allo stesso intero di nanosecondi — l'aritmetica in
        // f64 può restituire un valore di una frazione sotto quello vero
        // (es. 63.482999999999999 invece di 63.483), e un cast diretto a
        // i64 TRONCA anziché arrotondare, perdendo 1+ nanosecondi ad ogni
        // andata e ritorno. Un evento reimportato con una durata anche
        // solo di un nanosecondo diversa da quella già salvata non viene
        // più riconosciuto come duplicato dalla deduplicazione
        // dell'importazione (vedi event_identity in
        // aw-server/src/endpoints/import.rs), che lo confronta per
        // uguaglianza esatta — con questo bug, importare più volte lo
        // stesso file esportato da questa stessa app continuava ad
        // aggiungere eventi "nuovi" che in realtà erano già presenti
        // (bug segnalato dall'utente: conteggi "aggiunte" mai a zero
        // nemmeno reimportando lo stesso file invariato).
        chrono::Duration::nanoseconds((def.0 * 1_000_000_000.0).round() as i64)
    }
}

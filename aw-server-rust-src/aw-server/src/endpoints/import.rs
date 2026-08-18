use rocket::form::Form;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::serde::Serialize;
use rocket::State;

use std::collections::{BTreeMap, HashSet};

use aw_models::{BucketsExport, Event};

use aw_datastore::{Datastore, DatastoreError};

use crate::endpoints::{HttpErrorJson, ServerState};

/// Conteggio eventi aggiunti/già presenti (deduplicati) durante
/// un'importazione — richiesta esplicita per mostrare all'utente un esito
/// concreto ("aggiunte X attività, Y già presenti") invece del silenzio
/// totale di prima, indipendentemente dal risultato.
#[derive(Serialize)]
pub struct ImportStats {
    added: usize,
    skipped: usize,
}

/// Computes a dedup identity tuple for an event.
///
/// Uses canonical JSON serialization (sorted keys via `BTreeMap`) so that
/// events with identical key-value pairs but different insertion order
/// (e.g., from different clients) are correctly identified as duplicates.
fn event_identity(
    event: &Event,
) -> Result<(chrono::DateTime<chrono::Utc>, i64, String), HttpErrorJson> {
    // Arrotondato al secondo, non usato al nanosecondo esatto: la durata
    // arriva dal file come numero in virgola mobile (secondi, vedi
    // DurationSerialization in aw-models) e un giro di andata/ritorno
    // float non è garantito tornare all'intero di nanosecondi esatto,
    // nemmeno arrotondando alla conversione (vedi il commento lì per il
    // bug reale che questo ha causato). Nessun watcher di questa app
    // traccia/mostra una precisione sotto il secondo, quindi arrotondare
    // qui elimina il rischio del tutto (non solo per il rumore residuo
    // nostro, ma per qualunque client — Android, altri fork — che
    // esporti con una precisione in virgola mobile leggermente diversa).
    let duration_ns = event.duration.num_nanoseconds().ok_or_else(|| {
        HttpErrorJson::new(
            Status::InternalServerError,
            "Failed to encode event duration for dedup".to_string(),
        )
    })?;
    let duration_s = duration_ns / 1_000_000_000;
    // Sort keys before serializing for canonical, order-independent dedup.
    // This prevents missed duplicates when events from different clients
    // serialize the same data with different key orderings.
    let sorted: BTreeMap<_, _> = event.data.iter().collect();
    let data_json = serde_json::to_string(&sorted).map_err(|e| {
        HttpErrorJson::new(
            Status::InternalServerError,
            format!("Failed to encode event data for dedup: {e}"),
        )
    })?;
    // Il secondo campo è in secondi interi, non nanosecondi come il nome
    // del tipo di ritorno potrebbe far pensare — vedi duration_s sopra.
    Ok((event.timestamp, duration_s, data_json))
}

fn import(datastore: &Datastore, import: BucketsExport) -> Result<ImportStats, HttpErrorJson> {
    let mut added: usize = 0;
    let mut skipped: usize = 0;

    // Richiesta esplicita: "Importa bucket" serve solo ad aggiungere dati
    // a bucket che esistono già su questo dispositivo, mai a crearne di
    // nuovi "al buio" da un file — un file che referenzia anche un solo
    // bucket sconosciuto viene rifiutato per intero, prima di toccare
    // qualunque dato (nessuna importazione parziale silenziosa).
    for bucket_id in import.buckets.keys() {
        if datastore.get_bucket(bucket_id).is_err() {
            return Err(HttpErrorJson::new(
                Status::BadRequest,
                format!(
                    "Il bucket '{bucket_id}' non esiste su questo dispositivo — l'importazione può solo aggiungere dati a bucket già esistenti, non crearne di nuovi."
                ),
            ));
        }
    }

    for (_bucketname, mut bucket) in import.buckets {
        match datastore.create_bucket(&bucket) {
            Ok(_) => (),
            Err(DatastoreError::BucketAlreadyExists(_)) => {
                // Bucket already exists — merge events, skipping duplicates
                info!("Bucket '{}' already exists, merging events", bucket.id);
                if let Some(events) = bucket.events.take() {
                    let events_vec = events.take_inner();
                    if !events_vec.is_empty() {
                        let total_in_bucket = events_vec.len();
                        // Determine time range of events to import
                        let start = events_vec.iter().map(|e| e.timestamp).min().unwrap();
                        let end = events_vec
                            .iter()
                            .map(|e| e.calculate_endtime())
                            .max()
                            .unwrap();

                        // Fetch existing events in that range to detect duplicates.
                        // Events without an explicit ID would otherwise be inserted as new rows
                        // via AUTOINCREMENT, silently creating duplicates on re-import.
                        //
                        // **Memory note**: This loads all events in the import time range into
                        // memory for O(1) dedup lookups. Typical Android re-imports involve a
                        // few thousand events (~1-2 MB), which is well within server bounds.
                        // Pathological cases (years of data) could be mitigated with pagination
                        // or a bloom filter if OOM issues arise in practice.
                        let existing = datastore
                            .get_events_unclipped(&bucket.id, Some(start), Some(end), None)
                            .map_err(|e| {
                                HttpErrorJson::new(
                                    Status::InternalServerError,
                                    format!(
                                        "Failed to fetch existing events for dedup in '{}': {e:?}",
                                        bucket.id
                                    ),
                                )
                            })?;

                        let existing_identities: HashSet<_> = existing
                            .iter()
                            .map(event_identity)
                            .collect::<Result<_, _>>()?;

                        // Filter out events already present (matched by timestamp, duration, data)
                        let new_events: Vec<_> = events_vec
                            .into_iter()
                            .map(|event| Ok((event_identity(&event)?, event)))
                            .collect::<Result<Vec<_>, HttpErrorJson>>()?
                            .into_iter()
                            .filter_map(|(identity, mut event)| {
                                if existing_identities.contains(&identity) {
                                    return None;
                                }
                                // L'id nel file è quello ORIGINALE del bucket da cui
                                // è stato esportato — visto che gli id non vengono
                                // mai riusati (AUTOINCREMENT), quell'id appartiene
                                // ancora alla stessa riga su questo stesso
                                // dispositivo. insert_events fa un INSERT OR REPLACE
                                // includendo l'id: senza azzerarlo qui, un evento
                                // "nuovo" secondo il confronto sopra sovrascriveva
                                // silenziosamente qualunque riga avesse ora quello
                                // stesso id — anche se nel frattempo era cambiata
                                // (es. una sessione del cronometro progetto sigillata
                                // dopo l'esportazione) — corrompendo dati correnti
                                // con lo snapshot ormai vecchio del file. Bug reale
                                // segnalato dall'utente: reimportare più volte lo
                                // stesso file continuava ad "aggiungere" un numero di
                                // eventi che oscillava invece di scendere a zero,
                                // proprio perché l'app dal vivo e questa importazione
                                // si riscrivevano a vicenda le stesse righe. Azzerarlo
                                // fa assegnare a SQLite un id nuovo di zecca, un vero
                                // INSERT invece di una sostituzione.
                                event.id = None;
                                Some(event)
                            })
                            .collect();

                        skipped += total_in_bucket - new_events.len();
                        added += new_events.len();

                        if !new_events.is_empty() {
                            if let Err(e) = datastore.insert_events(&bucket.id, &new_events) {
                                let err_msg = format!(
                                    "Failed to merge events into existing bucket '{}': {e:?}",
                                    bucket.id
                                );
                                warn!("{}", err_msg);
                                return Err(HttpErrorJson::new(
                                    Status::InternalServerError,
                                    err_msg,
                                ));
                            }
                        }
                    }
                }
            }
            Err(e) => {
                let err_msg = format!("Failed to import bucket: {e:?}");
                warn!("{}", err_msg);
                return Err(HttpErrorJson::new(Status::InternalServerError, err_msg));
            }
        }
    }
    Ok(ImportStats { added, skipped })
}

#[post("/", data = "<json_data>", format = "application/json")]
pub fn bucket_import_json(
    state: &State<ServerState>,
    json_data: Json<BucketsExport>,
) -> Result<Json<ImportStats>, HttpErrorJson> {
    import(&state.datastore, json_data.into_inner()).map(Json)
}

#[derive(FromForm)]
pub struct ImportForm {
    // FIXME: In the web-ui the name of this field is buckets.json, but "." is not allowed in field
    // names in Rocket and just simply "buckets" seems to work apparently but not sure why.
    // FIXME: In aw-server python it will import all fields rather just the one named
    // "buckets.json", that should probably be done here as well.
    #[field(name = "buckets")]
    import: Json<BucketsExport>,
}

#[post("/", data = "<form>", format = "multipart/form-data")]
pub fn bucket_import_form(
    state: &State<ServerState>,
    form: Form<ImportForm>,
) -> Result<Json<ImportStats>, HttpErrorJson> {
    import(&state.datastore, form.into_inner().import.into_inner()).map(Json)
}

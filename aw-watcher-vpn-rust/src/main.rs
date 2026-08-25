//! Legge i log di due programmi VPN diversi (OpenVPN Connect e ZyWALL
//! SecuExtender) e manda ad ActivityWatch un evento per ogni sessione, con
//! inizio/fine precisi e nome del cliente — nello stesso bucket, con la
//! stessa tabella di corrispondenza indirizzo->cliente, così il pannello
//! "Top Clienti VPN" non deve sapere quale dei due programmi ha generato
//! la sessione. Il nome cliente per indirizzo si ottiene per lo più in
//! automatico leggendo i profili già configurati dall'utente dentro
//! OpenVPN Connect stesso (vedi `load_openvpn_profile_names`) — nessuna
//! lista da mantenere a mano, salvo override espliciti in
//! `client_mapping.txt` (indispensabile invece per ZyWALL, che non ha
//! profili con nome, solo IP).
//!
//! Le due fonti sono trattate come "sorgenti" indipendenti: ognuna tiene
//! il proprio punto di lettura nel log e l'eventuale sessione ancora
//! aperta, separatamente. Porting 1:1 da aw_watcher_vpn/main.py (Python),
//! con l'aggiunta della lettura automatica dei profili OpenVPN Connect
//! (non presente nell'originale Python).

use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration as StdDuration;

use chrono::{DateTime, Local, NaiveDateTime, SecondsFormat, TimeZone, Utc};
use clap::Parser;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

const CLIENT_NAME: &str = "aw-watcher-vpn";
const BUCKET_ID: &str = "vpn-sessions";
const BUCKET_TYPE: &str = "vpn.session";

/// Stampa una riga JSON su stdout invece di mandare una richiesta di
/// rete — il processo Tauri che ci ha lanciato legge questa pipe e
/// inoltra l'evento al server in-process (vedi BLUEPRINT.md, Fase 5,
/// per il contratto completo del formato).
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
    let mut stdout = std::io::stdout();
    let _ = writeln!(stdout, "{envelope}");
    let _ = stdout.flush();
}
// Deve restare un po' più alto dell'intervallo di controllo, altrimenti
// ActivityWatch potrebbe considerare due controlli consecutivi come
// sessioni diverse invece che una che continua.
const HEARTBEAT_MARGIN_SECONDS: i64 = 15;

/// Cartella scrivibile condivisa - default: %LOCALAPPDATA%\TrackFlow\app-data
/// (stessa cartella di icone/screenshot). Non più "accanto all'exe": quel
/// percorso cambia tra dev/release/installazione reale (spesso sotto
/// Program Files, di sola lettura per un utente standard), esattamente lo
/// stesso problema già risolto per aw-watcher-app-icons/screenshot.
fn default_app_data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_default()
        .join("TrackFlow")
        .join("app-data")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenSession {
    inizio: DateTime<Utc>,
    cliente: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SourceState {
    byte_letti: u64,
    sessione_aperta: Option<OpenSession>,
}

type State = HashMap<String, SourceState>;

fn load_state(path: &Path) -> State {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_state(path: &Path, state: &State) {
    if let Ok(json) = serde_json::to_string_pretty(state) {
        let _ = fs::write(path, json);
    }
}

fn stato_iniziale_fonte(percorso_log: &Path) -> SourceState {
    let posizione_iniziale = fs::metadata(percorso_log).map(|m| m.len()).unwrap_or(0);
    SourceState {
        byte_letti: posizione_iniziale,
        sessione_aperta: None,
    }
}

fn load_client_mapping(path: &Path) -> HashMap<String, String> {
    let mut mapping = HashMap::new();
    let Ok(content) = fs::read_to_string(path) else {
        return mapping;
    };
    for riga in content.lines() {
        let riga = riga.trim();
        if riga.is_empty() || riga.starts_with('#') || !riga.contains('=') {
            continue;
        }
        if let Some((indirizzo, nome_cliente)) = riga.split_once('=') {
            mapping.insert(indirizzo.trim().to_string(), nome_cliente.trim().to_string());
        }
    }
    mapping
}

/// Legge il file di configurazione interno di OpenVPN Connect
/// (`config.json`) per ottenere automaticamente nome↔indirizzo di ogni
/// profilo VPN già configurato dall'utente nell'app stessa — quello che si
/// vede nella schermata "My Profiles" di OpenVPN Connect.
///
/// NOTA sul formato: non è documentato pubblicamente, verificato solo
/// leggendo un file reale. È il tipico stato Redux persistito di un'app
/// Electron: il JSON esterno ha una chiave `"persist:root"` il cui valore
/// è ESSO STESSO un testo JSON (non un oggetto annidato), che a sua volta
/// ha una chiave `"status"` anch'essa un testo JSON, che finalmente
/// contiene `"profiles"`: un oggetto con un'entrata per profilo (chiave =
/// id numerico interno, non usato qui), ognuna con `"profileDisplayName"`
/// (il nome scelto dall'utente, es. "Zanin") e `"hostname"` (l'indirizzo
/// del server VPN). Va "sbustato" tre volte per arrivarci.
///
/// Qualunque cosa vada storta (file assente, struttura cambiata in un
/// futuro aggiornamento di OpenVPN Connect — non è un formato stabile
/// pensato per essere letto da fuori) restituisce semplicemente una mappa
/// vuota: non deve mai far fallire l'avvio del watcher, è un arricchimento
/// automatico, non un requisito.
fn load_openvpn_profile_names(config_path: &Path) -> HashMap<String, String> {
    let mut mapping = HashMap::new();

    let Ok(raw) = fs::read_to_string(config_path) else {
        return mapping;
    };
    let Ok(esterno) = serde_json::from_str::<Value>(&raw) else {
        return mapping;
    };
    let Some(persist_root_testo) = esterno.get("persist:root").and_then(Value::as_str) else {
        return mapping;
    };
    let Ok(persist_root) = serde_json::from_str::<Value>(persist_root_testo) else {
        return mapping;
    };
    let Some(status_testo) = persist_root.get("status").and_then(Value::as_str) else {
        return mapping;
    };
    let Ok(status) = serde_json::from_str::<Value>(status_testo) else {
        return mapping;
    };
    let Some(profili) = status.get("profiles").and_then(Value::as_object) else {
        return mapping;
    };

    for profilo in profili.values() {
        let nome = profilo.get("profileDisplayName").and_then(Value::as_str);
        let indirizzo = profilo.get("hostname").and_then(Value::as_str);
        if let (Some(nome), Some(indirizzo)) = (nome, indirizzo) {
            if !nome.is_empty() && !indirizzo.is_empty() {
                mapping.insert(indirizzo.to_string(), nome.to_string());
            }
        }
    }

    mapping
}

/// Unisce i nomi letti automaticamente dai profili OpenVPN Connect con gli
/// eventuali override espliciti in `client_mapping.txt` — questi ultimi
/// vincono sempre (indispensabili per ZyWALL, che non ha profili OpenVPN
/// con nome, e utili per rinominare un profilo dal nome poco chiaro).
fn build_client_mapping(
    da_openvpn: HashMap<String, String>,
    override_manuale: HashMap<String, String>,
) -> HashMap<String, String> {
    let mut mapping = da_openvpn;
    mapping.extend(override_manuale);
    mapping
}

const SEZIONE_AUTO_INIZIO: &str =
    "# === INIZIO SEZIONE AUTOMATICA (profili OpenVPN Connect - aggiornata da sola, non modificare qui sotto) ===";
const SEZIONE_AUTO_FINE: &str = "# === FINE SEZIONE AUTOMATICA ===";

/// Crea (se manca) o aggiorna `client_mapping.txt` con i nomi letti da
/// OpenVPN Connect, senza mai toccare eventuali righe aggiunte a mano
/// dall'utente (es. indirizzi ZyWALL, che non hanno un profilo OpenVPN e
/// vanno per forza scritti manualmente).
///
/// Il file viene diviso in due parti: la sezione "automatica" (delimitata
/// da `SEZIONE_AUTO_INIZIO`/`SEZIONE_AUTO_FINE`, riscritta ogni volta da
/// zero con lo stato corrente dei profili — così un profilo rinominato o
/// eliminato in OpenVPN Connect si riflette qui senza lasciare voci
/// vecchie) e tutto quello che sta dopo `SEZIONE_AUTO_FINE`, preservato
/// carattere per carattere (override manuali). Se il file esiste già ma
/// non ha mai avuto questi marcatori (es. creato a mano prima di questa
/// funzione), il suo intero contenuto viene trattato come "manuale" e
/// mantenuto, con la sezione automatica aggiunta sopra.
///
/// Non scrive nulla se `da_openvpn` è vuota: un `config.json` illeggibile
/// o temporaneamente in un formato inatteso non deve mai svuotare nomi
/// già noti scritti in precedenza.
fn write_openvpn_section(mapping_file: &Path, da_openvpn: &HashMap<String, String>) {
    if da_openvpn.is_empty() {
        return;
    }

    let contenuto_esistente = fs::read_to_string(mapping_file).unwrap_or_default();
    let parte_manuale = match contenuto_esistente.find(SEZIONE_AUTO_FINE) {
        Some(pos) => contenuto_esistente[pos + SEZIONE_AUTO_FINE.len()..]
            .trim_start_matches(['\r', '\n'])
            .to_string(),
        None => contenuto_esistente,
    };

    let mut indirizzi: Vec<&String> = da_openvpn.keys().collect();
    indirizzi.sort(); // ordine stabile: file leggibile e diff sensati tra un giro e l'altro

    let mut nuovo_contenuto = String::new();
    nuovo_contenuto.push_str(SEZIONE_AUTO_INIZIO);
    nuovo_contenuto.push('\n');
    for indirizzo in indirizzi {
        nuovo_contenuto.push_str(&format!("{} = {}\n", indirizzo, da_openvpn[indirizzo]));
    }
    nuovo_contenuto.push_str(SEZIONE_AUTO_FINE);
    nuovo_contenuto.push('\n');
    if !parte_manuale.trim().is_empty() {
        nuovo_contenuto.push('\n');
        nuovo_contenuto.push_str(parte_manuale.trim_end());
        nuovo_contenuto.push('\n');
    }

    let _ = fs::write(mapping_file, nuovo_contenuto);
}

fn nome_cliente_da_indirizzo(indirizzo: &str, mapping: &HashMap<String, String>) -> String {
    mapping
        .get(indirizzo)
        .cloned()
        .unwrap_or_else(|| indirizzo.to_string())
}

#[derive(Debug)]
enum ParsedEvent {
    Connected { quando: DateTime<Utc>, indirizzo: String },
    Disconnected { quando: DateTime<Utc> },
}

/// Interpreta un NaiveDateTime come ora locale del PC (i log scrivono
/// l'ora locale, non UTC) e lo converte in UTC, come Python's
/// `datetime.astimezone(timezone.utc)` su un datetime naive (che in
/// Python assume la zona locale del sistema).
fn local_naive_to_utc(naive: NaiveDateTime) -> Option<DateTime<Utc>> {
    Local
        .from_local_datetime(&naive)
        .single()
        .map(|dt| dt.with_timezone(&Utc))
}

// Formato riga: "[Aug 6, 2026, 10:36:09] EVENT: CONNECTED
// nomeutente@indirizzo:porta" oppure "[...] EVENT: DISCONNECTED"
fn estrai_eventi_openvpn(testo: &str) -> Vec<ParsedEvent> {
    // %e = giorno del mese, non zero-paddato (accetta "6" e " 6"), a
    // differenza di %d che in chrono richiede sempre 2 cifre.
    let pattern = Regex::new(
        r"\[([A-Za-z]{3} \d{1,2}, \d{4}, \d{2}:\d{2}:\d{2})\]\s*EVENT: (CONNECTED \S+@([^:]+):\d+|DISCONNECTED)",
    )
    .unwrap();

    let mut eventi = Vec::new();
    for cap in pattern.captures_iter(testo) {
        let quando_str = &cap[1];
        let Ok(naive) = NaiveDateTime::parse_from_str(quando_str, "%b %e, %Y, %H:%M:%S") else {
            continue;
        };
        let Some(quando) = local_naive_to_utc(naive) else {
            continue;
        };

        if cap[2].starts_with("CONNECTED") {
            eventi.push(ParsedEvent::Connected {
                quando,
                indirizzo: cap[3].to_string(),
            });
        } else {
            eventi.push(ParsedEvent::Disconnected { quando });
        }
    }
    eventi
}

// Formato riga: "[ 2026/08/06 18:07:29 ][SecuExtender Helper] ..."
// CONNECTED: riga "Succeed to prioritize route to <IP>, ..."
//            (non "Succeed to DELETE prioritize route", quella è la
//            disconnessione — l'ordine delle parole è diverso, quindi
//            il pattern qui sotto non la confonde con quella).
// DISCONNECTED: riga separatrice fatta di soli simboli "="
fn estrai_eventi_zywall(testo: &str) -> Vec<ParsedEvent> {
    let pattern = Regex::new(
        r"\[\s*(\d{4}/\d{2}/\d{2} \d{2}:\d{2}:\d{2})\s*\]\[SecuExtender Helper\]\s*(Succeed to prioritize route to ([\d.]+)|=====+)",
    )
    .unwrap();

    let mut eventi = Vec::new();
    for cap in pattern.captures_iter(testo) {
        let quando_str = &cap[1];
        let Ok(naive) = NaiveDateTime::parse_from_str(quando_str, "%Y/%m/%d %H:%M:%S") else {
            continue;
        };
        let Some(quando) = local_naive_to_utc(naive) else {
            continue;
        };

        if cap[2].starts_with("Succeed") {
            eventi.push(ParsedEvent::Connected {
                quando,
                indirizzo: cap[3].to_string(),
            });
        } else {
            eventi.push(ParsedEvent::Disconnected { quando });
        }
    }
    eventi
}

struct Fonte {
    chiave: &'static str,
    percorso_log: PathBuf,
    estrattore: fn(&str) -> Vec<ParsedEvent>,
}

/// Controlla se un processo con questo nome (case-insensitive) è
/// attualmente in esecuzione — stessa API ToolHelp32 già usata in
/// src-tauri/src/lib.rs (`invia_processi_in_esecuzione_a_icone`) per
/// elencare i processi.
///
/// Richiesta esplicita dell'utente: chiude spesso OpenVPN Connect
/// terminando l'intero programma invece di disconnettersi da dentro —
/// in quel caso il log non riceve mai la riga "EVENT: DISCONNECTED"
/// (quella la scrive l'app solo per una disconnessione "pulita" fatta
/// dalla sua stessa UI), quindi `sessione_aperta` restava valorizzata
/// per sempre nello stato: gli heartbeat periodici continuavano ad
/// allungare la sessione all'infinito anche ore dopo la disconnessione
/// vera, finché non arrivava una nuova connessione (che sovrascriveva lo
/// stato senza nemmeno chiudere quella vecchia). Vedi
/// `VpnWatcher::chiudi_se_processo_terminato`.
#[cfg(windows)]
fn processo_in_esecuzione(nome_processo_lower: &str) -> bool {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
    };

    unsafe {
        let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
            // Non possiamo verificare — meglio non chiudere una sessione
            // per sbaglio che potrebbe essere ancora attiva.
            return true;
        };
        let mut voce: PROCESSENTRY32W = std::mem::zeroed();
        voce.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut trovato = false;
        if Process32FirstW(snapshot, &mut voce).is_ok() {
            loop {
                let fine = voce.szExeFile.iter().position(|&c| c == 0).unwrap_or(voce.szExeFile.len());
                let nome = String::from_utf16_lossy(&voce.szExeFile[..fine]).to_lowercase();
                if nome == nome_processo_lower {
                    trovato = true;
                    break;
                }
                if Process32NextW(snapshot, &mut voce).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snapshot);
        trovato
    }
}

#[cfg(not(windows))]
fn processo_in_esecuzione(_nome_processo_lower: &str) -> bool {
    true
}

struct VpnWatcher {
    heartbeat_pulsetime: f64,
    fonti: Vec<Fonte>,
    mapping: HashMap<String, String>,
    stato: State,
    state_file: PathBuf,
    mapping_file: PathBuf,
    openvpn_config_path: PathBuf,
    ultimo_da_openvpn: HashMap<String, String>,
}

impl VpnWatcher {
    fn new(
        poll_interval_seconds: i64,
        fonti: Vec<Fonte>,
        mapping: HashMap<String, String>,
        state_file: PathBuf,
        mapping_file: PathBuf,
        openvpn_config_path: PathBuf,
        da_openvpn_iniziale: HashMap<String, String>,
    ) -> Self {
        let mut stato = load_state(&state_file);
        for fonte in &fonti {
            stato
                .entry(fonte.chiave.to_string())
                .or_insert_with(|| stato_iniziale_fonte(&fonte.percorso_log));
        }

        VpnWatcher {
            heartbeat_pulsetime: (poll_interval_seconds + HEARTBEAT_MARGIN_SECONDS) as f64,
            fonti,
            mapping,
            stato,
            state_file,
            mapping_file,
            openvpn_config_path,
            ultimo_da_openvpn: da_openvpn_iniziale,
        }
    }

    /// Ricontrolla ad OGNI giro (non più ogni 30 minuti — bug reale
    /// segnalato dall'utente, vedi sotto) sia `config.json` di OpenVPN
    /// Connect sia la parte MANUALE di `client_mapping.txt`
    /// (Impostazioni → Integrazioni → VPN, o scritta a mano per ZyWALL,
    /// che non ha profili da cui auto-rilevarsi). Entrambi sono letture
    /// di file piccoli (JSON/testo), il costo di rileggerli ad ogni poll
    /// (default 15s) è trascurabile.
    ///
    /// Prima, la parte manuale veniva ricaricata SOLO quando cambiava
    /// ANCHE il profilo OpenVPN nello stesso giro (il vecchio early
    /// return qui sotto usciva prima di toccare `override_manuale` se
    /// `da_openvpn` non era cambiato) — un salvataggio da Impostazioni
    /// (nuovo cliente ZyWALL, o correzione di un nome) restava quindi
    /// invisibile al watcher già in esecuzione anche per decine di
    /// minuti, o del tutto finché non veniva riavviato: causa reale
    /// delle notifiche "cliente VPN non associato" in ritardo o su
    /// client già registrati, e del nome che non cambiava mai per le
    /// connessioni successive a una registrazione ZyWALL appena fatta.
    fn sincronizza_mapping_se_serve(&mut self) {
        let da_openvpn = load_openvpn_profile_names(&self.openvpn_config_path);
        if !da_openvpn.is_empty() && da_openvpn != self.ultimo_da_openvpn {
            write_openvpn_section(&self.mapping_file, &da_openvpn);
            self.ultimo_da_openvpn = da_openvpn;
        }

        let override_manuale = load_client_mapping(&self.mapping_file);
        self.mapping = build_client_mapping(self.ultimo_da_openvpn.clone(), override_manuale);
    }

    fn controlla_tutte_le_fonti(&mut self) {
        for i in 0..self.fonti.len() {
            self.controlla_fonte(i);
        }
    }

    fn controlla_fonte(&mut self, idx: usize) {
        let chiave = self.fonti[idx].chiave.to_string();
        let percorso_log = self.fonti[idx].percorso_log.clone();
        let estrattore = self.fonti[idx].estrattore;

        if !percorso_log.exists() {
            return;
        }

        let dimensione_attuale = fs::metadata(&percorso_log).map(|m| m.len()).unwrap_or(0);
        let byte_gia_letti = self.stato.get(&chiave).map(|s| s.byte_letti).unwrap_or(0);

        // File troncato/sostituito: ripartiamo da capo.
        let byte_gia_letti = if dimensione_attuale < byte_gia_letti {
            0
        } else {
            byte_gia_letti
        };

        if dimensione_attuale == byte_gia_letti {
            return;
        }

        let Ok(mut file) = fs::File::open(&percorso_log) else {
            return;
        };
        use std::io::{Seek, SeekFrom};
        if file.seek(SeekFrom::Start(byte_gia_letti)).is_err() {
            return;
        }
        let mut testo_nuovo = String::new();
        // Lossy read, come Python's errors="ignore": se ci sono byte non
        // UTF-8 validi, li scartiamo invece di fallire l'intero giro.
        let mut buf = Vec::new();
        if file.read_to_end(&mut buf).is_err() {
            return;
        }
        testo_nuovo.push_str(&String::from_utf8_lossy(&buf));
        let nuova_posizione = byte_gia_letti + buf.len() as u64;

        for evento in estrattore(&testo_nuovo) {
            match evento {
                ParsedEvent::Connected { quando, indirizzo } => {
                    self.gestisci_connected(&chiave, quando, &indirizzo);
                }
                ParsedEvent::Disconnected { quando } => {
                    self.gestisci_disconnected(&chiave, quando);
                }
            }
        }

        if let Some(s) = self.stato.get_mut(&chiave) {
            s.byte_letti = nuova_posizione;
        }
        save_state(&self.state_file, &self.stato);
    }

    fn gestisci_connected(&mut self, chiave: &str, quando: DateTime<Utc>, indirizzo: &str) {
        let nome_cliente = nome_cliente_da_indirizzo(indirizzo, &self.mapping);
        if let Some(s) = self.stato.get_mut(chiave) {
            s.sessione_aperta = Some(OpenSession {
                inizio: quando,
                cliente: nome_cliente.clone(),
            });
        }
        self.manda_heartbeat(quando, &nome_cliente);
    }

    fn gestisci_disconnected(&mut self, chiave: &str, quando: DateTime<Utc>) {
        let sessione = match self.stato.get(chiave).and_then(|s| s.sessione_aperta.clone()) {
            Some(s) => s,
            None => return,
        };

        let durata = quando - sessione.inizio;
        if durata.num_milliseconds() > 0 {
            let mut data = Map::new();
            data.insert("cliente".to_string(), sessione.cliente.clone().into());
            let durata_secondi = durata.num_milliseconds() as f64 / 1000.0;
            emit("event", None, sessione.inizio, durata_secondi, data);
        }

        if let Some(s) = self.stato.get_mut(chiave) {
            s.sessione_aperta = None;
        }
    }

    /// Chiude subito la sessione OpenVPN se risulta ancora aperta nello
    /// stato ma il processo di OpenVPN Connect non è più in esecuzione —
    /// vedi il commento su `processo_in_esecuzione` per il motivo. Solo
    /// la fonte "openvpn": ZyWALL scrive sempre una riga di chiusura
    /// affidabile anche quando il servizio si ferma, non ha questo
    /// problema (segnalato esplicitamente dall'utente come "perfetto,
    /// non mi lamento").
    fn chiudi_se_processo_terminato(&mut self) {
        let aperta = self
            .stato
            .get("openvpn")
            .and_then(|s| s.sessione_aperta.as_ref())
            .is_some();
        if !aperta {
            return;
        }
        if processo_in_esecuzione("openvpnconnect.exe") {
            return;
        }
        self.gestisci_disconnected("openvpn", Utc::now());
        save_state(&self.state_file, &self.stato);
    }

    fn manda_heartbeat(&self, quando: DateTime<Utc>, nome_cliente: &str) {
        let mut data = Map::new();
        data.insert("cliente".to_string(), nome_cliente.into());
        emit("heartbeat", Some(self.heartbeat_pulsetime), quando, 0.0, data);
    }

    /// Per ogni fonte con una sessione ancora aperta, manda un heartbeat
    /// aggiornato così cresce in dashboard mentre resti connesso —
    /// indipendentemente dalle altre fonti.
    fn tieni_vive_le_sessioni_aperte(&self) {
        let adesso = Utc::now();
        for fonte in &self.fonti {
            if let Some(sessione) = self
                .stato
                .get(fonte.chiave)
                .and_then(|s| s.sessione_aperta.clone())
            {
                self.manda_heartbeat(adesso, &sessione.cliente);
            }
        }
    }
}

#[derive(Parser)]
#[command(about = "Watcher per le sessioni VPN (OpenVPN Connect e ZyWALL SecuExtender)")]
struct Args {
    /// Gira in modalità test: porta 5666, dati separati da quelli reali
    #[arg(long)]
    testing: bool,

    /// Ogni quanti secondi controllare i log (default: 15)
    #[arg(long, default_value_t = 15)]
    poll_interval: i64,

    /// Cartella scrivibile condivisa dove leggere `client_mapping.txt` e
    /// salvare lo stato - default: %LOCALAPPDATA%\TrackFlow\app-data
    /// (stessa cartella di icone/screenshot, passata da Tauri).
    #[arg(long)]
    app_data_dir: Option<PathBuf>,
}

fn main() {
    let args = Args::parse();

    let data_dir = args.app_data_dir.clone().unwrap_or_else(default_app_data_dir);
    let _ = fs::create_dir_all(&data_dir);
    let state_file = data_dir.join("vpn_watcher_state.json");
    let mapping_file = data_dir.join("client_mapping.txt");

    // Override opzionali (debug/test): se non impostati, usano i percorsi
    // reali di default, invariati.
    let openvpn_log_path = std::env::var("AW_WATCHER_VPN_OPENVPN_LOG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_default()
                .join("AppData")
                .join("Roaming")
                .join("OpenVPN Connect")
                .join("log")
                .join("ovpn.log")
        });
    let zywall_log_path = std::env::var("AW_WATCHER_VPN_ZYWALL_LOG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("C:/SecuExtenderHelper.log"));
    let openvpn_config_path = std::env::var("AW_WATCHER_VPN_OPENVPN_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_default()
                .join("AppData")
                .join("Roaming")
                .join("OpenVPN Connect")
                .join("config.json")
        });

    let fonti = vec![
        Fonte {
            chiave: "openvpn",
            percorso_log: openvpn_log_path,
            estrattore: estrai_eventi_openvpn,
        },
        Fonte {
            chiave: "zywall",
            percorso_log: zywall_log_path,
            estrattore: estrai_eventi_zywall,
        },
    ];

    let fonti_trovate: Vec<&Fonte> = fonti.iter().filter(|f| f.percorso_log.exists()).collect();
    if fonti_trovate.is_empty() {
        println!("ATTENZIONE: non trovo nessuno dei due log VPN attesi:");
        for f in &fonti {
            println!("  - {}: {}", f.chiave, f.percorso_log.display());
        }
        return;
    }

    let da_openvpn = load_openvpn_profile_names(&openvpn_config_path);
    // Crea client_mapping.txt se non esiste ancora, o aggiorna la sua
    // sezione automatica se i profili sono cambiati dall'ultima esecuzione
    // (rinomine, profili nuovi) — prima di leggerlo, così l'eventuale
    // override manuale già presente nel file resta comunque rispettato.
    write_openvpn_section(&mapping_file, &da_openvpn);
    let override_manuale = load_client_mapping(&mapping_file);
    let n_da_openvpn = da_openvpn.len();
    let n_override = override_manuale.len();
    let mapping = build_client_mapping(da_openvpn.clone(), override_manuale);

    println!(
        "Modalità: {}",
        if args.testing { "testing (porta 5666)" } else { "normale (porta 5600)" }
    );
    println!(
        "Mappatura clienti: {n_da_openvpn} nomi letti automaticamente dai profili OpenVPN Connect, {n_override} override manuali da client_mapping.txt ({} indirizzi noti in totale)",
        mapping.len()
    );
    for f in &fonti {
        let stato_str = if f.percorso_log.exists() { "trovato" } else { "NON trovato, ignorato" };
        println!("Fonte '{}': {} ({})", f.chiave, f.percorso_log.display(), stato_str);
    }

    let mut watcher = VpnWatcher::new(
        args.poll_interval,
        fonti,
        mapping,
        state_file,
        mapping_file,
        openvpn_config_path,
        da_openvpn,
    );

    loop {
        thread::sleep(StdDuration::from_secs(args.poll_interval as u64));
        watcher.controlla_tutte_le_fonti();
        watcher.chiudi_se_processo_terminato();
        watcher.tieni_vive_le_sessioni_aperte();
        watcher.sincronizza_mapping_se_serve();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(windows)]
    fn processo_in_esecuzione_trova_se_stesso() {
        // Il processo di test stesso è sicuramente in esecuzione — verifica
        // che il confronto case-insensitive con ToolHelp32 funzioni
        // davvero, non solo che compili.
        let nome_proprio = std::env::current_exe()
            .ok()
            .and_then(|p| p.file_name().map(|f| f.to_string_lossy().to_lowercase()))
            .expect("nome dell'eseguibile di test");
        assert!(processo_in_esecuzione(&nome_proprio));
    }

    #[test]
    #[cfg(windows)]
    fn processo_in_esecuzione_nome_inventato_non_trovato() {
        assert!(!processo_in_esecuzione("nome-inventato-che-non-esiste-mai.exe"));
    }

    #[test]
    fn load_openvpn_profile_names_reads_real_config_format() {
        // Struttura verificata su un file config.json reale di OpenVPN
        // Connect (nomi/indirizzi qui sotto inventati, non quelli veri):
        // "persist:root" è una stringa JSON, contenente a sua volta
        // "status" come stringa JSON, contenente finalmente l'oggetto
        // "profiles" vero e proprio - tre livelli da sbustare.
        let dir = std::env::temp_dir().join(format!("aw-vpn-test-cfg-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");

        let status_interno = serde_json::json!({
            "profiles": {
                "1111111111111": {
                    "profileDisplayName": "Cliente Alfa",
                    "hostname": "203.0.113.10"
                },
                "2222222222222": {
                    "profileDisplayName": "Cliente Beta",
                    "hostname": "203.0.113.20"
                },
                "3333333333333": {
                    "profileDisplayName": "",
                    "hostname": "203.0.113.30"
                }
            }
        });
        let persist_root_interno = serde_json::json!({ "status": status_interno.to_string() });
        let esterno = serde_json::json!({ "persist:root": persist_root_interno.to_string() });
        std::fs::write(&path, esterno.to_string()).unwrap();

        let mapping = load_openvpn_profile_names(&path);
        assert_eq!(mapping.get("203.0.113.10").unwrap(), "Cliente Alfa");
        assert_eq!(mapping.get("203.0.113.20").unwrap(), "Cliente Beta");
        // Nome vuoto (profilo senza etichetta scelta): non inserito, non
        // vogliamo etichette vuote al posto dell'indirizzo.
        assert!(!mapping.contains_key("203.0.113.30"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_openvpn_profile_names_missing_file_returns_empty() {
        let mapping = load_openvpn_profile_names(Path::new("C:/percorso/che/non/esiste-mai.json"));
        assert!(mapping.is_empty());
    }

    #[test]
    fn load_openvpn_profile_names_unexpected_format_returns_empty_not_panic() {
        let dir = std::env::temp_dir().join(format!("aw-vpn-test-cfg2-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        // Formato inatteso (es. dopo un aggiornamento di OpenVPN Connect):
        // deve tornare una mappa vuota, mai andare in panic.
        std::fs::write(&path, r#"{"qualcosa":"di diverso"}"#).unwrap();

        let mapping = load_openvpn_profile_names(&path);
        assert!(mapping.is_empty());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn build_client_mapping_override_wins_over_openvpn_name() {
        let mut da_openvpn = HashMap::new();
        da_openvpn.insert("203.0.113.10".to_string(), "Cliente Alfa".to_string());
        da_openvpn.insert("203.0.113.20".to_string(), "Cliente Beta".to_string());

        let mut override_manuale = HashMap::new();
        override_manuale.insert("203.0.113.10".to_string(), "Cliente Alfa (rinominato)".to_string());
        // Un indirizzo tipo ZyWALL, mai presente nei profili OpenVPN.
        override_manuale.insert("198.51.100.7".to_string(), "Cliente ZyWALL".to_string());

        let mapping = build_client_mapping(da_openvpn, override_manuale);

        assert_eq!(mapping.get("203.0.113.10").unwrap(), "Cliente Alfa (rinominato)");
        assert_eq!(mapping.get("203.0.113.20").unwrap(), "Cliente Beta");
        assert_eq!(mapping.get("198.51.100.7").unwrap(), "Cliente ZyWALL");
    }

    #[test]
    fn write_openvpn_section_creates_file_when_missing() {
        let dir = std::env::temp_dir().join(format!("aw-vpn-test-write1-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("client_mapping.txt");
        assert!(!path.exists());

        let mut da_openvpn = HashMap::new();
        da_openvpn.insert("203.0.113.10".to_string(), "Cliente Alfa".to_string());
        write_openvpn_section(&path, &da_openvpn);

        let mapping = load_client_mapping(&path);
        assert_eq!(mapping.get("203.0.113.10").unwrap(), "Cliente Alfa");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn write_openvpn_section_preserves_manual_lines_below_marker() {
        let dir = std::env::temp_dir().join(format!("aw-vpn-test-write2-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("client_mapping.txt");

        // Prima scrittura, come farebbe l'avvio del watcher.
        let mut da_openvpn = HashMap::new();
        da_openvpn.insert("203.0.113.10".to_string(), "Cliente Alfa".to_string());
        write_openvpn_section(&path, &da_openvpn);

        // L'utente aggiunge a mano una riga ZyWALL sotto la sezione auto.
        let mut contenuto = std::fs::read_to_string(&path).unwrap();
        contenuto.push_str("\n198.51.100.7 = Cliente ZyWALL\n");
        std::fs::write(&path, &contenuto).unwrap();

        // Riscrittura successiva (es. dopo 30 min) con un profilo rinominato.
        let mut da_openvpn_2 = HashMap::new();
        da_openvpn_2.insert("203.0.113.10".to_string(), "Cliente Alfa Rinominato".to_string());
        write_openvpn_section(&path, &da_openvpn_2);

        let mapping = load_client_mapping(&path);
        // Rinomina applicata automaticamente.
        assert_eq!(mapping.get("203.0.113.10").unwrap(), "Cliente Alfa Rinominato");
        // Riga manuale mai toccata.
        assert_eq!(mapping.get("198.51.100.7").unwrap(), "Cliente ZyWALL");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn write_openvpn_section_treats_pre_existing_manual_file_as_manual() {
        let dir = std::env::temp_dir().join(format!("aw-vpn-test-write3-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("client_mapping.txt");
        // File creato a mano prima di questa funzione, senza marcatori.
        std::fs::write(&path, "198.51.100.7 = Cliente ZyWALL storico\n").unwrap();

        let mut da_openvpn = HashMap::new();
        da_openvpn.insert("203.0.113.10".to_string(), "Cliente Alfa".to_string());
        write_openvpn_section(&path, &da_openvpn);

        let mapping = load_client_mapping(&path);
        assert_eq!(mapping.get("203.0.113.10").unwrap(), "Cliente Alfa");
        // La riga storica, scritta a mano prima ancora che esistessero i
        // marcatori, non deve andare persa.
        assert_eq!(mapping.get("198.51.100.7").unwrap(), "Cliente ZyWALL storico");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn write_openvpn_section_does_nothing_when_openvpn_map_empty() {
        let dir = std::env::temp_dir().join(format!("aw-vpn-test-write4-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("client_mapping.txt");
        std::fs::write(&path, "203.0.113.10 = Cliente Alfa\n").unwrap();

        // config.json illeggibile/vuoto in questo giro: non deve svuotare
        // il file già esistente.
        write_openvpn_section(&path, &HashMap::new());

        let mapping = load_client_mapping(&path);
        assert_eq!(mapping.get("203.0.113.10").unwrap(), "Cliente Alfa");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn openvpn_parses_connect_and_disconnect() {
        let log = "[Aug 6, 2026, 10:36:09] EVENT: CONNECTED nomeutente@203.0.113.5:1194\n\
                    [Aug 6, 2026, 11:15:42] EVENT: DISCONNECTED\n";
        let eventi = estrai_eventi_openvpn(log);
        assert_eq!(eventi.len(), 2);

        match &eventi[0] {
            ParsedEvent::Connected { quando, indirizzo } => {
                assert_eq!(indirizzo, "203.0.113.5");
                // Ora locale 10:36:09 -> confrontiamo riconvertendo in
                // locale e verificando che torni identica (indipendente
                // dal fuso orario della macchina che esegue il test).
                let back = quando.with_timezone(&Local);
                assert_eq!(back.format("%H:%M:%S").to_string(), "10:36:09");
                assert_eq!(back.format("%Y-%m-%d").to_string(), "2026-08-06");
            }
            other => panic!("expected Connected, got {other:?}"),
        }

        match &eventi[1] {
            ParsedEvent::Disconnected { quando } => {
                let back = quando.with_timezone(&Local);
                assert_eq!(back.format("%H:%M:%S").to_string(), "11:15:42");
            }
            other => panic!("expected Disconnected, got {other:?}"),
        }

        // Durata attesa: 11:15:42 - 10:36:09 = 39 minuti 33 secondi.
        let durata = match (&eventi[0], &eventi[1]) {
            (ParsedEvent::Connected { quando: c, .. }, ParsedEvent::Disconnected { quando: d }) => {
                *d - *c
            }
            _ => unreachable!(),
        };
        assert_eq!(durata.num_seconds(), 39 * 60 + 33);
    }

    #[test]
    fn openvpn_single_digit_day_parses() {
        // "Aug 6" (giorno singola cifra, senza zero-padding) deve
        // combaciare con %e — con %d chrono fallirebbe qui.
        let log = "[Aug 6, 2026, 09:00:00] EVENT: CONNECTED Foo@10.0.0.1:1194\n";
        let eventi = estrai_eventi_openvpn(log);
        assert_eq!(eventi.len(), 1);
    }

    #[test]
    fn openvpn_real_log_line_with_trailing_details_parses() {
        // Riga con la stessa forma di una riga reale osservata su un PC
        // di lavoro (indirizzo/nome anonimizzati qui): dopo
        // l'indirizzo:porta il log continua con altri dettagli sulla
        // stessa riga ("(ip) via /TCP on TUN_WIN/..."). Il pattern non
        // deve richiedere che la riga finisca subito dopo la porta.
        let log = "[Aug 6, 2026, 18:23:05] EVENT: CONNECTED nomeutente@203.0.113.99:8080 (203.0.113.99) via /TCP on TUN_WIN/192.168.150.3/ gw=[192.168.150.1/] mtu=1500\n";
        let eventi = estrai_eventi_openvpn(log);
        assert_eq!(eventi.len(), 1);
        match &eventi[0] {
            ParsedEvent::Connected { indirizzo, .. } => assert_eq!(indirizzo, "203.0.113.99"),
            other => panic!("expected Connected, got {other:?}"),
        }
    }

    #[test]
    fn zywall_parses_connect_and_disconnect() {
        let log = "[ 2026/08/06 18:07:29 ][SecuExtender Helper] Succeed to prioritize route to 198.51.100.7, metric 1\n\
                    [ 2026/08/06 18:42:10 ][SecuExtender Helper] ====================================\n";
        let eventi = estrai_eventi_zywall(log);
        assert_eq!(eventi.len(), 2);

        match &eventi[0] {
            ParsedEvent::Connected { indirizzo, .. } => assert_eq!(indirizzo, "198.51.100.7"),
            other => panic!("expected Connected, got {other:?}"),
        }
        assert!(matches!(eventi[1], ParsedEvent::Disconnected { .. }));
    }

    #[test]
    fn zywall_delete_route_is_not_mistaken_for_connect() {
        // "Succeed to DELETE prioritize route" non deve combaciare col
        // pattern CONNECTED (che richiede "Succeed to prioritize route to").
        let log = "[ 2026/08/06 19:00:00 ][SecuExtender Helper] Succeed to DELETE prioritize route to 198.51.100.7\n";
        let eventi = estrai_eventi_zywall(log);
        assert_eq!(eventi.len(), 0, "non deve produrre eventi: {eventi:?}");
    }

    #[test]
    fn client_mapping_parses_and_falls_back_to_address() {
        let dir = std::env::temp_dir().join(format!("aw-vpn-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("client_mapping.txt");
        std::fs::write(
            &path,
            "# commento\n203.0.113.5=Cliente Rossi\n\nnon una riga valida\n198.51.100.7 = Cliente Bianchi \n",
        )
        .unwrap();

        let mapping = load_client_mapping(&path);
        assert_eq!(mapping.get("203.0.113.5").unwrap(), "Cliente Rossi");
        assert_eq!(mapping.get("198.51.100.7").unwrap(), "Cliente Bianchi");
        assert_eq!(nome_cliente_da_indirizzo("203.0.113.5", &mapping), "Cliente Rossi");
        // Indirizzo sconosciuto: torna l'indirizzo stesso invariato.
        assert_eq!(nome_cliente_da_indirizzo("10.0.0.99", &mapping), "10.0.0.99");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn state_roundtrips_through_json() {
        let dir = std::env::temp_dir().join(format!("aw-vpn-test-state-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.json");

        let mut state: State = HashMap::new();
        state.insert(
            "openvpn".to_string(),
            SourceState {
                byte_letti: 1234,
                sessione_aperta: Some(OpenSession {
                    inizio: Utc::now(),
                    cliente: "Cliente Test".to_string(),
                }),
            },
        );
        save_state(&path, &state);

        let loaded = load_state(&path);
        assert_eq!(loaded.get("openvpn").unwrap().byte_letti, 1234);
        assert_eq!(
            loaded.get("openvpn").unwrap().sessione_aperta.as_ref().unwrap().cliente,
            "Cliente Test"
        );

        std::fs::remove_dir_all(&dir).ok();
    }
}

# Changelog

Ogni versione ha una sezione `### it` e una `### en` — l'app mostra solo
quella della lingua attiva, per la versione in esecuzione (vedi
`src-tauri/src/about.rs`, che scarica questo file da GitHub Pages).
Non toccare questo formato senza aggiornare anche quel parser.

## 0.1.16

### it
- Corretto un bug per cui import ed export dipendevano dal nome del PC: un file esportato da un computer non veniva mai riconosciuto importandolo su un altro, anche se il watcher corrispondente esisteva eccome. Ora l'abbinamento non guarda più il nome macchina, e i file esportati non lo rivelano più.
- Corretto un bug più raro nello stesso import, che poteva sovrascrivere un evento cambiato nel frattempo con una versione più vecchia da un reimport.
- Impostazioni → Integrazioni → VPN: gli indirizzi rilevati automaticamente e quelli aggiunti a mano sono ora in un'unica tabella modificabile, ordinata per nome cliente — prima una voce aggiunta a mano non compariva mai insieme a quelle automatiche.
- Watcher Excel: ora traccia un file finché resta aperto, non solo mentre la sua finestra è a fuoco — passare ad un'altra finestra non interrompe più la registrazione. Intervallo di controllo aumentato da 2 a 20 secondi.
- Watcher VPN: la disconnessione viene ora rilevata anche chiudendo del tutto OpenVPN Connect dalla system tray, non solo disconnettendosi dalla sua interfaccia — prima la sessione restava segnata come "in corso" per sempre in quel caso.
- Corretto un raro rischio di corruzione del database alla chiusura dell'app.

### en
- Fixed a bug where import and export depended on the machine's name: a file exported from one computer was never recognized when imported on another, even when the matching watcher existed. Matching no longer looks at the machine name, and exported files no longer reveal it.
- Fixed a rarer bug in the same import feature that could overwrite an event changed in the meantime with an older version from a re-import.
- Settings → Integrations → VPN: auto-detected and manually added addresses now live in a single editable table, sorted by client name — previously a manually added entry never showed up alongside the automatic ones.
- Excel watcher: now tracks a file for as long as it stays open, not only while its window is focused — switching to another window no longer interrupts tracking. Check interval increased from 2 to 20 seconds.
- VPN watcher: disconnection is now detected even when fully closing OpenVPN Connect from the system tray, not only when disconnecting from within its interface — previously the session stayed marked as "ongoing" forever in that case.
- Fixed a rare database corruption risk on app shutdown.

## 0.1.15

### it
- Corretto un bug per cui "Importa bucket" (pagina Watcher) non funzionava mai davvero trascinando o scegliendo un file reale — l'importazione falliva in silenzio, senza nessun messaggio.
- Nuovo: trascinando un file JSON/CSV sopra la finestra, la pagina si oscura ed evidenzia con un bordo dorato dove rilasciarlo; a importazione completata viene mostrato quante attività sono state aggiunte e quante erano già presenti.
- Corretto un bug per cui reimportare più volte lo stesso file continuava ad aggiungere attività invece di riconoscerle come già presenti.
- Corretto un bug più serio nella stessa importazione, che in rari casi poteva sovrascrivere dati più recenti con una versione più vecchia dello stesso evento.
- Corretto un bug per cui alcune impostazioni non comparivano cercandole dalla barra di ricerca in Impostazioni.
- Nuovo: attivando "Avvia con Windows", l'app si avvia ora in background senza aprire la finestra, restando comunque attiva nella system tray.
- Nuovo: la finestra principale ricorda ora schermo, posizione e dimensione tra un riavvio e l'altro.
- Se il sistema operativo non è né in italiano né in inglese, l'app si apre ora in inglese invece che in italiano.
- Al primo avvio, i moduli VPN, Claude Code, Excel e VoiSpeed partono ora disattivati di default; anche "Nascondi moduli/corsie Timeline quando vuoti" parte disattivato.
- Cambiati i moduli mostrati di default nella Home al primo avvio.
- Piccole rifiniture grafiche in Impostazioni (notifiche, integrazione Claude).

### en
- Fixed a bug where "Import buckets" (Watchers page) never actually worked when dragging or picking a real file — the import silently failed with no message.
- New: dragging a JSON/CSV file over the window now dims the page and highlights where to drop it with a gold border; once the import completes, it shows how many activities were added and how many were already present.
- Fixed a bug where re-importing the same file repeatedly kept adding activities instead of recognizing them as already present.
- Fixed a more serious bug in the same import feature that, in rare cases, could overwrite newer data with an older version of the same event.
- Fixed a bug where some settings didn't show up when searched for in the Settings search bar.
- New: enabling "Start with Windows" now launches the app in the background without opening the window, while staying fully active in the system tray.
- New: the main window now remembers its screen, position, and size between restarts.
- If the operating system isn't in Italian or English, the app now opens in English instead of Italian.
- On first launch, the VPN, Claude Code, Excel, and VoiSpeed modules now start disabled by default; "Hide modules/Timeline lanes when empty" also starts disabled.
- Changed the modules shown by default on the Home page on first launch.
- Small visual polish in Settings (notifications, Claude integration).

## 0.1.14

### it
- Nuovo: l'attività dei browser (Chrome, Firefox, Edge, Zen, Brave e molti altri) ha ora una corsia dedicata nella Timeline della Home, invece di riempire la corsia "Generale" con un blocco per ogni pagina/scheda visitata.
- Nuovo: nel dettaglio di un blocco della Timeline, quando ci sono più titoli distinti (es. più pagine visitate nello stesso browser), "Altre occorrenze" li mostra ora raggruppati per titolo invece che come un unico elenco mescolato.
- Nuovo: ogni titolo raggruppato ha un pulsante per vedere solo gli screenshot catturati durante gli orari di quel titolo specifico, non quelli dell'intero blocco.
- Corretto un bug per cui, cliccando un'attività specifica di un browser da "Titoli finestra principali", la Timeline evidenziava per sbaglio tutta la corsia (tutte le sessioni della giornata) invece della sola attività cliccata.

### en
- New: browser activity (Chrome, Firefox, Edge, Zen, Brave and many others) now gets its own dedicated lane in the Home Timeline, instead of filling the "General" lane with one block per page/tab visited.
- New: in a Timeline block's detail popup, when there are multiple distinct titles (e.g. several pages visited in the same browser), "Other occurrences" now groups them by title instead of one mixed list.
- New: each grouped title has a button to view only the screenshots captured during that specific title's own time ranges, not the whole block's.
- Fixed a bug where clicking a specific browser activity from "Top Window Titles" incorrectly highlighted the entire lane (every session that day) instead of just the clicked activity.

## 0.1.13

### it
- Nuovo: quando crei un watcher personalizzato puoi scegliere tra 8 modelli di visualizzazione pronti per la Home (stato acceso/spento, valore numerico, testo, barra di avanzamento, ultimo aggiornamento, elenco a pillole, tempo totale acceso oggi, classifica dei valori più frequenti oggi).
- Nuovo: ogni watcher personalizzato ha ora una pagina di dettaglio con stato del processo, tempo totale tracciato, un log in tempo reale di ogni esecuzione, e la possibilità di cambiare in qualsiasi momento lo script associato (con rilevamento automatico dell'interprete: PowerShell, Python, JavaScript, cmd/bat).
- Nuovo: la dimensione e il modello di visualizzazione scelti per un watcher personalizzato vengono ora ricordati, anche rimuovendo e riaggiungendo il suo modulo dalla Home.
- Nuovo: i watcher personalizzati appena creati includono un collegamento diretto alla documentazione online.
- Corretto un bug per cui eliminare un watcher personalizzato non fermava il suo processo né cancellava la sua cartella, lasciandolo "riapparire" da solo senza alcun controllo per rimuoverlo davvero.
- Corretto un crash dell'app che poteva verificarsi creando un nuovo watcher personalizzato.
- Corretto un bug per cui il campo "Numero eventi" nella pagina di dettaglio di una sorgente dati era sempre vuoto.
- Corretto un bug per cui la scelta del modello di visualizzazione durante la creazione di un watcher personalizzato mostrava solo l'opzione "predefinito" invece dell'elenco completo.
- Corretto un fastidioso lampeggiare di finestre nere (prompt dei comandi) quando un watcher personalizzato viene avviato.
- Corretto un bug per cui, dopo aver creato un watcher personalizzato, il pulsante "Apri" per vederne i dati poteva non comparire mai, restando bloccato su "in attesa di dati" anche a watcher funzionante.
- Rifatta la pagina di dettaglio di una sorgente dati: nuova timeline nello stesso stile di quella della Home (al posto della vecchia libreria non più coerente col resto dell'app), selettore dell'intervallo semplificato a "1h / 4h / Giorno", e sia la timeline che la tabella eventi ora si aggiornano da sole quasi in tempo reale.
- Rifatta anche la tabella eventi sotto la timeline: colonne chiare invece di un unico elenco di etichette, altezza fissa con scorrimento invece di allungare tutta la pagina.
- Corretto l'aspetto della finestra di modifica di un evento: i campi data/ora avevano uno stile chiaro stonato in mezzo a una finestra scura.
- Il pulsante "Cerca" nella barra in alto è ora nascosto nella pagina "Sorgenti dati", dove non aveva alcun effetto.
- Rifatta la documentazione online sui watcher personalizzati, ora divisa in guide separate e più complete.

### en
- New: when creating a custom watcher you can now pick from 8 ready-made visualization templates for its Home module (on/off status, numeric value, text, progress bar, last update, pill list, total on-time today, top values ranking today).
- New: every custom watcher now has a detail page with process status, total tracked time, a live log of every run, and the ability to change its associated script at any time (with automatic interpreter detection: PowerShell, Python, JavaScript, cmd/bat).
- New: the width and visualization template chosen for a custom watcher's Home module are now remembered, even if you remove and re-add it.
- New: newly created custom watchers now include a direct link to the online documentation.
- Fixed a bug where deleting a custom watcher didn't stop its process or delete its folder, causing it to silently reappear with no way to actually remove it.
- Fixed a crash that could occur when creating a new custom watcher.
- Fixed a bug where the "Event count" field on a data source's detail page was always empty.
- Fixed a bug where choosing a visualization template while creating a custom watcher only showed the "default" option instead of the full list.
- Fixed black command-prompt windows briefly flashing on screen when a custom watcher starts running.
- Fixed a bug where, after creating a custom watcher, the "Open" button to view its data could never appear, staying stuck on "waiting for data" even though the watcher was working fine.
- Redesigned a data source's detail page: a new timeline in the same style as the Home page's (replacing the old, no-longer-consistent library), a simplified time-range picker ("1h / 4h / Day"), and both the timeline and the events table now update on their own, near real-time.
- Also redesigned the events table below the timeline: clear columns instead of a single wall of tags, fixed height with scrolling instead of stretching the whole page.
- Fixed the look of the event-edit popup: the date/time fields had a jarring light style in the middle of a dark window.
- The "Search" button in the top bar is now hidden on the Data sources page, where it had no effect.
- Rewrote the online documentation for custom watchers into separate, more complete guides.

## 0.1.11

### it
- L'assistente AI ora categorizza subito tutte le app già tracciate non appena colleghi una chiave API, invece di aspettare che ognuna ricompaia per caso in Timeline.
- Le liste di app in Impostazioni (Notifiche e Categorizzazione) ora mostrano solo le app davvero comparse nella Timeline, non più anche processi di sistema in background mai usati attivamente.
- Corretto un bug per cui, dopo un aggiornamento, l'app poteva continuare a girare sulla versione precedente invece di quella appena installata.
- La disinstallazione ora rimuove davvero tutti i dati raccolti (in precedenza alcuni file potevano restare sul disco anche scegliendo di eliminarli).

### en
- The AI assistant now immediately categorizes all already-tracked apps as soon as you connect an API key, instead of waiting for each one to reappear in the Timeline by chance.
- App lists in Settings (Notifications and Categorization) now only show apps that actually appeared in the Timeline, no longer background system processes you never actively used.
- Fixed a bug where, after an update, the app could keep running the previous version instead of the newly installed one.
- Uninstalling now genuinely removes all collected data (previously some files could remain on disk even when choosing to delete them).

## 0.1.10

### it
- Corretto un bug per cui i moduli "Applicazioni principali" e "Titoli finestra principali" nella Home potevano mostrare "Nessun dato" tornando da un'altra pagina, pur essendoci dati reali.
- Corretto un piccolo difetto grafico in Impostazioni → Informazioni (titolo troppo vicino al riquadro sottostante).

### en
- Fixed a bug where the "Top Applications" and "Top Window Titles" Home modules could show "No data" after navigating back from another page, even with real data available.
- Fixed a minor visual glitch in Settings → About (section title sitting too close to the box below it).

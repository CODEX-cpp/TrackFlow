# Changelog

Ogni versione ha una sezione `### it` e una `### en` — l'app mostra solo
quella della lingua attiva, per la versione in esecuzione (vedi
`src-tauri/src/about.rs`, che scarica questo file da GitHub Pages).
Non toccare questo formato senza aggiornare anche quel parser.

## 0.1.23

### it
- Corretto un bug per cui la chat con l'assistente AI restava bloccata con il messaggio "serve prima una chiave API" anche con il provider "Claude (abbonamento Desktop)" correttamente collegato — quel provider non ha mai bisogno di una chiave, ma il controllo la richiedeva comunque.
- Corretto un bug per cui la categorizzazione automatica delle app non scattava mai con il provider "Claude (abbonamento Desktop)" — stessa causa del bug sopra, ma nella logica di categorizzazione, che ora funziona con entrambi i provider.
- Screenshot: ora organizzati automaticamente in una sottocartella per ogni giorno ("gg.mm.yyyy") invece di finire tutti insieme nella stessa cartella; gli screenshot già esistenti vengono ordinati da soli al primo avvio dopo l'aggiornamento, senza bisogno di alcuna azione manuale.

### en
- Fixed a bug where the AI assistant chat stayed stuck on "an API key is needed first" even with the "Claude (Desktop subscription)" provider correctly connected — that provider never needs a key, but the check required one anyway.
- Fixed a bug where automatic app categorization never ran with the "Claude (Desktop subscription)" provider — same root cause as the bug above, but in the categorization logic, which now works with both providers.
- Screenshots: now automatically organized into a per-day subfolder ("dd.mm.yyyy") instead of all landing in the same folder; existing screenshots are sorted automatically on the first launch after updating, no manual action needed.

## 0.1.21

### it
- **Nuova opzione per l'assistente AI**: usa l'abbonamento Claude Pro/Max già attivo su Claude Desktop invece di una chiave API Anthropic a consumo (Impostazioni → Agente AI → "Claude (abbonamento Desktop)") — nessuna chiave da procurarsi, stessi strumenti e le stesse limitazioni di sicurezza di prima (nessun accesso a file o comandi del sistema). Ora è l'opzione proposta di default a chi non ha ancora configurato nulla. La sessione resta attiva tra un messaggio e l'altro della stessa conversazione, così solo il primo messaggio paga il costo di avvio.
- Corretto un bug per cui, disattivando il watcher Excel dedicato, l'attività su Excel spariva del tutto dalla Timeline invece di tornare visibile nella corsia Generale come qualunque altra app.
- Il modulo Home "File Excel principali" mostra ora una classifica anche a watcher Excel disattivato, ricavata dai titoli delle finestre tracciate dal watcher generico invece di restare vuoto.

### en
- **New AI assistant option**: use the Claude Pro/Max subscription already active on Claude Desktop instead of a pay-per-token Anthropic API key (Settings → AI Agent → "Claude (Desktop subscription)") — no key to get, same tools and the same security restrictions as before (no access to files or system commands). Now the default option offered to anyone who hasn't configured anything yet. The session stays alive between messages of the same conversation, so only the first message pays the process startup cost.
- Fixed a bug where disabling the dedicated Excel watcher made Excel activity disappear entirely from the Timeline instead of showing up in the general lane like any other app.
- The Home "Top Excel Files" module now shows a ranking even with the Excel watcher disabled, derived from window titles tracked by the generic watcher instead of staying empty.

## 0.1.20

### it
- Cambiare giorno nella Timeline e nei moduli della Home è ora molto più veloce e fluido, specialmente scorrendo rapidamente tra più giorni: i giorni già visti restano pronti in memoria, e i moduli aggiornano i valori sul posto invece di sparire e riapparire ad ogni cambio.
- Corretto un bug per cui, lasciando l'app aperta a cavallo della mezzanotte, Home/Timeline/Moduli restavano bloccati sui dati del giorno precedente anche se la barra in alto mostrava già "Oggi".
- Corretto un bug segnalato da un utente (**@pupontech**, issue [#2](https://github.com/CODEX-cpp/TrackFlow/issues/2)) per cui il menu dell'icona nella barra delle applicazioni restava sempre in italiano, ignorando la lingua scelta nell'app — ora segue subito la lingua, senza bisogno di riavviare.
- Screenshot: corretta una drastica perdita di qualità con più monitor o uno schermo molto grande (ora la riduzione è proporzionata all'area totale, non più concentrata su un solo lato); alzata anche la qualità di base, prima i testi di dimensione media non si leggevano bene.
- Impostazioni → Screenshot: nuovo interruttore per catturare solo la finestra in primo piano invece di tutti i monitor uniti, e una nuova riga per aprire la cartella degli screenshot, vedere lo spazio occupato ed eliminarli tutti in un clic.
- L'assistente AI ora può rispondere a domande come "ho lavorato con qualche cliente oggi?" guardando direttamente i titoli reali delle finestre, invece di doverti chiedere quali parole cercare.
- VoiSpeed: corretto un bug per cui collegare l'account disconnetteva la sessione dell'app desktop usata per le chiamate; il rinnovo automatico dell'accesso ora avviene anche meno spesso.
- Impostazioni → Sviluppatore: nuova opzione per un log diagnostico avanzato (per indagare eventuali problemi di prestazioni), disattivato di default, con scelta della cartella dove salvarlo.

### en
- Switching days in the Timeline and Home modules is now much faster and smoother, especially when scrolling quickly through several days: already-seen days stay ready in memory, and modules update their values in place instead of disappearing and reappearing on every change.
- Fixed a bug where, with the app left open across midnight, Home/Timeline/Modules stayed stuck on the previous day's data even though the top bar already showed "Today".
- Fixed a bug reported by a user (**@pupontech**, issue [#2](https://github.com/CODEX-cpp/TrackFlow/issues/2)) where the tray icon's menu always stayed in Italian, ignoring the app's chosen language — it now follows the language immediately, no restart needed.
- Screenshots: fixed a drastic quality loss with multiple monitors or one very large screen (the reduction is now proportional to total area, not concentrated on a single dimension); also raised the base quality, since medium-sized text used to be hard to read.
- Settings → Screenshot: new toggle to capture only the active foreground window instead of all monitors combined, plus a new row to open the screenshots folder, see how much space they use, and delete them all in one click.
- The AI assistant can now answer questions like "did I work with any client today?" by looking directly at real window titles, instead of having to ask you which words to search for.
- VoiSpeed: fixed a bug where connecting the account would disconnect the desktop app session used for calls; automatic session renewal is now also less frequent.
- Settings → Developer: new option for an advanced diagnostics log (for troubleshooting performance issues), off by default, with a choice of folder to save it to.

## 0.1.19

### it
- VPN: la mappatura indirizzo→cliente ora si aggiorna in tempo reale (ogni ~15 secondi, non più ogni 30 minuti) — le notifiche per client non ancora mappati arrivano più in fretta e non risultano più sbagliate, e registrare un nuovo cliente aggiorna subito anche le sessioni storiche già in Timeline, senza dover riavviare l'app.
- Impostazioni → VPN: l'elenco degli indirizzi mappati è ora scorrevole invece di allungare la finestra all'infinito; "Aggiungi indirizzo" porta subito la vista alla nuova riga.
- La finestra, se chiusa massimizzata, torna davvero massimizzata al riavvio (prima tornava a una dimensione fissa che lasciava vedere il desktop attorno).
- Timeline: cliccando un singolo blocco (senza aver selezionato l'intera app da un modulo riepilogo) si vede ora l'elenco cronologico preciso delle attività di quel blocco, con orari precisi e attività sotto i 15 secondi filtrate — utile per trovare "cosa stavo facendo alle 12:11" senza scorrere decine di voci raggruppate per titolo.
- Corretto un bug per cui uno stesso file Excel aperto in sola lettura e poi in modifica appariva come due file diversi in Timeline — ora è un unico blocco, con la modalità (sola lettura/normale) indicata nel dettaglio di ogni sessione.
- Timeline: le corsie VPN, Claude, VS Code, Excel e VoiSpeed assegnano ora ad ogni nuovo client/file/progetto della giornata un colore ben distinguibile dagli altri già in uso, invece di un colore semi-casuale che poteva far sembrare uguali due elementi diversi.
- Nuovo pulsante nella Timeline per allegare un'attività direttamente alla chat dell'assistente AI (come rispondere a un messaggio) — l'assistente riceve così i dati esatti di quel blocco senza doverli cercare da solo. La finestra della chat è anche ridimensionabile a mano e ricorda la dimensione scelta tra un riavvio e l'altro.

### en
- VPN: the address→client mapping now updates in real time (every ~15 seconds instead of every 30 minutes) — notifications for unmapped clients arrive faster and are no longer occasionally wrong, and registering a new client immediately updates historical sessions already in the Timeline too, without needing an app restart.
- Settings → VPN: the mapped addresses list now scrolls instead of stretching the window indefinitely; "Add address" jumps the view straight to the new row.
- The window, if closed while maximized, now genuinely returns maximized on restart (it used to come back at a fixed size that left the desktop visible around it).
- Timeline: clicking a single block (without an entire app selected from a summary module) now shows a precise chronological list of that block's activity, with exact times and activities under 15 seconds filtered out — useful for finding "what was I doing at 12:11" without scrolling dozens of title-grouped entries.
- Fixed a bug where the same Excel file opened read-only and then editable showed up as two different files in the Timeline — it's now a single block, with the mode (read-only/normal) shown in each session's detail.
- Timeline: the VPN, Claude, VS Code, Excel and VoiSpeed lanes now give each new client/file/project of the day a color clearly distinguishable from the ones already in use, instead of a semi-random one that could make two different things look the same.
- New button in the Timeline to attach an activity directly to the AI assistant chat (like replying to a message) — the assistant gets that block's exact data without having to look it up itself. The chat window is also resizable by hand and remembers the chosen size across restarts.

## 0.1.18

### it
- Corretto un bug per cui il database poteva corrompersi dopo una chiusura non pulita (es. "Termina attività" da Gestione attività), lasciando l'app con lo schermo bianco al riavvio successivo. Ora, ad ogni avvio, l'app controlla l'integrità del database e, se lo trova danneggiato, si ripristina da sola dall'ultimo backup automatico (il file danneggiato non viene mai cancellato, resta disponibile per un recupero manuale). L'app si fa anche un backup automatico ogni 3 ore quando il database è sano.
- Nuovo modulo Home "Flusso di lavoro": una riga per ciascuna delle tue categorie, colonne da 15 minuti sull'arco della giornata, colorate in base a quanto di quello slot era davvero occupato da attività di quella categoria; un'ultima riga mostra i periodi di assenza dal PC. Passando il mouse su un quadratino, la Timeline evidenzia l'esatto intervallo corrispondente con due linee e scurisce il resto.
- Impostazioni → Categorizzazione: ogni categoria ha ora un'icona a forma di matita per assegnarle un colore a scelta tra 21 tonalità tenui — usato ovunque compaia un colore di categoria (barra Categorie, diagramma ad albero, modulo "Flusso di lavoro").
- Corretto un bug per cui il changelog in Impostazioni → Info poteva comparire in inglese anche con l'app impostata in italiano.
- Timeline: le icone dentro le barre hanno ora uno sfondo proprio, per restare visibili anche quando il colore dell'icona coincide con quello della barra.

### en
- Fixed a bug where the database could get corrupted after an unclean shutdown (e.g. "End task" from Task Manager), leaving the app stuck on a white screen on the next launch. Now, on every startup, the app checks the database's integrity and, if it finds it damaged, restores itself from the latest automatic backup (the damaged file is never deleted, and stays available for manual recovery). The app also takes an automatic backup every 3 hours while the database is healthy.
- New Home module "Workflow": one row per category, 15-minute columns across the day, colored by how much of that slot was actually covered by activity in that category; a final row shows time away from the PC. Hovering a cell highlights the matching time range in the Timeline with two lines and dims the rest.
- Settings → Categorization: every category now has a pencil icon to assign it a color from 21 muted tones — used everywhere a category color appears (Categories bar, treemap, "Workflow" module).
- Fixed a bug where the changelog in Settings → About could show in English even with the app set to Italian.
- Timeline: icons inside bars now have their own background, so they stay visible even when the icon's color matches the bar's.

## 0.1.17

### it
- Corretto un bug per cui la finestra risultava invisibile al riavvio se era stata chiusa minimizzata: Windows salvava una posizione/dimensione fittizia, e la finestra ripartiva fuori schermo e microscopica.
- Rimossa la finestra della console che si apriva ad ogni avvio dell'app.
- Nuovo modulo Home "Ore lavorate (calendario)": una griglia stile "contributi GitHub" degli ultimi 6 mesi, colorata in base alle ore effettivamente lavorate rispetto a un budget giornaliero configurabile (Impostazioni → orari di lavoro).
- Nuovo modulo Home "Categorie (treemap)": un diagramma ad albero proporzionale che mostra il tempo per categoria, con le app più usate di ciascuna categoria annidate all'interno — le voci troppo piccole per essere leggibili si accorpano automaticamente in "Altro" o vengono nascoste.
- Modalità "Modifica moduli" nella Home: trascinare un modulo ora lo fissa nella colonna scelta invece di farlo ricadere sempre nella colonna più corta, con un'anteprima tratteggiata di dove verrà rilasciato. Corretto anche un bug di animazione per cui, rilasciando un modulo, quelli sotto potevano accavallarsi o lasciare uno spazio vuoto.
- Timeline: le barre ora mostrano icona e nome dell'app quando c'è spazio a sufficienza, aggiornandosi dal vivo durante lo zoom; il testo passa automaticamente a nero sui colori più chiari per restare leggibile.

### en
- Fixed a bug where the window was invisible on restart if it had been closed while minimized: Windows saved a fake off-screen position/size, and the window came back tiny and off-screen.
- Removed the console window that used to open on every app launch.
- New Home module "Hours worked (calendar)": a GitHub-contributions-style grid of the last 6 months, colored by actual hours worked against a configurable daily budget (Settings → working hours).
- New Home module "Categories (treemap)": a proportional treemap showing time per category, with each category's most-used apps nested inside — entries too small to read merge automatically into "Other" or are hidden.
- Home "Edit modules" mode: dragging a module now pins it to the chosen column instead of always falling back to the shortest one, with a dashed preview of where it will land. Also fixed an animation bug where releasing a module could make the ones below overlap or leave an empty gap.
- Timeline: bars now show the app's icon and name when there's enough room, updating live while zooming; the text automatically switches to black on lighter colors to stay readable.

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

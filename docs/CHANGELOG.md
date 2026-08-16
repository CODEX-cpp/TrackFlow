# Changelog

Ogni versione ha una sezione `### it` e una `### en` — l'app mostra solo
quella della lingua attiva, per la versione in esecuzione (vedi
`src-tauri/src/about.rs`, che scarica questo file da GitHub Pages).
Non toccare questo formato senza aggiornare anche quel parser.

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

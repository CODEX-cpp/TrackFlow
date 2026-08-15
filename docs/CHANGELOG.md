# Changelog

Ogni versione ha una sezione `### it` e una `### en` — l'app mostra solo
quella della lingua attiva, per la versione in esecuzione (vedi
`src-tauri/src/about.rs`, che scarica questo file da GitHub Pages).
Non toccare questo formato senza aggiornare anche quel parser.

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

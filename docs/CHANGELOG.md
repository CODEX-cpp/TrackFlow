# Changelog

Ogni versione ha una sezione `### it` e una `### en` — l'app mostra solo
quella della lingua attiva, per la versione in esecuzione (vedi
`src-tauri/src/about.rs`, che scarica questo file da GitHub Pages).
Non toccare questo formato senza aggiornare anche quel parser.

## 0.1.10

### it
- Corretto un bug per cui i moduli "Applicazioni principali" e "Titoli finestra principali" nella Home potevano mostrare "Nessun dato" tornando da un'altra pagina, pur essendoci dati reali.
- Corretto un piccolo difetto grafico in Impostazioni → Informazioni (titolo troppo vicino al riquadro sottostante).

### en
- Fixed a bug where the "Top Applications" and "Top Window Titles" Home modules could show "No data" after navigating back from another page, even with real data available.
- Fixed a minor visual glitch in Settings → About (section title sitting too close to the box below it).

## 0.1.9

### it
- Solo infrastruttura interna: automazione delle release da GitHub Actions — nessun cambiamento visibile nell'app.

### en
- Internal infrastructure only: GitHub Actions release automation — no user-visible change.

## 0.1.8

### it
- **Novità**: avvio automatico con Windows, opzionale (Impostazioni → Generale), attivo di default.
- Le icone delle app già aperte vengono ora estratte subito all'avvio, invece di aspettare che l'app diventi la finestra attiva.
- Il controllo aggiornamenti riparte in modo affidabile quando la finestra torna in primo piano dalla tray.
- Corretto il nome mostrato in Gestione attività → Avvio ("TrackFlow" invece di "TrackFlow launcher").
- **Novità**: pagina Impostazioni → Info, con versione e novità della release (ora scaricate da qui, non più impacchettate nell'installer).

### en
- **New**: optional autostart with Windows (Settings → General), enabled by default.
- App icons for already-open apps are now captured right at startup, instead of waiting for the app to become the active window.
- Update checks now reliably fire when bringing the window back to the foreground from the tray.
- Fixed the name shown in Task Manager's Startup tab ("TrackFlow" instead of "TrackFlow launcher").
- **New**: Settings → About page, with version and release notes (now fetched from here, no longer bundled with the installer).

## 0.1.2

### it
- **Novità**: sistema di aggiornamento automatico — controllo, download, verifica della firma digitale e installazione con un semplice popup "Riavvia per aggiornare".
- Corretta l'icona generica mostrata al posto di quella di TrackFlow (Start Menu, ricerca, notifiche).
- Chiave API di Claude cifrata a riposo con DPAPI di Windows.

### en
- **New**: automatic update system — check, download, digital signature verification, and installation via a simple "Restart to update" popup.
- Fixed the generic icon shown instead of TrackFlow's own (Start Menu, search, notifications).
- Claude API key now encrypted at rest with Windows DPAPI.

## 0.1.0

### it
- Prima pubblicazione pubblica.

### en
- First public release.

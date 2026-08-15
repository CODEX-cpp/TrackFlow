# TrackFlow

Un'app desktop per Windows che traccia automaticamente come passi il tempo al lavoro — app usate, finestre attive, sessioni VPN, chiamate VoiSpeed, sessioni Claude Code, screenshot periodici — e le organizza in una Timeline giornaliera, moduli riassuntivi in Home, e progetti con cronometro dedicato.

È un fork di [ActivityWatch](https://activitywatch.net/), riscritto quasi per intero: il backend Python originale (server, watcher, `aw-notify`) è stato sostituito con un unico processo Rust incorporato in [Tauri](https://tauri.app/) — nessun server esterno, nessuna porta di rete aperta, tutto gira in-process nella stessa app — e la webui è stata pesantemente ridisegnata sopra la base Vue 2 originale.

**TrackFlow non è affiliato al progetto ActivityWatch principale.** Segue i requisiti ufficiali per i fork ([docs.activitywatch.net/en/latest/forking.html](https://docs.activitywatch.net/en/latest/forking.html)): nome e logo propri, nessuna associazione con il progetto originale, stessa licenza (MPL-2.0), codice sorgente pubblico.

## Funzionalità principali

- **Timeline giornaliera** con corsie per app, VPN, Claude Code, VS Code, Excel, VoiSpeed, browser
- **Moduli Home** riordinabili — Top App, Top Titoli Finestra, Uso Claude, e altro
- **Categorizzazione app→categoria**, assegnabile a mano o in automatico da un agente AI (Claude)
- **Progetti** con cronometro avvia/pausa, budget ore, scadenze e avvisi di sforamento
- **Notifiche personalizzate** — regole configurabili per categoria/app/progetto/inattività/VPN, consegnate come notifiche native di Windows
- **Watcher dedicati**: finestra attiva, inattività (AFK), sessioni VPN (OpenVPN Connect + ZyWALL SecuExtender), VoiSpeed, Claude Code, VS Code, Excel, screenshot periodici, icone app
- **Filtri privacy** configurabili — scartano o oscurano dati sensibili prima ancora che vengano salvati su disco
- **Chat con un agente AI** (Claude) che risponde a domande sui propri dati di attività

## Stack tecnico

- **Frontend**: Vue 2 + TypeScript + Pinia + Vite
- **Backend/shell**: Rust + [Tauri 2](https://tauri.app/) — un unico processo, server ActivityWatch (`aw-server-rust`, vendored con patch locali) incorporato in-process, nessuna rete
- **Watcher**: ognuno un piccolo binario Rust indipendente, lanciato come sidecar da Tauri e comunicante via stdout/JSON
- **Solo Windows** per ora (i watcher usano API Win32 dirette per gran parte delle funzionalità)

## Compilazione

### Prerequisiti

- [Node.js](https://nodejs.org/) 18+ e npm
- [Rust](https://rustup.rs/) (toolchain stabile) + target MSVC su Windows
- [Tauri CLI](https://v2.tauri.app/start/prerequisites/) (installata automaticamente come dipendenza npm, vedi `package.json`)

### Sviluppo

```bash
npm install
npx tauri dev
```

Avvia il frontend (Vite, hot reload) e l'app Tauri insieme, puntata al server di sviluppo.

### Build di produzione

```bash
npm run build      # compila il frontend in dist/
npx tauri build     # compila l'intera app + genera gli installer (MSI e NSIS) in src-tauri/target/release/bundle/
```

L'installer NSIS (`*-setup.exe`) è quello pensato per la distribuzione normale; l'MSI è disponibile come alternativa.

### Altri comandi utili

```bash
npm run serve   # solo il frontend, nel browser (senza la shell Tauri/i dati reali)
npm run lint    # ESLint su src/ e test/
```

## Struttura del progetto

```
src/                        webui (Vue 2 + TypeScript + Pinia)
src-tauri/                  shell Tauri (Rust) — comandi, tray, notifiche, server in-process
aw-server-rust-src/         server ActivityWatch vendored, con patch locali (vedi commenti nel codice)
aw-watcher-*-rust/          watcher indipendenti (VPN, AFK, finestra, VoiSpeed-adiacenti, screenshot, ecc.)
```

## Licenza

[Mozilla Public License 2.0](LICENSE.txt) — stessa licenza del progetto originale [ActivityWatch](https://github.com/ActivityWatch/activitywatch).

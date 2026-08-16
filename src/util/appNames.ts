// Best-effort mapping from a raw process/executable name (as recorded
// by aw-watcher-window, e.g. "Code.exe") to a friendlier display name
// and an icon.
//
// The icon is a REAL icon extracted from the actual .exe on the
// Windows machine by aw-watcher-app-icons (Rust watcher, runs
// continuously in the background — see BLUEPRINT.md's Fase 1/Fase 4).
// It writes into a directory the current OS user can always write to
// (%LOCALAPPDATA%\TrackFlow\app-data\, not the app's own install
// directory — that one's typically read-only under Program Files
// without elevation, which was a real bug found and fixed while
// building the Tauri shell: icons/colors/names discovered AFTER the
// app was installed silently never showed up otherwise). aw-server-rust
// serves that directory at /pages/app-data/ via --custom-static.
//
// appIconColors.json/appAutoNames.json bundled at build time (imported
// below) are only the BASELINE known when this build was made — colors.
// json/names.json fetched at runtime from /pages/app-data/ on top of
// that baseline are always more current, since the watcher keeps
// writing there live, independent of whenever the webui itself was
// last rebuilt. iconUrlForApp() points straight at that same live
// directory too, no baseline needed there (an <img> 404 already falls
// back to the emoji below, whether the icon predates this build or not).
import appIconColorsBaseline from './appIconColors.json';
import appAutoNamesBaseline from './appAutoNames.json';

// Copie mutabili: partono dal baseline compilato nel bundle, poi
// vengono aggiornate (mai sostituite del tutto, solo ampliate/
// sovrascritte voce per voce) dal fetch a runtime qui sotto.
const appIconColors: Record<string, string> = { ...(appIconColorsBaseline as Record<string, string>) };
const appAutoNames: Record<string, string> = { ...(appAutoNamesBaseline as Record<string, string>) };

const APP_DATA_URL = '/pages/app-data';

// Richiamata una volta all'avvio dell'app (vedi App.vue) per allineare
// colori/nomi con quello che il watcher ha scoperto dall'ultima build
// della webui in poi. Fallisce silenziosamente se il server non serve
// ancora quel percorso (--custom-static non configurato, o prima
// esecuzione senza dati ancora scritti) — resta valido il baseline.
// Elenco delle app "conosciute" — ogni chiave normalizzata mai vista da
// aw-watcher-app-icons (icona estratta) o dall'auto-namer (nome
// ricavato dai metadati dell'exe), unite: le due fonti non si
// sovrappongono mai perfettamente. Usato dal costruttore di regole di
// notifica (NotificationRulesSettings.vue) per offrire un menu a
// tendina invece di far scrivere il nome dell'eseguibile a mano.
export function knownAppKeys(): string[] {
  return Array.from(new Set([...Object.keys(appIconColors), ...Object.keys(appAutoNames)])).sort();
}

export async function refreshDynamicAppData(): Promise<void> {
  await Promise.all([
    fetch(`${APP_DATA_URL}/appIconColors.json`)
      .then(r => (r.ok ? r.json() : null))
      .then(data => data && Object.assign(appIconColors, data))
      .catch(() => undefined),
    fetch(`${APP_DATA_URL}/appAutoNames.json`)
      .then(r => (r.ok ? r.json() : null))
      .then(data => data && Object.assign(appAutoNames, data))
      .catch(() => undefined),
  ]);
}

interface AppInfo {
  name: string;
  fallbackIcon: string;
}

const APP_INFO: Record<string, AppInfo> = {
  'code.exe': { name: 'VS Code', fallbackIcon: '🆚' },
  'code - insiders.exe': { name: 'VS Code Insiders', fallbackIcon: '🆚' },
  'zen.exe': { name: 'Zen Browser', fallbackIcon: '🦊' },
  'firefox.exe': { name: 'Firefox', fallbackIcon: '🦊' },
  'chrome.exe': { name: 'Chrome', fallbackIcon: '🌐' },
  'msedge.exe': { name: 'Edge', fallbackIcon: '🌐' },
  'brave.exe': { name: 'Brave', fallbackIcon: '🦁' },
  'opera.exe': { name: 'Opera', fallbackIcon: '🌐' },
  'claude.exe': { name: 'Claude', fallbackIcon: '✳️' },
  'windowsterminal.exe': { name: 'Windows Terminal', fallbackIcon: '💻' },
  'powershell.exe': { name: 'PowerShell', fallbackIcon: '💻' },
  'pwsh.exe': { name: 'PowerShell', fallbackIcon: '💻' },
  'cmd.exe': { name: 'Prompt dei comandi', fallbackIcon: '💻' },
  'vncviewer.exe': { name: 'RealVNC Viewer', fallbackIcon: '🖥️' },
  'mstsc.exe': { name: 'Connessione desktop remoto', fallbackIcon: '🖥️' },
  'explorer.exe': { name: 'Esplora risorse', fallbackIcon: '📁' },
  'excel.exe': { name: 'Excel', fallbackIcon: '📊' },
  'winword.exe': { name: 'Word', fallbackIcon: '📝' },
  'outlook.exe': { name: 'Outlook', fallbackIcon: '📧' },
  'onenote.exe': { name: 'OneNote', fallbackIcon: '📔' },
  'teams.exe': { name: 'Microsoft Teams', fallbackIcon: '💬' },
  'slack.exe': { name: 'Slack', fallbackIcon: '💬' },
  'discord.exe': { name: 'Discord', fallbackIcon: '💬' },
  'notepad.exe': { name: 'Blocco note', fallbackIcon: '📝' },
  'notepad++.exe': { name: 'Notepad++', fallbackIcon: '📝' },
  'sublime_text.exe': { name: 'Sublime Text', fallbackIcon: '📝' },
  'idea64.exe': { name: 'IntelliJ IDEA', fallbackIcon: '🧠' },
  'pycharm64.exe': { name: 'PyCharm', fallbackIcon: '🐍' },
  'spotify.exe': { name: 'Spotify', fallbackIcon: '🎵' },
  'acrord32.exe': { name: 'Acrobat Reader', fallbackIcon: '📕' },
  'acrobat.exe': { name: 'Acrobat', fallbackIcon: '📕' },
  'taskmgr.exe': { name: 'Gestione attività', fallbackIcon: '📊' },
  'snippingtool.exe': { name: 'Strumento di cattura', fallbackIcon: '✂️' },
};

const GENERIC_ICON = '🗔';

// Windows system/shell processes that show up in the window-watcher
// bucket but were never something the user actually "used" — mostly
// invisible host processes for the taskbar, search, notifications,
// etc. Hidden from Top Applications/Titles and the Timeline's Generale
// lane so they don't clutter either with noise nobody asked to track.
// Deliberately NOT here: cmd.exe/powershell.exe/pwsh.exe (explicit
// request to keep those — they're real work, not shell chrome) and
// anything not a stock Windows component.
const HIDDEN_SYSTEM_APPS = new Set([
  'searchhost.exe',
  'searchapp.exe',
  'startmenuexperiencehost.exe',
  'shellexperiencehost.exe',
  'sihost.exe',
  'dwm.exe',
  'lockapp.exe',
  'logonui.exe',
  'textinputhost.exe',
  'applicationframehost.exe',
  'runtimebroker.exe',
  'systemsettings.exe',
  'backgroundtaskhost.exe',
  'taskhostw.exe',
  'ctfmon.exe',
  'explorer.exe',
  'widgets.exe',
  'widgetservice.exe',
  'securityhealthsystray.exe',
  'gamebar.exe',
  'gamebarftserver.exe',
]);

function normalize(rawName: string): string {
  return (rawName || '').trim().toLowerCase();
}

export function isHiddenSystemApp(rawName: string): boolean {
  return HIDDEN_SYSTEM_APPS.has(normalize(rawName));
}

const VSCODE_APPS = new Set(['code.exe', 'code - insiders.exe']);

export function isVSCodeApp(rawName: string): boolean {
  return VSCODE_APPS.has(normalize(rawName));
}

export function isExcelApp(rawName: string): boolean {
  return normalize(rawName) === 'excel.exe';
}

// Nomi .exe Windows dei browser più comuni — richiesta esplicita
// "una lista di browser il più completa possibile", usata per
// separare l'attività di navigazione dalla corsia "Generale" della
// Timeline (vedi HomeTimelineSection.vue). Solo nomi processo esatti
// (a differenza di browser_appname_regex in queries.ts, pensato per
// bucket ID multi-piattaforma delle estensioni browser) — sufficiente
// per l'app watcher finestra su Windows, dove il nome del processo non
// varia per versione/piattaforma come su Linux/macOS.
const BROWSER_APPS = new Set([
  'chrome.exe',
  'chromium.exe',
  'msedge.exe',
  'firefox.exe',
  'librewolf.exe',
  'waterfox.exe',
  'brave.exe',
  'opera.exe',
  'opera_gx.exe',
  'vivaldi.exe',
  'zen.exe',
  'arc.exe',
  'browser.exe',
  'floorp.exe',
  'helium.exe',
  'orion.exe',
  'iexplore.exe',
  'seamonkey.exe',
  'maxthon.exe',
  'ucbrowser.exe',
  'coccoc.exe',
  'epic.exe',
  'palemoon.exe',
  'basilisk.exe',
  'avastbrowser.exe',
  'avgbrowser.exe',
  'whale.exe',
  'sleipnir.exe',
  'torch.exe',
  'iron.exe',
  'dragon.exe',
  'icedragon.exe',
  'min.exe',
  'colibri.exe',
  'wavebox.exe',
  'superbird.exe',
  'thorium.exe',
]);

export function isBrowserApp(rawName: string): boolean {
  return BROWSER_APPS.has(normalize(rawName));
}

// VS Code's window title encodes what's open as "<file> - <project> -
// Visual Studio Code" once a folder/workspace is open, or just
// "<file> - Visual Studio Code" with a single file and no folder, or
// bare "Visual Studio Code" with nothing open. Top Window Titles wants
// the project name when there is one (it's already covered file-by-file
// by the Top Editor Files panel), falling back to the file name when
// there's no project — so this picks whichever segment that is instead
// of showing the raw, noisier title.
export function vscodeTitleDisplayName(rawTitle: string): string {
  const parts = (rawTitle || '').split(' - ').map(p => p.trim());
  if (parts.length >= 3) return parts[parts.length - 2];
  if (parts.length === 2) return parts[0];
  return rawTitle;
}

export function displayNameForApp(rawName: string): string {
  if (!rawName) return rawName;
  const key = normalize(rawName);
  // Curated (Italian, hand-picked) name wins when it exists; otherwise
  // fall back to the English name aw-watcher-app-icons auto-extracted
  // from the exe's own metadata (see appAutoNames.json) — better than
  // the raw exe filename, but not a substitute for adding a proper
  // entry to APP_INFO above when you notice one.
  return APP_INFO[key]?.name || appAutoNames[key] || rawName;
}

// Emoji shown while the real icon hasn't loaded yet / doesn't exist —
// used as the <img>'s error fallback, not as the primary icon anymore.
export function fallbackIconForApp(rawName: string): string {
  return APP_INFO[normalize(rawName)]?.fallbackIcon || GENERIC_ICON;
}

// Expected URL for the real extracted icon — see the file header for
// how it gets there. Always returns a guess; the caller has to handle
// the case where nothing actually exists at that URL.
export function iconUrlForApp(rawName: string): string {
  return `${APP_DATA_URL}/app-icons/${normalize(rawName)}.png`;
}

// A per-app color pulled from the app's real icon (dominant hue,
// pushed into a warm/pastel range) instead of the generic hash-per-
// name coloring in util/hashColor.ts — computed by aw-watcher-app-icons
// for every new icon it extracts (see the file header for where it
// writes it and how it gets here). Returns null for anything not
// known yet (unmapped app, or icon too neutral/gray to have a
// dominant color) — the caller falls back to colorVarForName in that
// case, same pattern as fallbackIconForApp above.
export function iconColorForApp(rawName: string): string | null {
  return appIconColors[normalize(rawName)] || null;
}

// Blocca tasto destro e le scorciatoie che aprono i DevTools (F12,
// Ctrl+Shift+I/J, Ctrl+U) quando l'opzione "DevTools" in Impostazioni →
// Sviluppatore è spenta — installato una sola volta all'avvio (vedi
// main.js) e reattivo ai cambi di quell'impostazione, non serve
// riavviare l'app per far scattare/togliere il blocco.
//
// Nota: questo blocca solo i MODI PER APRIRE i DevTools, non forza la
// chiusura di una finestra DevTools già aperta — `close_devtools()` di
// Tauri/wry è un no-op su Windows (vedi src-tauri/src/devtools.rs), non
// esiste un modo per chiuderla da codice su questa piattaforma.
import { useSettingsStore } from '~/stores/settings';

function isDevtoolsShortcut(e: KeyboardEvent): boolean {
  if (e.key === 'F12') return true;
  const key = e.key.toLowerCase();
  if (e.ctrlKey && e.shiftKey && (key === 'i' || key === 'j' || key === 'c')) return true;
  if (e.ctrlKey && key === 'u') return true;
  return false;
}

export function installDevtoolsGuard(): void {
  document.addEventListener(
    'contextmenu',
    e => {
      if (!useSettingsStore().devtoolsEnabled) {
        e.preventDefault();
      }
    },
    true
  );

  document.addEventListener(
    'keydown',
    e => {
      if (!useSettingsStore().devtoolsEnabled && isDevtoolsShortcut(e)) {
        e.preventDefault();
        e.stopPropagation();
      }
    },
    true
  );
}

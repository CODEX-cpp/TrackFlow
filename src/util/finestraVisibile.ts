// Stato condiviso "la finestra è visibile adesso?" + un helper per
// timer periodici che si fermano da soli quando non lo è.
//
// Perché serve: nascondere la finestra (tray, X, Alt+F4) non la
// distrugge mai — resta lo stesso processo, stessa webview, stesso
// Vue mai smontato (scelta esplicita, vedi App.vue). Prima di questo
// modulo, NIENTE in tutto il programma teneva conto di questo: ogni
// `setInterval` di refresh periodico (Timeline, moduli Home, widget,
// poll di Impostazioni) continuava a fare query di rete + ricostruzione
// DOM ogni pochi secondi per tutta la durata in cui l'app restava in
// tray — anche ore, lasciando il PC acceso tutto il giorno al lavoro.
// Causa diagnosticata di un consumo di RAM anomalo del processo
// webview, segnalato dall'utente (2026-10-03).
//
// Ascolta i due eventi espliciti emessi da lib.rs esattamente nei punti
// in cui Rust decide di nascondere/mostrare la finestra
// (`emit_finestra_nascosta`/`emit_finestra_mostrata`) — non la Page
// Visibility API del browser né `onFocusChanged` di Tauri: entrambi si
// sono già rivelati inaffidabili in questo programma per il caso
// opposto (vedi commento in App.vue su "mostra"), quindi non c'è motivo
// di fidarsi dell'uno o dell'altro qui.
import { listen } from '@tauri-apps/api/event';
import { logEvento } from '~/util/diagnostics';

// Vero finché non si riceve il primo evento di "nascosta" — la finestra
// nasce sempre visibile.
let visibile = true;
const ascoltatori = new Set<(visibile: boolean) => void>();

let avviato = false;

// Aggancia i due listener Tauri una sola volta per sessione (stesso
// pattern di avviato/abilitata già usato in util/diagnostics.ts).
// Chiamata da App.vue all'avvio, prima che qualunque componente possa
// creare un intervallo pausabile.
export function avviaAscoltoVisibilita(): void {
  if (avviato) return;
  avviato = true;
  try {
    listen('trackflow://finestra-nascosta', () => impostaVisibile(false));
    listen('trackflow://finestra-mostrata', () => impostaVisibile(true));
  } catch {
    // Fuori da Tauri (npx vite puro) — listen() non emetterà mai,
    // resta semplicemente sempre visibile: non bloccante.
  }
}

function impostaVisibile(valore: boolean): void {
  if (valore === visibile) return;
  visibile = valore;
  // No-op se il log diagnostico (Impostazioni → Sviluppatore) è
  // spento — marcatore utile per correlare a mano, in analisi, la
  // crescita di RAM con l'esatto momento in cui i timer si fermano o
  // ripartono (vedi diagnostics.rs per il resto dell'indagine RAM).
  logEvento('finestra_visibilita', { visibile: valore });
  for (const cb of ascoltatori) cb(visibile);
}

export function finestraVisibile(): boolean {
  return visibile;
}

/// Crea un intervallo che si ferma da solo quando la finestra si
/// nasconde e riparte da solo quando torna visibile — chiamando `fn`
/// immediatamente alla ripresa (altrimenti, tornando dopo ore in tray,
/// si vedrebbero dati vecchi finché non scatta il prossimo giro
/// naturale, fino a `ms` dopo).
///
/// Uso: sostituisce 1:1 `setInterval(fn, ms)` nei `mounted()` dei
/// componenti con refresh periodico — il `.ferma()` del risultato
/// sostituisce il corrispondente `clearInterval()` in `beforeDestroy()`.
export function creaIntervalloPausabile(fn: () => void, ms: number): { ferma: () => void } {
  let handle: ReturnType<typeof setInterval> | null = null;

  const avviaTimer = () => {
    if (handle) return;
    handle = setInterval(fn, ms);
  };
  const fermaTimer = () => {
    if (!handle) return;
    clearInterval(handle);
    handle = null;
  };

  const suCambioVisibilita = (visibileOra: boolean) => {
    if (visibileOra) {
      fn();
      avviaTimer();
    } else {
      fermaTimer();
    }
  };

  if (visibile) avviaTimer();
  ascoltatori.add(suCambioVisibilita);

  return {
    ferma() {
      fermaTimer();
      ascoltatori.delete(suCambioVisibilita);
    },
  };
}

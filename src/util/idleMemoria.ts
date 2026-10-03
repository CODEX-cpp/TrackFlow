// Dice esplicitamente a WebView2 di liberare RAM quando l'utente resta
// fermo qualche secondo, invece di aspettare che Chromium decida da
// solo — richiesta esplicita dell'utente dopo aver misurato con un log
// diagnostico reale che la RAM nativa (GPU/compositor) resta alta per
// decine di secondi dopo che si smette di interagire, anche se lo heap
// JS e i nodi DOM sono già piatti (2026-10-03). Vedi
// `imposta_livello_memoria` in lib.rs per l'API WebView2 vera.
//
// Nessun impatto visivo: "basso" è solo un suggerimento al motore,
// tornare a "normale" alla prima interazione è istantaneo.
import { invoke } from '@tauri-apps/api/core';

const SOGLIA_INATTIVITA_MS = 5000;

let timerInattivita: ReturnType<typeof setTimeout> | null = null;
let livelloBassoAttivo = false;
let avviato = false;

function impostaLivello(basso: boolean): void {
  if (basso === livelloBassoAttivo) return;
  livelloBassoAttivo = basso;
  try {
    invoke('imposta_livello_memoria', { basso }).catch(() => {});
  } catch {
    // Fuori da Tauri (npx vite puro) — non bloccante.
  }
}

function suAttivita(): void {
  impostaLivello(false);
  if (timerInattivita) clearTimeout(timerInattivita);
  timerInattivita = setTimeout(() => impostaLivello(true), SOGLIA_INATTIVITA_MS);
}

// Chiamata una sola volta da App.vue — stesso pattern "avviato" già
// usato in util/diagnostics.ts e util/finestraVisibile.ts.
export function avviaTrimMemoriaInattiva(): void {
  if (avviato) return;
  avviato = true;
  // passive: true — questi listener non devono mai bloccare lo scroll o
  // il rendering, leggono solo "è successo qualcosa".
  const opzioni = { passive: true, capture: true } as const;
  document.addEventListener('mousemove', suAttivita, opzioni);
  document.addEventListener('mousedown', suAttivita, opzioni);
  document.addEventListener('keydown', suAttivita, opzioni);
  document.addEventListener('wheel', suAttivita, opzioni);
  document.addEventListener('touchstart', suAttivita, opzioni);
  suAttivita();
}

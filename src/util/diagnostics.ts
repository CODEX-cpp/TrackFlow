// Modulo diagnostico avanzato (perf), disattivato di default — vedi
// src-tauri/src/diagnostics.rs (dove finiscono davvero questi log) e
// Impostazioni → Sviluppatore per attivarlo/disattivarlo e scegliere la
// cartella di destinazione. `avvia()` è chiamato da App.vue solo se
// l'impostazione è attiva: se disattivata, questo modulo non aggancia
// nessun listener/interval, non solo non scrive su disco.
import { invoke } from '@tauri-apps/api/core';

// Specchio locale, lato JS, dell'impostazione 'diagnosticsLoggingEnabled'
// — evita di fare un invoke() IPC (che poi si scarterebbe comunque lato
// Rust, vedi diagnostics.rs's ATTIVA) per OGNI query/evento quando il
// log è disattivato: `strumentaClient()` avvolge ogni chiamata di rete
// dei client AW, quindi senza questo controllo qui il costo si
// pagherebbe comunque anche a log disattivato.
let abilitata = false;

// Chiamato da App.vue all'avvio (con lo stato salvato) e dal toggle in
// Impostazioni → Sviluppatore (per applicarlo subito, senza dover
// riavviare l'app). Aggancia i listener di performance/memoria SOLO la
// prima volta che viene attivato in questa sessione.
export function setAbilitata(valore: boolean): void {
  abilitata = valore;
  if (valore) avvia();
}

export function logEvento(evento: string, dettagli: Record<string, unknown> = {}): void {
  if (!abilitata) return;
  try {
    invoke('log_frontend_diagnostica', { evento, dettagli }).catch(() => {});
  } catch {
    // Fuori da Tauri (npx vite puro) — invoke() non esiste, nessun log
    // possibile in quel contesto di sviluppo, non bloccante.
  }
}

// Cronometra una funzione sync o async e logga automaticamente durata +
// esito — evita di ripetere il boilerplate di performance.now() ad ogni
// punto strumentato.
export async function cronometra<T>(
  evento: string,
  dettagliExtra: Record<string, unknown>,
  fn: () => T | Promise<T>
): Promise<T> {
  const inizio = performance.now();
  try {
    const risultato = await fn();
    logEvento(evento, {
      ...dettagliExtra,
      durata_ms: Math.round((performance.now() - inizio) * 100) / 100,
      esito: 'ok',
    });
    return risultato;
  } catch (e) {
    logEvento(evento, {
      ...dettagliExtra,
      durata_ms: Math.round((performance.now() - inizio) * 100) / 100,
      esito: 'errore',
      errore: String(e),
    });
    throw e;
  }
}

// Avvolge i metodi di un'istanza AWClient (aw-client) per cronometrare
// automaticamente OGNI chiamata — getEvents/query sono quelle che
// interessano di più (sono quelle che "interrogano il DB"), ma tutte e
// quattro sono avvolte per completezza. Applicato una volta sola subito
// dopo la creazione di ogni istanza (vedi util/awclient.ts), così
// nessun modulo deve essere toccato singolarmente: tutti passano da qui.
export function strumentaClient(client: any, etichettaClient: string): void {
  const metodi = ['getEvents', 'query', 'getBuckets', 'getBucketInfo'] as const;
  for (const nome of metodi) {
    const originale = client[nome];
    if (typeof originale !== 'function') continue;
    client[nome] = function (...args: unknown[]) {
      const bucketId = nome === 'getEvents' || nome === 'getBucketInfo' ? args[0] : undefined;
      return cronometra(
        'client_' + nome,
        { client: etichettaClient, bucket: bucketId },
        () => originale.apply(client, args)
      );
    };
  }
}

let avviato = false;

export function avvia(): void {
  if (avviato) return;
  avviato = true;

  // Info macchina/ambiente una tantum — utile per confrontare il
  // portatile col PC fisso senza doverselo far descrivere a mano, e per
  // verificare direttamente l'ipotesi "niente accelerazione GPU nel
  // WebView2 di questa macchina".
  let webglDisponibile = false;
  let gpuInfo: string | null = null;
  try {
    const canvas = document.createElement('canvas');
    const gl = (canvas.getContext('webgl') ||
      canvas.getContext('experimental-webgl')) as WebGLRenderingContext | null;
    webglDisponibile = !!gl;
    if (gl) {
      const debugInfo = gl.getExtension('WEBGL_debug_renderer_info');
      if (debugInfo) gpuInfo = gl.getParameter(debugInfo.UNMASKED_RENDERER_WEBGL);
    }
  } catch {
    // Ambiente senza canvas/WebGL utilizzabile — non bloccante, resta
    // semplicemente webglDisponibile: false.
  }
  logEvento('ambiente', {
    userAgent: navigator.userAgent,
    hardwareConcurrency: navigator.hardwareConcurrency,
    webglDisponibile,
    gpuInfo,
    devicePixelRatio: window.devicePixelRatio,
    schermo: `${screen.width}x${screen.height}`,
  });

  // Long task = il thread principale è rimasto bloccato più di 50ms —
  // è ESATTAMENTE il sintomo "il programma si blocca" segnalato
  // dall'utente, con durata reale invece di una sensazione soggettiva.
  try {
    const observer = new PerformanceObserver(list => {
      for (const entry of list.getEntries()) {
        logEvento('long_task', {
          durata_ms: Math.round(entry.duration),
          nome: entry.name,
          inizio_ms: Math.round(entry.startTime),
        });
      }
    });
    observer.observe({ entryTypes: ['longtask'] });
  } catch {
    // longtask non supportato in questo contesto — non bloccante.
  }

  // Uso memoria JS (solo Chromium/WebView2, performance.memory non è
  // standard) campionato ogni 5s — per vedere se cresce senza fermarsi
  // durante l'uso normale (perdita di memoria) invece di stabilizzarsi.
  const perfConMemoria = performance as Performance & {
    memory?: { usedJSHeapSize: number; totalJSHeapSize: number; jsHeapSizeLimit: number };
  };
  if (perfConMemoria.memory) {
    setInterval(() => {
      const m = perfConMemoria.memory!;
      logEvento('memoria_js', {
        usata_mb: Math.round(m.usedJSHeapSize / 1024 / 1024),
        totale_mb: Math.round(m.totalJSHeapSize / 1024 / 1024),
        limite_mb: Math.round(m.jsHeapSizeLimit / 1024 / 1024),
      });
    }, 5000);
  }
}

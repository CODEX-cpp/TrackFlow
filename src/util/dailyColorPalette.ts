// Assegnazione "primo arrivato, primo colore libero" dalla palette
// CATEGORY_COLOR_PALETTE (21 tonalità distinte a occhio, vedi
// hashColor.ts) per le corsie Timeline che non hanno un'icona reale a
// cui appoggiarsi (VPN, Claude, VS Code, Excel, VoiSpeed) — richiesta
// esplicita dell'utente dopo aver notato più file/client ADIACENTI con
// lo stesso colore hash (colorVarForName, solo 8 colori: con nomi
// illimitati le collisioni sono matematicamente inevitabili prima o
// poi), scambiabili a colpo d'occhio per la stessa entità pur essendo
// diversi. A differenza dell'hash, qui la stessa entità mantiene lo
// stesso colore finché ricompare nella stessa giornata, ma due entità
// diverse non condividono MAI un colore finché la palette non si
// esaurisce (21 file/client/chiamate diverse nello stesso giorno sulla
// stessa corsia — improbabile, gestito comunque con un fallback
// all'hash invece di un errore).
//
// Indipendente per corsia (lo stesso colore può essere "occupato" in
// VPN e libero in Excel nello stesso momento — ogni corsia ha la sua
// mappa) e si azzera ad ogni cambio di giorno (nuova giornata = tutti i
// colori di nuovo liberi per tutte le corsie), come richiesto
// esplicitamente.
import { CATEGORY_COLOR_PALETTE, colorVarForName } from './hashColor';

// L'assegnazione "primo colore libero" in ordine di ARRAY (indice 0, 1,
// 2...) sceglieva sempre tonalità VICINE tra loro per entità comparse
// vicine nel tempo — CATEGORY_COLOR_PALETTE è ordinata come una ruota
// cromatica continua (passo ~17° tra un indice e il successivo, vedi
// hashColor.ts), quindi il colore #0 e il #1 sono quasi lo stesso
// marrone. Bug/richiesta reale dell'utente: blocchi Timeline adiacenti
// nel tempo (aperti uno dopo l'altro) restavano troppo simili da
// distinguere a colpo d'occhio.
//
// Questo ordine risolve il problema scegliendo, ad ogni nuova
// assegnazione, il PUNTO DELLA RUOTA più lontano da tutti i colori già
// scelti finora (bisezione ricorsiva del gap circolare più grande) —
// standard per distribuire N punti su un cerchio nel modo più uniforme
// possibile ad ogni prefisso della sequenza, non solo alla fine. Con
// questo ordine, il 2° colore assegnato è sempre opposto al 1°, il 3°
// e il 4° bisecano le due metà risultanti, e così via — quindi anche
// solo 2-3 entità aperte vicine nel tempo prendono sempre colori ben
// contrapposti, non i primi due dell'array.
function ordineMassimamenteDistribuito(n: number): number[] {
  if (n <= 0) return [];
  const scelti: number[] = [0];
  while (scelti.length < n) {
    const ordinati = [...scelti].sort((a, b) => a - b);
    let gapMigliore = -1;
    let inizioMigliore = 0;
    for (let i = 0; i < ordinati.length; i++) {
      const a = ordinati[i];
      const b = ordinati[(i + 1) % ordinati.length];
      const dimensione = ((b - a + n) % n) || n;
      if (dimensione > gapMigliore) {
        gapMigliore = dimensione;
        inizioMigliore = a;
      }
    }
    scelti.push((inizioMigliore + Math.floor(gapMigliore / 2)) % n);
  }
  return scelti;
}

const ORDINE_ASSEGNAZIONE = ordineMassimamenteDistribuito(CATEGORY_COLOR_PALETTE.length);

let giornoCorrente = '';
let mappaCorrente = new Map<string, Map<string, string>>();

export function colorePerGiorno(laneKey: string, entityKey: string): string {
  const oggi = new Date().toISOString().slice(0, 10);
  if (oggi !== giornoCorrente) {
    giornoCorrente = oggi;
    mappaCorrente = new Map();
  }
  let assegnati = mappaCorrente.get(laneKey);
  if (!assegnati) {
    assegnati = new Map();
    mappaCorrente.set(laneKey, assegnati);
  }
  const esistente = assegnati.get(entityKey);
  if (esistente) return esistente;

  const usati = new Set(assegnati.values());
  const indiceLibero = ORDINE_ASSEGNAZIONE.find(i => !usati.has(CATEGORY_COLOR_PALETTE[i]));
  // Palette esaurita (21 entità diverse nella stessa corsia nello
  // stesso giorno) — fallback all'hash invece di ripetere un colore già
  // assegnato, comunque un caso limite molto improbabile.
  const colore = indiceLibero !== undefined ? CATEGORY_COLOR_PALETTE[indiceLibero] : colorVarForName(entityKey);
  assegnati.set(entityKey, colore);
  return colore;
}

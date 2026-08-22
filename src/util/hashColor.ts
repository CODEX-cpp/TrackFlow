// Deterministic "same name → same color" hash, used to color timeline
// blocks by client/app/domain without a fixed lookup table — a new
// client just gets *a* consistent color for free instead of needing
// someone to register it somewhere.
//
// Maps onto the --client-color-1..8 custom properties in theme.css
// (shared across light/dark themes on purpose, see theme.css).
const COLOR_COUNT = 8;

export function colorIndexForName(name: string): number {
  let hash = 0;
  for (let i = 0; i < name.length; i++) {
    hash = (hash << 5) - hash + name.charCodeAt(i);
    hash |= 0; // keep it a 32-bit int
  }
  return (Math.abs(hash) % COLOR_COUNT) + 1;
}

export function colorVarForName(name: string): string {
  return `var(--client-color-${colorIndexForName(name)})`;
}

// Stessa palette di --client-color-1..8 in theme.css, duplicata qui in
// esadecimale — sicuro perché quella palette è condivisa fra tema
// chiaro e scuro apposta (vedi il commento in cima al file), quindi non
// cambia mai sotto ai piedi di questa copia. Serve SOLO a calcolare la
// luminanza quando isLightColor() sotto riceve un riferimento
// "var(--client-color-N)" invece di un hex diretto — non deve mai
// diventare la fonte di verità per il colore stesso, quella resta
// sempre theme.css.
const CLIENT_COLOR_HEX: Record<number, string> = {
  1: '#d3a355',
  2: '#b8663f',
  3: '#8c3a2e',
  4: '#8b9a5c',
  5: '#d99a45',
  6: '#a14e3a',
  7: '#c9ae5a',
  8: '#7d6a56',
};

// true se `color` è abbastanza chiaro da richiedere testo NERO sopra
// invece del bianco di default — richiesto per i nomi app dentro i
// blocchi della Timeline: alcuni colori icona estratti automaticamente
// (es. verde/azzurro pastello, oro chiaro) sono troppo chiari per il
// testo bianco. Accetta sia un hex diretto (es. da iconColorForApp) sia
// un "var(--client-color-N)" prodotto da colorVarForName sopra —
// quest'ultimo risolto tramite CLIENT_COLOR_HEX invece di leggere il
// valore calcolato dal DOM (costerebbe una query per blocco ad ogni
// render, per un dato che qui è comunque statico). Formula YIQ standard
// (percezione della luminosità, pesa il verde più di rosso/blu) con
// soglia 150 su 255 — un filo più permissiva del classico 128, perché
// il testo bianco ha comunque un'ombra scura che aiuta la leggibilità
// anche su colori appena sopra la soglia "pura".
export function isLightColor(color: string): boolean {
  let hex = color;
  const varMatch = /^var\(--client-color-(\d)\)$/.exec(color);
  if (varMatch) hex = CLIENT_COLOR_HEX[Number(varMatch[1])] || '#000000';
  const m = /^#([0-9a-f]{6})$/i.exec(hex);
  if (!m) return false;
  const r = parseInt(m[1].slice(0, 2), 16);
  const g = parseInt(m[1].slice(2, 4), 16);
  const b = parseInt(m[1].slice(4, 6), 16);
  const yiq = (r * 299 + g * 587 + b * 114) / 1000;
  return yiq >= 150;
}

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

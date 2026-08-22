// Algoritmo "squarified treemap" (Bruls, Huizing, van Wijk) — impacca un
// elenco di valori in rettangoli il cui rapporto larghezza/altezza resta
// il più vicino possibile a un quadrato, mantenendo l'area di ciascuno
// proporzionale al suo valore. Scritto a mano (nessuna libreria tipo
// d3-hierarchy) per restare coerenti con tutte le altre visualizzazioni
// del progetto, tutte già scritte a mano. Usato dal modulo Home
// "Categorie (treemap)" — sia per il livello categorie sia, ricorsivo,
// per le app dentro ciascuna categoria.

export interface TreemapInput {
  key: string;
  value: number;
}

export interface TreemapRect extends TreemapInput {
  x: number;
  y: number;
  width: number;
  height: number;
}

// Rapporto d'aspetto peggiore (più lontano da 1) che risulterebbe
// mettendo `row` in una striscia di lunghezza (spessore) `length` — i
// valori in `row` sono già in "unità di area" (px²), non i valori
// grezzi originali, così il confronto ha senso in pixel reali. Basta
// guardare il valore più grande e quello più piccolo della riga: sono
// sempre loro a produrre il rapporto peggiore, non serve calcolarlo per
// ogni elemento (proprietà nota dell'algoritmo, non un'approssimazione).
function worst(row: TreemapInput[], length: number): number {
  if (row.length === 0) return Infinity;
  const sum = row.reduce((a, b) => a + b.value, 0);
  let max = -Infinity;
  let min = Infinity;
  for (const r of row) {
    if (r.value > max) max = r.value;
    if (r.value < min) min = r.value;
  }
  const lenSq = length * length;
  const sumSq = sum * sum;
  return Math.max((lenSq * max) / sumSq, sumSq / (lenSq * min));
}

export function squarify<T extends TreemapInput>(
  itemsIn: T[],
  x: number,
  y: number,
  width: number,
  height: number
): (T & { x: number; y: number; width: number; height: number })[] {
  const result: (T & { x: number; y: number; width: number; height: number })[] = [];
  const filtered = itemsIn.filter(i => i.value > 0);
  const totalValue = filtered.reduce((a, b) => a + b.value, 0);
  if (totalValue <= 0 || width <= 0 || height <= 0) return result;

  // Riscala ogni valore in "pixel² di area" UNA volta sola qui, non ad
  // ogni chiamata ricorsiva — dopo questo passaggio value == area, quindi
  // la somma dei valori di un sottoinsieme coincide sempre esattamente
  // con l'area del rettangolo che gli viene assegnato (mantenuto per
  // induzione dalla ricorsione sotto), senza dover riportare le unità in
  // scala ad ogni livello.
  const scale = (width * height) / totalValue;
  const data: T[] = filtered
    .map(i => ({ ...i, value: i.value * scale }))
    .sort((a, b) => b.value - a.value);

  function layoutRow(
    row: T[],
    rx: number,
    ry: number,
    rw: number,
    rh: number,
    horizontal: boolean,
    rowLength: number
  ) {
    const sum = row.reduce((a, b) => a + b.value, 0);
    let offset = 0;
    for (const item of row) {
      const fraction = sum > 0 ? item.value / sum : 0;
      if (horizontal) {
        const w = rw * fraction;
        result.push({ ...item, x: rx + offset, y: ry, width: w, height: rowLength });
        offset += w;
      } else {
        const h = rh * fraction;
        result.push({ ...item, x: rx, y: ry + offset, width: rowLength, height: h });
        offset += h;
      }
    }
  }

  function recurse(remaining: T[], rx: number, ry: number, rw: number, rh: number) {
    if (remaining.length === 0) return;
    if (remaining.length === 1) {
      result.push({ ...remaining[0], x: rx, y: ry, width: rw, height: rh });
      return;
    }
    // La striscia corrente riempie per intero il lato PIÙ CORTO del
    // rettangolo rimasto (orizzontale = riempie la larghezza, crescendo
    // in spessore verso il basso — scelto quando il box è più alto che
    // largo) — è la scelta che, ad ogni passo, avvicina di più i
    // rettangoli risultanti a un quadrato. Bug reale trovato dal vivo
    // (verificato isolando la funzione in uno script Node prima di
    // toccare l'app): la condizione e il lato passato a worst()/usato
    // per calcolare lo spessore erano invertiti, producendo altezze
    // negative e riquadri completamente fuori scala.
    const horizontal = rw <= rh;
    const shortSide = horizontal ? rw : rh;

    let row: T[] = [remaining[0]];
    let i = 1;
    while (i < remaining.length) {
      const candidateRow = row.concat(remaining[i]);
      // Finché aggiungere il prossimo elemento migliora (o non
      // peggiora) il rapporto d'aspetto peggiore della riga, lo si
      // aggiunge — appena peggiora, la riga corrente è quella definitiva.
      if (worst(candidateRow, shortSide) <= worst(row, shortSide)) {
        row = candidateRow;
        i++;
      } else {
        break;
      }
    }

    const rowSum = row.reduce((a, b) => a + b.value, 0);
    const rowLength = horizontal ? rowSum / rw : rowSum / rh;
    layoutRow(row, rx, ry, rw, rh, horizontal, rowLength);

    const rest = remaining.slice(row.length);
    if (rest.length === 0) return;
    if (horizontal) {
      recurse(rest, rx, ry + rowLength, rw, rh - rowLength);
    } else {
      recurse(rest, rx + rowLength, ry, rw - rowLength, rh);
    }
  }

  recurse(data, x, y, width, height);
  return result;
}

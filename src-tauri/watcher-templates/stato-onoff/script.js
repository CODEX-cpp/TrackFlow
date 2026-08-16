// Modello "Stato acceso/spento": legge l'ultimo evento del bucket
// indicato (bucket_id nella query string, impostato da SelectableVisualization.vue)
// e mostra un pallino colorato in base al primo campo "acceso/spento"
// trovato nei dati - non serve sapere in anticipo come si chiama quel
// campo. Non si puo' dare per scontato che uno script scritto a mano
// produca sempre un booleano JSON vero (true/false): a seconda del
// linguaggio/librerie usate potrebbe arrivare come numero (0/1) o come
// stringa ("si"/"no", "acceso"/"spento", "on"/"off"...) - interpretaStato
// riconosce tutte queste forme comuni.
(function () {
  const PAROLE_VERE = new Set([
    'true', 'si', 'sì', 'yes', 'on', 'attivo', 'acceso', 'aperto',
    'connesso', 'running', 'avviato', 'presente', 'ok',
  ]);
  const PAROLE_FALSE = new Set([
    'false', 'no', 'off', 'nonattivo', 'spento', 'chiuso',
    'disconnesso', 'stopped', 'fermo', 'assente',
  ]);

  // Ritorna true/false se il valore e' riconoscibile come stato
  // acceso/spento in una qualunque forma comune, altrimenti null (non
  // riconosciuto, va cercato un altro campo).
  function interpretaStato(valore) {
    if (typeof valore === 'boolean') return valore;
    // Solo 0/1 esatti, non un numero qualunque diverso da zero - un
    // "minuti_attivi": 12 non è uno stato acceso/spento, è un conteggio
    // (quello lo intercetta piuttosto "Valore numerico in evidenza").
    if (valore === 0) return false;
    if (valore === 1) return true;
    if (typeof valore === 'string') {
      const normalizzato = valore.trim().toLowerCase().replace(/\s+/g, '');
      if (PAROLE_VERE.has(normalizzato)) return true;
      if (PAROLE_FALSE.has(normalizzato)) return false;
    }
    return null;
  }

  const params = new URLSearchParams(window.location.search);
  const bucketId = params.get('bucket_id');
  const pallino = document.getElementById('pallino');
  const etichetta = document.getElementById('etichetta');

  async function aggiorna() {
    if (!bucketId) {
      etichetta.textContent = 'Nessun bucket collegato.';
      return;
    }
    try {
      const query = [
        `events = sort_by_timestamp(flood(query_bucket("${bucketId}")));`,
        'RETURN = events;',
      ];
      const risposta = await fetch('/api/0/query/', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          query,
          timeperiods: ['2000-01-01T00:00:00.000Z/2100-01-01T00:00:00.000Z'],
        }),
      });
      if (!risposta.ok) throw new Error('risposta non ok');
      const dataPerPeriodo = await risposta.json();
      const eventi = (dataPerPeriodo && dataPerPeriodo[0]) || [];
      const ultimo = eventi[eventi.length - 1];
      const dati = ultimo ? ultimo.data : null;

      if (!dati) {
        pallino.className = 'pallino';
        etichetta.textContent = 'In attesa dei primi dati...';
        return;
      }

      let chiaveTrovata = null;
      let statoTrovato = null;
      for (const chiave of Object.keys(dati)) {
        const stato = interpretaStato(dati[chiave]);
        if (stato !== null) {
          chiaveTrovata = chiave;
          statoTrovato = stato;
          break;
        }
      }

      if (chiaveTrovata === null) {
        pallino.className = 'pallino';
        etichetta.textContent = 'Nessun valore acceso/spento trovato nei dati.';
        return;
      }

      pallino.className = 'pallino ' + (statoTrovato ? 'acceso' : 'spento');
      etichetta.textContent = chiaveTrovata + ': ' + (statoTrovato ? 'Attivo' : 'Non attivo');
    } catch (e) {
      etichetta.textContent = 'Errore nel caricamento dei dati.';
    }
  }

  aggiorna();
  setInterval(aggiorna, 30000);
})();

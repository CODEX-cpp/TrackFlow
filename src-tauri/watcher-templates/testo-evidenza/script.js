// Modello "Testo in evidenza": legge l'ultimo evento del bucket
// (bucket_id nella query string) e mostra in grande il primo campo di
// tipo testo (non vuoto) trovato nei dati - non serve sapere in
// anticipo come si chiama quel campo.
(function () {
  const params = new URLSearchParams(window.location.search);
  const bucketId = params.get('bucket_id');
  const testo = document.getElementById('testo');
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
        testo.textContent = '-';
        etichetta.textContent = 'In attesa dei primi dati...';
        return;
      }

      const chiave = Object.keys(dati).find((k) => typeof dati[k] === 'string' && dati[k].trim() !== '');
      if (!chiave) {
        testo.textContent = '-';
        etichetta.textContent = 'Nessun valore testuale trovato nei dati.';
        return;
      }

      testo.textContent = dati[chiave];
      etichetta.textContent = chiave;
    } catch (e) {
      etichetta.textContent = 'Errore nel caricamento dei dati.';
    }
  }

  aggiorna();
  setInterval(aggiorna, 30000);
})();

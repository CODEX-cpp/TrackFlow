// Modello "Barra di avanzamento": legge l'ultimo evento del bucket
// (bucket_id nella query string) e mostra come barra il primo campo
// numerico trovato nei dati, considerandolo una percentuale (valore
// tra 0 e 100 - se il tuo watcher scrive un numero in un range
// diverso, questo modello non e' quello giusto, meglio "Valore
// numerico in evidenza").
(function () {
  const params = new URLSearchParams(window.location.search);
  const bucketId = params.get('bucket_id');
  const etichetta = document.getElementById('etichetta');
  const riempimento = document.getElementById('riempimento');

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
        etichetta.textContent = 'In attesa dei primi dati...';
        riempimento.style.width = '0%';
        return;
      }

      const chiave = Object.keys(dati).find((k) => typeof dati[k] === 'number');
      if (!chiave) {
        etichetta.textContent = 'Nessun valore numerico trovato nei dati.';
        riempimento.style.width = '0%';
        return;
      }

      const percentuale = Math.max(0, Math.min(100, dati[chiave]));
      riempimento.style.width = percentuale + '%';
      etichetta.textContent = chiave + ': ' + percentuale + '%';
    } catch (e) {
      etichetta.textContent = 'Errore nel caricamento dei dati.';
    }
  }

  aggiorna();
  setInterval(aggiorna, 30000);
})();

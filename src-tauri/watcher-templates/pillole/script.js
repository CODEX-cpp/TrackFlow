// Modello "Elenco a pillole": legge l'ultimo evento del bucket
// (bucket_id nella query string) e mostra TUTTI i campi dei dati come
// piccole pillole "chiave: valore" - alternativa più compatta e
// colorata rispetto all'elenco chiave/valore predefinito, utile
// quando un watcher scrive più campi insieme.
(function () {
  const params = new URLSearchParams(window.location.search);
  const bucketId = params.get('bucket_id');
  const contenitore = document.getElementById('contenitore');

  function mostraMessaggio(testo) {
    contenitore.innerHTML = '';
    const div = document.createElement('div');
    div.className = 'vuoto';
    div.textContent = testo;
    contenitore.appendChild(div);
  }

  async function aggiorna() {
    if (!bucketId) {
      mostraMessaggio('Nessun bucket collegato.');
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

      if (!dati || Object.keys(dati).length === 0) {
        mostraMessaggio('In attesa dei primi dati...');
        return;
      }

      contenitore.innerHTML = '';
      for (const chiave of Object.keys(dati)) {
        const pillola = document.createElement('div');
        pillola.className = 'pillola';

        const spanChiave = document.createElement('span');
        spanChiave.className = 'pillola-chiave';
        spanChiave.textContent = chiave + ':';

        const spanValore = document.createElement('span');
        spanValore.className = 'pillola-valore';
        spanValore.textContent = String(dati[chiave]);

        pillola.appendChild(spanChiave);
        pillola.appendChild(spanValore);
        contenitore.appendChild(pillola);
      }
    } catch (e) {
      mostraMessaggio('Errore nel caricamento dei dati.');
    }
  }

  aggiorna();
  setInterval(aggiorna, 30000);
})();

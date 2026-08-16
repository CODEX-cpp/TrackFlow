// Modello "Ultimo aggiornamento": non guarda i dati del watcher, solo
// QUANDO e' arrivato l'ultimo evento - utile come indicatore generico
// "e' ancora vivo?" per qualunque watcher, senza dover conoscere i
// suoi campi. Pallino rosso se l'ultimo evento e' piu' vecchio di 15
// minuti (soglia generica, non legata all'intervallo reale del
// watcher, che questa pagina statica non conosce).
(function () {
  const SOGLIA_VECCHIO_MS = 15 * 60 * 1000;
  const params = new URLSearchParams(window.location.search);
  const bucketId = params.get('bucket_id');
  const pallino = document.getElementById('pallino');
  const testo = document.getElementById('testo');

  function formattaRelativo(dataEvento) {
    const secondi = Math.max(0, Math.round((Date.now() - dataEvento.getTime()) / 1000));
    if (secondi < 60) return 'pochi secondi fa';
    const minuti = Math.round(secondi / 60);
    if (minuti < 60) return minuti + (minuti === 1 ? ' minuto fa' : ' minuti fa');
    const ore = Math.round(minuti / 60);
    if (ore < 24) return ore + (ore === 1 ? ' ora fa' : ' ore fa');
    const giorni = Math.round(ore / 24);
    return giorni + (giorni === 1 ? ' giorno fa' : ' giorni fa');
  }

  async function aggiorna() {
    if (!bucketId) {
      testo.textContent = 'Nessun bucket collegato.';
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

      if (!ultimo || !ultimo.timestamp) {
        testo.textContent = 'In attesa dei primi dati...';
        pallino.className = 'pallino';
        return;
      }

      const dataEvento = new Date(ultimo.timestamp);
      const vecchio = Date.now() - dataEvento.getTime() > SOGLIA_VECCHIO_MS;
      pallino.className = 'pallino' + (vecchio ? ' vecchio' : '');
      testo.textContent = 'Ultimo aggiornamento: ' + formattaRelativo(dataEvento);
    } catch (e) {
      testo.textContent = 'Errore nel caricamento dei dati.';
    }
  }

  aggiorna();
  setInterval(aggiorna, 30000);
})();

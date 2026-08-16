// Modello "Classifica valori più frequenti (oggi)": individua il primo
// campo testuale nell'evento più recente (es. "file", "cliente",
// "progetto"...), raggruppa TUTTI gli eventi di oggi per quel campo e
// mostra i 5 valori con più tempo totale - utile per un watcher che
// scrive un nome/etichetta di cui vuoi sapere qual è il più frequente
// (es. quale file Excel aperto di più), non solo l'ultimo visto.
(function () {
  function periodoOggi() {
    const ora = new Date();
    const inizio = new Date(ora.getFullYear(), ora.getMonth(), ora.getDate(), 0, 0, 0, 0);
    return inizio.toISOString() + '/' + ora.toISOString();
  }

  function formattaDurata(secondi) {
    const s = Math.round(secondi);
    const ore = Math.floor(s / 3600);
    const minuti = Math.floor((s % 3600) / 60);
    if (ore > 0) return ore + 'h ' + minuti + 'm';
    if (minuti > 0) return minuti + 'm';
    return s + 's';
  }

  const params = new URLSearchParams(window.location.search);
  const bucketId = params.get('bucket_id');
  const lista = document.getElementById('lista');

  function mostraMessaggio(testo) {
    lista.innerHTML = '';
    const div = document.createElement('div');
    div.className = 'vuoto';
    div.textContent = testo;
    lista.appendChild(div);
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
        body: JSON.stringify({ query, timeperiods: [periodoOggi()] }),
      });
      if (!risposta.ok) throw new Error('risposta non ok');
      const dataPerPeriodo = await risposta.json();
      const eventi = (dataPerPeriodo && dataPerPeriodo[0]) || [];

      if (eventi.length === 0) {
        mostraMessaggio('Nessun dato per oggi.');
        return;
      }

      const ultimo = eventi[eventi.length - 1];
      const chiave = Object.keys(ultimo.data || {}).find(
        (k) => typeof ultimo.data[k] === 'string' && ultimo.data[k].trim() !== ''
      );
      if (!chiave) {
        mostraMessaggio('Nessun valore testuale da raggruppare nei dati.');
        return;
      }

      const totali = new Map();
      for (const ev of eventi) {
        const valore = ev.data ? ev.data[chiave] : undefined;
        if (typeof valore !== 'string' || valore.trim() === '') continue;
        totali.set(valore, (totali.get(valore) || 0) + (ev.duration || 0));
      }

      const classificati = [...totali.entries()].sort((a, b) => b[1] - a[1]).slice(0, 5);
      if (classificati.length === 0) {
        mostraMessaggio('Nessun dato da raggruppare.');
        return;
      }

      lista.innerHTML = '';
      for (const [valore, tempo] of classificati) {
        const riga = document.createElement('div');
        riga.className = 'riga';
        const nome = document.createElement('span');
        nome.className = 'riga-nome';
        nome.textContent = valore;
        const dur = document.createElement('span');
        dur.className = 'riga-durata';
        dur.textContent = formattaDurata(tempo);
        riga.appendChild(nome);
        riga.appendChild(dur);
        lista.appendChild(riga);
      }
    } catch (e) {
      mostraMessaggio('Errore nel caricamento dei dati.');
    }
  }

  aggiorna();
  setInterval(aggiorna, 30000);
})();

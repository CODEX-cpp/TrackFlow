// Modello "Tempo totale acceso (oggi)": individua il primo campo
// "acceso/spento" nell'evento più recente (stessa interpretazione
// tollerante di "Stato acceso/spento": booleano, 0/1, o parole comuni
// come "on"/"connesso"/"attivo"), poi somma le durate di TUTTI gli
// eventi di oggi in cui quel campo risultava vero - utile per un
// watcher che manda un segnale acceso/spento (es. VPN connessa) di cui
// vuoi sapere il totale, non solo lo stato attuale.
(function () {
  const PAROLE_VERE = new Set([
    'true', 'si', 'sì', 'yes', 'on', 'attivo', 'acceso', 'aperto',
    'connesso', 'running', 'avviato', 'presente', 'ok',
  ]);
  const PAROLE_FALSE = new Set([
    'false', 'no', 'off', 'nonattivo', 'spento', 'chiuso',
    'disconnesso', 'stopped', 'fermo', 'assente',
  ]);

  function interpretaStato(valore) {
    if (typeof valore === 'boolean') return valore;
    if (valore === 0) return false;
    if (valore === 1) return true;
    if (typeof valore === 'string') {
      const normalizzato = valore.trim().toLowerCase().replace(/\s+/g, '');
      if (PAROLE_VERE.has(normalizzato)) return true;
      if (PAROLE_FALSE.has(normalizzato)) return false;
    }
    return null;
  }

  // Giorno solare locale (mezzanotte -> adesso) - semplificazione
  // deliberata: questa pagina statica non conosce l'orario "inizio
  // giornata" personalizzato di TrackFlow (Impostazioni), userebbe
  // un'altra chiamata solo per quello.
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
  const durata = document.getElementById('durata');
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
        body: JSON.stringify({ query, timeperiods: [periodoOggi()] }),
      });
      if (!risposta.ok) throw new Error('risposta non ok');
      const dataPerPeriodo = await risposta.json();
      const eventi = (dataPerPeriodo && dataPerPeriodo[0]) || [];

      if (eventi.length === 0) {
        durata.textContent = '-';
        etichetta.textContent = 'Nessun dato per oggi.';
        return;
      }

      const ultimo = eventi[eventi.length - 1];
      let chiave = null;
      for (const k of Object.keys(ultimo.data || {})) {
        if (interpretaStato(ultimo.data[k]) !== null) {
          chiave = k;
          break;
        }
      }

      if (!chiave) {
        durata.textContent = '-';
        etichetta.textContent = 'Nessun valore acceso/spento trovato nei dati.';
        return;
      }

      let totale = 0;
      for (const ev of eventi) {
        if (interpretaStato(ev.data ? ev.data[chiave] : undefined) === true) {
          totale += ev.duration || 0;
        }
      }

      durata.textContent = formattaDurata(totale);
      etichetta.textContent = chiave + ' oggi';
    } catch (e) {
      etichetta.textContent = 'Errore nel caricamento dei dati.';
    }
  }

  aggiorna();
  setInterval(aggiorna, 30000);
})();

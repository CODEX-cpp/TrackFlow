import { defineStore } from 'pinia';

// Orologio reattivo, aggiornato ogni minuto — esiste SOLO per essere
// letto dentro i computed che chiamano get_today_with_offset() (Topbar,
// HomeModulesSection, homeActivityRangeMixin): quella funzione legge
// l'ora reale (moment()), che Vue non ha modo di tracciare come
// dipendenza — un computed che la usa resta "congelato" al valore della
// prima valutazione finché nessun'ALTRA dipendenza reattiva (i parametri
// della route, l'impostazione "inizio giornata") cambia, anche se nel
// frattempo è passata mezzanotte (o l'orario di inizio giornata) per
// davvero. Bug reale segnalato dall'utente: con l'app rimasta aperta a
// cavallo del cambio giorno, Home/Timeline/Moduli mostravano ancora i
// dati di ieri mentre il selettore della Topbar indicava già "oggi" —
// sistemato cambiando giorno a mano e tornando indietro (che invalidava
// la cache toccando i parametri della route), ma sarebbe potuto
// ripresentarsi alla prossima mezzanotte. Leggere `tick` dentro quei
// computed (anche solo per il suo effetto collaterale di registrare la
// dipendenza) li fa ricalcolare da soli entro un minuto dal cambio
// giorno, invece di restare bloccati fino al prossimo tocco manuale.
export const useClockStore = defineStore('clock', {
  state: () => ({
    tick: Date.now(),
    _avviato: false,
  }),
  actions: {
    avvia() {
      if (this._avviato) return;
      this._avviato = true;
      setInterval(() => {
        this.tick = Date.now();
      }, 60_000);
    },
  },
});

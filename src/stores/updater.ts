import { defineStore } from 'pinia';
import { invoke } from '@tauri-apps/api/core';

// Stato dell'auto-aggiornamento — vedi UpdatePopup.vue per i tre stati
// mostrati e src-tauri/src/updater.rs per la logica vera di controllo/
// download/verifica/installazione (Task #3). App.vue chiama
// controllaAggiornamenti() una volta ad ogni avvio: se autoUpdateEnabled
// è acceso scarica subito, altrimenti resta in 'available' finché
// l'utente non clicca il popup.
type UpdateStatus = 'idle' | 'available' | 'downloading' | 'ready' | 'error';

interface InfoAggiornamento {
  versione: string;
  url_asset: string;
  url_firma: string;
}

export const useUpdaterStore = defineStore('updater', {
  state: () => ({
    status: 'idle' as UpdateStatus,
    version: null as string | null,
    // Servono a scarica_e_prepara_aggiornamento — tenute qui invece che
    // ririchieste a GitHub una seconda volta al momento del download.
    pendingInfo: null as InfoAggiornamento | null,
    errorMessage: '',
  }),
  actions: {
    // Chiamato una volta da App.vue ad ogni avvio dell'app (non un
    // interval — un controllo al lancio è sufficiente, coerente con la
    // richiesta dell'utente "l'app deve controllare ad ogni avvio").
    async controllaAggiornamenti(autoUpdateEnabled: boolean) {
      let info: InfoAggiornamento | null = null;
      try {
        info = await invoke<InfoAggiornamento | null>('controlla_aggiornamento');
      } catch (e) {
        // Fuori da Tauri (dev server puro nel browser), o rete assente —
        // nessun aggiornamento da segnalare, non è un errore da mostrare
        // all'utente ad ogni avvio.
        console.warn('Controllo aggiornamenti non riuscito:', e);
        return;
      }
      if (!info) return;

      this.pendingInfo = info;
      if (autoUpdateEnabled) {
        await this.startDownload();
      } else {
        this.status = 'available';
        this.version = info.versione;
      }
    },
    // Click su "Update disponibile" (se gli aggiornamenti automatici
    // sono spenti) oppure chiamato subito dopo il controllo se sono
    // accesi.
    async startDownload() {
      if (!this.pendingInfo) return;
      this.status = 'downloading';
      this.version = this.pendingInfo.versione;
      try {
        await invoke('scarica_e_prepara_aggiornamento', {
          versione: this.pendingInfo.versione,
          urlAsset: this.pendingInfo.url_asset,
          urlFirma: this.pendingInfo.url_firma,
        });
        this.status = 'ready';
      } catch (e) {
        console.error('Download aggiornamento fallito:', e);
        this.status = 'error';
        this.errorMessage = String(e);
      }
    },
    // Click su "Riavvia per aggiornare" — sposta current.txt sulla
    // versione già scaricata+verificata e rilancia via launcher.exe.
    async installaERiavvia() {
      if (!this.version) return;
      try {
        await invoke('installa_aggiornamento_e_riavvia', { versione: this.version });
      } catch (e) {
        console.error('Installazione aggiornamento fallita:', e);
        this.status = 'error';
        this.errorMessage = String(e);
      }
    },
    reset() {
      this.status = 'idle';
      this.version = null;
      this.pendingInfo = null;
      this.errorMessage = '';
    },
  },
});

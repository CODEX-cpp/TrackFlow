// Ponte tra il popup di dettaglio della Timeline (TimelineBlockDetailModal)
// e il widget della chat AI (AiChatWidget) — due componenti senza relazione
// diretta parent/child (stesso motivo di stores/timelineHighlight.ts), serve
// uno store per passare "questa attività va allegata al prossimo messaggio".
//
// Stile "rispondi a un messaggio" di WhatsApp: `label` è quello che l'utente
// VEDE sopra il campo di scrittura (es. "Firefox 12:38 – 13:49"), `extra` è
// il testo con i dati veri (elenco attività/orari) che viene anteposto in
// automatico al messaggio digitato SOLO al momento dell'invio — non fa mai
// parte della bolla mostrata in chat, l'utente continua a vedere solo quello
// che ha scritto lui.
import { defineStore } from 'pinia';

export interface ContestoChatAi {
  label: string;
  extra: string;
}

export const useAiChatContextStore = defineStore('aiChatContext', {
  state: () => ({
    contesto: null as ContestoChatAi | null,
  }),
  actions: {
    imposta(contesto: ContestoChatAi) {
      this.contesto = contesto;
    },
    pulisci() {
      this.contesto = null;
    },
  },
});

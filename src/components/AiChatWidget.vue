<template lang="pug">
div.ai-chat-widget
  button.ai-chat-fab(v-if="!open" type="button" @click="apri" :title="$t('aiChat.open')")
    icon(name="comments")
  div.ai-chat-panel(
    v-if="open"
    :style="dimensioniPersonalizzate ? { width: dimensioniPersonalizzate.width + 'px', height: dimensioniPersonalizzate.height + 'px' } : null"
  )
    //- Zone di trascinamento per ridimensionare il pannello — solo in
    //- alto e a sinistra (non destra/basso), perché il pannello resta
    //- ancorato in basso a destra dello schermo (right/bottom in CSS):
    //- crescere da lì significa "spingere" il bordo opposto verso l'alto
    //- e verso sinistra, richiesta esplicita dell'utente.
    div.ai-chat-resize-top(@mousedown="iniziaRidimensionamento('top', $event)")
    div.ai-chat-resize-left(@mousedown="iniziaRidimensionamento('left', $event)")
    div.ai-chat-resize-corner(@mousedown="iniziaRidimensionamento('corner', $event)")
    div.ai-chat-header
      span.ai-chat-title {{ $t('aiChat.title') }}
      div.ai-chat-header-actions
        button.ai-chat-icon-btn(type="button" @click="nuovaConversazione" :title="$t('aiChat.newConversation')")
          icon(name="sync")
        button.ai-chat-icon-btn(type="button" @click="chiudi" :title="$t('aiChat.close')")
          icon(name="times")

    div.ai-chat-messages.themed-scroll(ref="messagesEl")
      //- Controllo fatto PRIMA di poter scrivere, non più dopo l'invio —
      //- bug segnalato dall'utente: prima si scopriva la chiave API
      //- mancante solo dopo aver già mandato il primo messaggio, con un
      //- errore che sembrava un fallimento della richiesta invece che una
      //- configurazione mancante.
      div.ai-chat-empty(v-if="apiConfigurata === false")
        | {{ $t('aiChat.apiKeyMissing') }}
        div.ai-chat-empty-action(@click="apriImpostazioniAgente") {{ $t('aiChat.apiKeyMissingAction') }}
      div.ai-chat-empty(v-else-if="messaggi.length === 0") {{ $t('aiChat.emptyHint') }}
      div.ai-chat-bubble(v-for="(m, i) in messaggi" :key="i" :class="'ai-chat-bubble-' + m.ruolo")
        //- Stessa etichetta mostrata nella barra di composizione prima
        //- dell'invio (vedi ai-chat-context-row sotto), ma qui resta
        //- attaccata per sempre al messaggio mandato — effetto "citazione"
        //- di WhatsApp, richiesta esplicita dell'utente.
        div.ai-chat-bubble-quote(v-if="m.contestoLabel") {{ m.contestoLabel }}
        //- Mostra quali dati sono stati consultati prima di rispondere —
        //- richiesta esplicita dell'utente: non deve sembrare un botta e
        //- risposta a memoria.
        div.ai-chat-tools(v-if="m.strumenti && m.strumenti.length")
          | 🔍 {{ $t('aiChat.toolsUsedPrefix') }} {{ etichettaStrumenti(m.strumenti) }}
        //- Le risposte dell'assistente sono spesso Markdown (grassetto,
        //- tabelle, elenchi) — renderizzate qui invece di mostrare i
        //- simboli letterali (**, |, -). I messaggi dell'utente restano
        //- testo semplice: non c'è motivo di interpretarli come Markdown.
        div.ai-chat-markdown(v-if="m.ruolo === 'assistant'" v-html="renderizzaMarkdown(m.testo)")
        div.ai-chat-plain(v-else) {{ m.testo }}
      div.ai-chat-bubble.ai-chat-bubble-assistant.ai-chat-bubble-loading(v-if="inviando")
        | {{ $t('aiChat.thinking') }}

    div.ai-chat-alert(v-if="errore") {{ errore }}

    //- Wrapper unico attorno a citazione + campo di scrittura — quando la
    //- citazione è presente devono sembrare un solo "cubo" con due zone
    //- di colore diverso (bordo condiviso, nessuno stacco), come
    //- rispondere a un messaggio su WhatsApp, non due riquadri separati.
    //- Richiesta estetica esplicita dell'utente.
    div.ai-chat-compose
      //- Stile "rispondi a un messaggio" di WhatsApp: l'attività allegata
      //- da un blocco della Timeline (vedi apriConversazioneAi() in
      //- TimelineBlockDetailModal.vue) resta visibile qui sopra il campo
      //- finché non viene mandata (o tolta a mano con la ×) — il testo
      //- coi dati veri non si vede mai, solo questa etichetta riassuntiva.
      div.ai-chat-context-row(v-if="contestoAttivo")
        div.ai-chat-context-label {{ contestoAttivo.label }}
        button.ai-chat-context-remove(type="button" @click="aiChatContextStore.pulisci()" :title="$t('aiChat.removeContext')")
          icon(name="times")

      div.ai-chat-input-row(:class="{ 'ai-chat-input-row--attached': contestoAttivo }")
        textarea.ai-chat-input.themed-scroll(
          ref="inputEl"
          :class="{ 'ai-chat-input--attached': contestoAttivo }"
          v-model="bozza"
          :placeholder="$t('aiChat.placeholder')"
          :disabled="inviando || apiConfigurata === false"
          @keydown.enter.exact.prevent="invia"
          @input="adattaAltezzaInput"
        )
        button.ai-chat-send-btn(type="button" @click="invia" :disabled="inviando || !bozza.trim() || apiConfigurata === false")
          icon(name="paper-plane")
</template>

<script lang="ts">
import 'vue-awesome/icons/comments';
import 'vue-awesome/icons/sync';
import 'vue-awesome/icons/times';
import 'vue-awesome/icons/paper-plane';

import { invoke } from '@tauri-apps/api/core';
import { marked } from 'marked';
import DOMPurify from 'dompurify';
import { useAiChatContextStore } from '~/stores/aiChatContext';

marked.setOptions({ breaks: true });

const DIMENSIONI_STORAGE_KEY = 'aiChat.dimensioniPannello';
const DIMENSIONE_MIN_WIDTH = 300;
const DIMENSIONE_MIN_HEIGHT = 320;
// Costo API notato insolitamente alto dall'utente (vedi agent.rs): una
// cronologia tenuta viva a lungo tra un uso sporadico e l'altro pesa
// sempre di più ad ogni ripresa, senza che la cache di 5 minuti di
// Anthropic possa aiutare (troppo tempo nel mezzo). L'azzeramento
// automatico dopo un po' di INATTIVITÀ del popup (non del semplice
// scrivere: solo mandare/ricevere messaggi conta, vedi riavviaTimerInattivita())
// tiene la cronologia corta senza dover ricordarsi di premere il tasto
// reset a mano.
const TIMEOUT_INATTIVITA_MS = 5 * 60 * 1000;

interface Messaggio {
  ruolo: 'user' | 'assistant';
  testo: string;
  strumenti?: string[];
  // Etichetta dell'attività allegata (vedi contestoAttivo/store) —
  // presente solo sui messaggi utente mandati con un'attività citata,
  // resta visibile in cronologia come una citazione WhatsApp (effetto
  // permanente, a differenza della barra di composizione che sparisce
  // dopo l'invio).
  contestoLabel?: string;
}

interface RispostaAgente {
  testo: string;
  strumenti_usati: string[];
}

export default {
  name: 'AiChatWidget',
  data() {
    return {
      open: false,
      messaggi: [] as Messaggio[],
      bozza: '',
      inviando: false,
      errore: '',
      // null finché non è stata ancora controllata (primo apri()) — non
      // false di default, altrimenti lampeggerebbe per un istante l'avviso
      // "chiave mancante" anche quando è configurata, prima che la
      // risposta di invoke() arrivi.
      apiConfigurata: null as boolean | null,
      // null = ancora alla dimensione di default (clamp() in CSS) — una
      // volta che l'utente trascina un bordo anche solo una volta, questa
      // diventa la dimensione fissa in px, sostituendo il clamp. Letta da
      // localStorage qui sotto (vedi leggiDimensioniSalvate()) così
      // sopravvive anche a chiusura/riapertura dell'APP, non solo del
      // pannello — richiesta esplicita dell'utente dopo aver notato che
      // riavviando l'app la dimensione tornava a quella di default.
      dimensioniPersonalizzate: this.leggiDimensioniSalvate(),
      ridimensionamento: null as
        | { bordo: 'top' | 'left' | 'corner'; startX: number; startY: number; startWidth: number; startHeight: number }
        | null,
      aiChatContextStore: useAiChatContextStore(),
      // Id del setTimeout dell'auto-chiusura per inattività — non
      // reattivo apposta (usare `null as number|null` in `data()` andrebbe
      // comunque bene, ma non serve che Vue lo osservi: è puro stato
      // interno di gestione timer).
      timerInattivita: null as ReturnType<typeof setTimeout> | null,
    };
  },
  computed: {
    contestoAttivo() {
      return this.aiChatContextStore.contesto;
    },
  },
  watch: {
    // Un'attività allegata dal popup di dettaglio della Timeline apre la
    // chat da sola (se non era già aperta) — l'utente ha già cliccato
    // "chiedi all'AI" lì, non serve un secondo click qui.
    contestoAttivo(nuovo) {
      if (!nuovo) return;
      this.open = true;
      this.riavviaTimerInattivita();
      this.verificaConfigurazione();
      this.$nextTick(() => {
        const el = this.$refs.inputEl as HTMLTextAreaElement | undefined;
        if (el) el.focus();
      });
    },
  },
  beforeDestroy() {
    window.removeEventListener('mousemove', this.aggiornaRidimensionamento);
    window.removeEventListener('mouseup', this.terminaRidimensionamento);
    if (this.timerInattivita) clearTimeout(this.timerInattivita);
  },
  methods: {
    async apri() {
      this.open = true;
      this.riavviaTimerInattivita();
      await this.verificaConfigurazione();
    },
    // Chiusura volontaria (pulsante ×) — azzera la cronologia insieme al
    // pannello, richiesta esplicita dell'utente: la stessa "chat" non
    // deve poter ripartire più tardi trascinandosi dietro tutta la
    // conversazione precedente.
    chiudi() {
      this.open = false;
      this.nuovaConversazione();
    },
    // Nessuna interazione (invio/ricezione messaggio) per
    // TIMEOUT_INATTIVITA_MS mentre il pannello è aperto — stesso
    // trattamento della chiusura volontaria: chiude E azzera, così
    // riaprendo più tardi non si riparte da una cronologia vecchia di
    // minuti/ore che peserebbe per intero sulla prossima chiamata (la
    // cache di Anthropic è comunque scaduta a quel punto).
    chiudiPerInattivita() {
      this.timerInattivita = null;
      this.chiudi();
    },
    riavviaTimerInattivita() {
      if (this.timerInattivita) clearTimeout(this.timerInattivita);
      this.timerInattivita = this.open
        ? setTimeout(this.chiudiPerInattivita, TIMEOUT_INATTIVITA_MS)
        : null;
    },
    iniziaRidimensionamento(bordo: 'top' | 'left' | 'corner', event: MouseEvent) {
      const panel = (this.$el as HTMLElement).querySelector('.ai-chat-panel') as HTMLElement | null;
      if (!panel) return;
      const rect = panel.getBoundingClientRect();
      this.ridimensionamento = {
        bordo,
        startX: event.clientX,
        startY: event.clientY,
        startWidth: rect.width,
        startHeight: rect.height,
      };
      window.addEventListener('mousemove', this.aggiornaRidimensionamento);
      window.addEventListener('mouseup', this.terminaRidimensionamento);
      event.preventDefault();
    },
    aggiornaRidimensionamento(event: MouseEvent) {
      if (!this.ridimensionamento) return;
      const { bordo, startX, startY, startWidth, startHeight } = this.ridimensionamento;
      // Il pannello è ancorato con right/bottom fissi in CSS: crescere in
      // larghezza/altezza sposta automaticamente il bordo opposto (in
      // alto/a sinistra) senza bisogno di toccare la posizione a mano —
      // trascinare verso sinistra (dx negativo) o verso l'alto (dy
      // negativo) deve INGRANDIRE il pannello, da cui il segno invertito.
      let width = startWidth;
      let height = startHeight;
      if (bordo === 'left' || bordo === 'corner') width = startWidth - (event.clientX - startX);
      if (bordo === 'top' || bordo === 'corner') height = startHeight - (event.clientY - startY);
      const maxWidth = window.innerWidth - 48;
      const maxHeight = window.innerHeight - 48;
      this.dimensioniPersonalizzate = {
        width: Math.min(Math.max(width, DIMENSIONE_MIN_WIDTH), maxWidth),
        height: Math.min(Math.max(height, DIMENSIONE_MIN_HEIGHT), maxHeight),
      };
    },
    terminaRidimensionamento() {
      this.ridimensionamento = null;
      window.removeEventListener('mousemove', this.aggiornaRidimensionamento);
      window.removeEventListener('mouseup', this.terminaRidimensionamento);
      this.salvaDimensioni();
    },
    // localStorage invece del backend Tauri: è una preferenza puramente
    // cosmetica del pannello, non un dato dell'app — non vale la pena
    // farla transitare dal file di configurazione sul disco come le
    // impostazioni vere (vedi stores/settings.ts) solo per questo.
    leggiDimensioniSalvate(): { width: number; height: number } | null {
      try {
        const raw = localStorage.getItem(DIMENSIONI_STORAGE_KEY);
        if (!raw) return null;
        const parsed = JSON.parse(raw);
        if (typeof parsed?.width !== 'number' || typeof parsed?.height !== 'number') return null;
        // Clampata di nuovo contro la finestra ATTUALE — la dimensione fu
        // salvata magari con la finestra dell'app più grande di ora
        // (riavviata più piccola, monitor diverso...).
        return {
          width: Math.min(Math.max(parsed.width, DIMENSIONE_MIN_WIDTH), window.innerWidth - 48),
          height: Math.min(Math.max(parsed.height, DIMENSIONE_MIN_HEIGHT), window.innerHeight - 48),
        };
      } catch {
        // localStorage non disponibile o valore corrotto — si riparte
        // semplicemente dalla dimensione di default (clamp() in CSS).
        return null;
      }
    },
    salvaDimensioni() {
      if (!this.dimensioniPersonalizzate) return;
      try {
        localStorage.setItem(DIMENSIONI_STORAGE_KEY, JSON.stringify(this.dimensioniPersonalizzate));
      } catch {
        // Idem sopra — non bloccante, il ridimensionamento in sé ha
        // comunque funzionato per la sessione corrente.
      }
    },
    async verificaConfigurazione() {
      try {
        const config = await invoke<{ api_key: string } | null>('ai_agent_get_config');
        this.apiConfigurata = !!config && !!config.api_key.trim();
      } catch {
        // fuori da Tauri (npx vite puro) invoke() non esiste — non blocca
        // l'input in quel contesto di sviluppo, non c'è comunque un vero
        // backend a cui mandare il messaggio.
        this.apiConfigurata = true;
      }
    },
    apriImpostazioniAgente() {
      this.chiudi();
      this.$router.push('/settings/integrations');
    },
    async invia() {
      const testo = this.bozza.trim();
      if (!testo || this.inviando || this.apiConfigurata === false) return;
      this.errore = '';
      // La bolla mostrata in chat resta SOLO quello che l'utente ha
      // scritto (più l'etichetta, non i dati veri — vedi contestoLabel
      // sotto) — il testo coi dati veri viene anteposto solo alla
      // stringa mandata al backend qui sotto, mai a quella pushata in
      // messaggi. L'attività allegata vale per un solo messaggio, come
      // rispondere a un messaggio su WhatsApp — da qui lo scarto subito
      // dopo (ma l'etichetta resta in cronologia, sul messaggio stesso).
      const contesto = this.contestoAttivo;
      const testoDaInviare = contesto ? `${contesto.extra}\n\nDomanda dell'utente: ${testo}` : testo;
      if (contesto) this.aiChatContextStore.pulisci();
      this.messaggi.push({ ruolo: 'user', testo, contestoLabel: contesto?.label });
      this.bozza = '';
      // v-model svuota il valore ma non l'altezza impostata a mano da
      // adattaAltezzaInput() qui sotto — senza, il campo restava grande
      // anche da vuoto dopo un messaggio lungo.
      this.$nextTick(this.adattaAltezzaInput);
      this.inviando = true;
      this.riavviaTimerInattivita();
      this.scrollToBottom();
      try {
        const risposta = await invoke<RispostaAgente>('ai_agent_send_message', { testo: testoDaInviare });
        this.messaggi.push({
          ruolo: 'assistant',
          testo: risposta.testo,
          strumenti: risposta.strumenti_usati,
        });
        this.riavviaTimerInattivita();
      } catch (e: any) {
        this.errore = e?.toString?.() ?? String(e);
      } finally {
        this.inviando = false;
        this.scrollToBottom();
      }
    },
    async nuovaConversazione() {
      this.messaggi = [];
      this.errore = '';
      try {
        await invoke('ai_agent_new_conversation');
      } catch (e) {
        // fuori da Tauri (npx vite puro) invoke() non esiste — non è un
        // errore da mostrare, la cronologia locale è comunque svuotata.
      }
    },
    etichettaStrumenti(strumenti: string[]): string {
      const nomi = strumenti.map(s => {
        const chiave = `aiChat.tools.${s}`;
        const tradotto = this.$t(chiave) as string;
        return tradotto === chiave ? s : tradotto;
      });
      // Duplicati collassati (es. interroga_periodo chiamato due volte
      // per confrontare due periodi) invece di ripetere la stessa
      // etichetta più volte.
      return [...new Set(nomi)].join(', ');
    },
    renderizzaMarkdown(testo: string): string {
      // Il modello formatta spesso le risposte in Markdown (grassetto,
      // tabelle, elenchi) — parsato qui invece di mostrare i simboli
      // letterali. Sanitizzato con DOMPurify (stesso pacchetto già
      // usato altrove nell'app, es. util/swimlane.ts) prima di iniettarlo
      // con v-html: il testo arriva da una API esterna, non va mai
      // trattato come HTML sicuro senza passarci prima.
      const html = marked.parse(testo, { async: false }) as string;
      return DOMPurify.sanitize(html);
    },
    scrollToBottom() {
      this.$nextTick(() => {
        const el = this.$refs.messagesEl as HTMLElement | undefined;
        if (el) el.scrollTop = el.scrollHeight;
      });
    },
    // Il campo cresce con il testo digitato invece di restare fisso a
    // un rigo — richiesta esplicita dell'utente. `height: auto` prima di
    // rileggere scrollHeight è necessario: senza, un testo più corto di
    // uno precedente non farebbe mai RESTRINGERE il campo (scrollHeight
    // riflette l'altezza già impostata, non quella "naturale" del nuovo
    // contenuto). Il tetto a 140px è lo stesso già fissato in CSS
    // (max-height) — oltre, subentra lo scroll interno (.themed-scroll).
    adattaAltezzaInput() {
      const el = this.$refs.inputEl as HTMLTextAreaElement | undefined;
      if (!el) return;
      el.style.height = 'auto';
      el.style.height = `${Math.min(el.scrollHeight, 140)}px`;
    },
  },
};
</script>

<style scoped lang="scss">
@import '../style/theme.css';
@import '../style/modals.css';

.ai-chat-widget {
  // Sopra ai modali esistenti (arrivano fino a z-index 50, vedi
  // style/modals.css) — è il primo elemento fluttuante persistente
  // dell'app, deve restare visibile anche quando un modale è aperto.
  position: fixed;
  right: 24px;
  bottom: 24px;
  z-index: 60;
}

.ai-chat-fab {
  width: 52px;
  height: 52px;
  border-radius: 50%;
  border: none;
  background-color: var(--color-accent1);
  color: #241a12;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 20px;
  cursor: pointer;
  box-shadow: var(--shadow-elevated);

  &:hover {
    filter: brightness(1.08);
  }
}

.ai-chat-panel {
  // Ingrandita su richiesta esplicita dell'utente (era 360×480, troppo
  // piccola per leggere risposte più lunghe come tabelle di dati) — MA
  // con dimensioni fisse in px restava sempre 520×680 anche a finestra
  // piccola (non a schermo intero), sproporzionata rispetto alla
  // finestra reale. `clamp()` invece scala col viewport (= la finestra
  // stessa, qui, essendo un'app desktop senza chrome del browser): resta
  // 520×680 su una finestra grande (il valore massimo copre lo schermo
  // intero già visto andar bene), si restringe proporzionalmente su una
  // più piccola, senza mai scendere sotto un minimo leggibile.
  width: clamp(300px, 40vw, 520px);
  height: clamp(380px, 60vh, 680px);
  max-height: calc(100vh - 48px);
  max-width: calc(100vw - 48px);
  display: flex;
  flex-direction: column;
  background-color: var(--color-bg-elev);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-elevated);
  overflow: hidden;
  // Contenitore di posizionamento per le zone di trascinamento sotto —
  // altrimenti "absolute" le posizionerebbe rispetto a .ai-chat-widget
  // (che include anche il FAB) invece che rispetto al bordo del pannello.
  position: relative;
}

// Zone invisibili di trascinamento per il ridimensionamento manuale —
// solo in alto e a sinistra (vedi il commento nel template sul perché),
// leggermente più larghe dell'1px del bordo visibile per essere facili
// da agganciare col mouse, ma tenute DENTRO il bordo (non a cavallo,
// con offset negativi) perché overflow: hidden qui sopra le taglierebbe.
.ai-chat-resize-top {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 6px;
  cursor: ns-resize;
  z-index: 2;
}

.ai-chat-resize-left {
  position: absolute;
  top: 0;
  left: 0;
  bottom: 0;
  width: 6px;
  cursor: ew-resize;
  z-index: 2;
}

.ai-chat-resize-corner {
  position: absolute;
  top: 0;
  left: 0;
  width: 12px;
  height: 12px;
  cursor: nwse-resize;
  z-index: 3;
}

.ai-chat-header {
  flex: none;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 14px;
  border-bottom: 1px solid var(--color-border);
}

.ai-chat-title {
  font-weight: var(--font-weight-semibold);
  color: var(--color-text);
  font-size: var(--font-size-lg);
}

.ai-chat-header-actions {
  display: flex;
  gap: 2px;
}

.ai-chat-icon-btn {
  width: 34px;
  height: 34px;
  font-size: 16px;
  border: none;
  background: transparent;
  border-radius: var(--radius-sm);
  color: var(--color-text-dim);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;

  &:hover {
    background-color: var(--color-surface2);
  }
}

.ai-chat-messages {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 14px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.ai-chat-empty {
  color: var(--color-text-faint);
  font-size: var(--font-size-base);
  text-align: center;
  margin-top: 24px;
}

.ai-chat-empty-action {
  display: inline-block;
  margin-top: 10px;
  color: var(--color-accent1);
  font-weight: var(--font-weight-semibold);
  cursor: pointer;

  &:hover {
    text-decoration: underline;
  }
}

.ai-chat-bubble {
  max-width: 90%;
  padding: 10px 14px;
  border-radius: var(--radius-md);
  font-size: var(--font-size-base);
  line-height: 1.5;
  word-break: break-word;
}

// Solo i messaggi dell'utente (testo semplice, non Markdown) preservano
// gli a-capo letterali — il contenuto Markdown renderizzato gestisce la
// propria spaziatura tramite gli elementi HTML veri (<p>, <li>, ...),
// applicare pre-wrap anche lì raddoppierebbe gli spazi.
.ai-chat-plain {
  white-space: pre-wrap;
}

// Vue 2 (non Vue 3): ::v-deep, non :deep() — stesso pattern già usato
// altrove nell'app per raggiungere markup iniettato dinamicamente
// (qui l'HTML prodotto da marked via v-html).
.ai-chat-markdown {
  ::v-deep p {
    margin: 0 0 8px;

    &:last-child {
      margin-bottom: 0;
    }
  }

  ::v-deep ul,
  ::v-deep ol {
    margin: 0 0 8px;
    padding-left: 20px;

    &:last-child {
      margin-bottom: 0;
    }
  }

  ::v-deep li {
    margin-bottom: 2px;
  }

  ::v-deep strong {
    font-weight: var(--font-weight-bold);
  }

  ::v-deep code {
    font-family: 'SFMono-Regular', Consolas, 'Liberation Mono', Menlo, monospace;
    font-size: 0.9em;
    background-color: var(--color-bg);
    border-radius: var(--radius-sm);
    padding: 1px 5px;
  }

  ::v-deep pre {
    background-color: var(--color-bg);
    border-radius: var(--radius-sm);
    padding: 8px 10px;
    overflow-x: auto;
    margin: 0 0 8px;

    code {
      background: none;
      padding: 0;
    }
  }

  ::v-deep a {
    color: var(--color-accent1);
  }

  ::v-deep table {
    border-collapse: collapse;
    margin: 0 0 8px;
    font-size: 0.95em;
    max-width: 100%;
    display: block;
    overflow-x: auto;
  }

  ::v-deep th,
  ::v-deep td {
    border: 1px solid var(--color-border);
    padding: 4px 8px;
    text-align: left;
  }

  ::v-deep th {
    background-color: var(--color-bg);
    font-weight: var(--font-weight-semibold);
  }
}

.ai-chat-bubble-user {
  align-self: flex-end;
  background-color: var(--color-accent1);
  color: #241a12;
}

// Citazione permanente dell'attività allegata — stesso effetto della
// risposta a un messaggio su WhatsApp, ma qui resta per sempre nella
// bolla invece di sparire dopo l'invio (a differenza della barra di
// composizione, vedi ai-chat-context-row sotto). Overlay scuro
// semi-trasparente invece di un colore fisso: la bolla è gialla
// (--color-accent1) con testo scuro, un nero trasparente crea contrasto
// leggibile senza dover inventare un secondo colore ad hoc.
.ai-chat-bubble-quote {
  font-size: var(--font-size-sm);
  padding: 6px 8px;
  margin-bottom: 6px;
  border-left: 3px solid rgba(36, 26, 18, 0.5);
  border-radius: var(--radius-sm);
  background-color: rgba(36, 26, 18, 0.12);
  color: inherit;
  opacity: 0.85;
}

.ai-chat-bubble-assistant {
  align-self: flex-start;
  background-color: var(--color-surface2);
  color: var(--color-text);
}

.ai-chat-bubble-loading {
  color: var(--color-text-faint);
  font-style: italic;
}

.ai-chat-tools {
  font-size: var(--font-size-sm);
  color: var(--color-text-faint);
  margin-bottom: 4px;
  opacity: 0.85;
}

.ai-chat-alert {
  flex: none;
  margin: 0 14px 10px;
  padding: 10px 14px;
  border-radius: var(--radius-md);
  background-color: rgba(217, 83, 79, 0.15);
  color: var(--color-text);
  font-size: var(--font-size-base);
}

// Contenitore condiviso da citazione + campo di scrittura — quando la
// citazione è presente, i due elementi devono leggersi come un solo
// "cubo" con due zone di colore diverso (bordo continuo, nessuno
// stacco), non due riquadri separati. Richiesta estetica esplicita
// dell'utente, con uno screenshot di WhatsApp come riferimento.
.ai-chat-compose {
  flex: none;
  padding: 10px;
  border-top: 1px solid var(--color-border);
}

.ai-chat-context-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 8px 10px;
  // Barra verticale a sinistra come una vera "citazione" — stesso
  // linguaggio visivo del reply di WhatsApp. border-bottom: none perché
  // si salda direttamente al campo di scrittura sotto (vedi
  // .ai-chat-input-row--attached), niente doppio bordo nel mezzo.
  border: 1px solid var(--color-border);
  border-bottom: none;
  border-left: 3px solid var(--color-accent1);
  border-radius: var(--radius-md) var(--radius-md) 0 0;
  background-color: var(--color-bg);
}

.ai-chat-context-label {
  font-size: var(--font-size-sm);
  color: var(--color-text-dim);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

// Area di click ingrandita da 22 a 30px — segnalata troppo piccola
// dall'utente.
.ai-chat-context-remove {
  flex-shrink: 0;
  width: 30px;
  height: 30px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  background: transparent;
  color: var(--color-text-faint);
  font-size: 15px;
  cursor: pointer;
  border-radius: var(--radius-sm);

  &:hover {
    color: var(--color-text);
    background-color: var(--color-surface2);
  }
}

.ai-chat-input-row {
  display: flex;
  align-items: flex-end;
  gap: 8px;
}

// Bordo/sfondo del campo si sposta sulla riga intera e si salda a quello
// della citazione sopra (stesso var(--color-border), angoli superiori
// squadrati) — il textarea stesso perde il proprio bordo (sotto) per non
// disegnarne uno doppio nel punto di contatto.
.ai-chat-input-row--attached {
  padding: 8px;
  border: 1px solid var(--color-border);
  border-radius: 0 0 var(--radius-md) var(--radius-md);
  background-color: var(--color-surface2);
}

.ai-chat-input {
  flex: 1;
  resize: none;
  height: 36px;
  max-height: 140px;
  overflow-y: auto;
  padding: 7px 10px;
  border-radius: var(--radius-md);
  border: 1px solid var(--color-border);
  background-color: var(--color-surface2);
  color: var(--color-text);
  font-size: var(--font-size-base);
  font-family: inherit;

  &:focus {
    outline: none;
    border-color: var(--color-accent1);
  }
}

.ai-chat-input--attached {
  border: none;
  border-radius: 0;
  background-color: transparent;

  &:focus {
    outline: none;
  }
}

.ai-chat-send-btn {
  width: 36px;
  height: 36px;
  flex: none;
  border: none;
  border-radius: var(--radius-md);
  background-color: var(--color-accent1);
  color: #241a12;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;

  &:disabled {
    opacity: 0.5;
    cursor: default;
  }

  &:not(:disabled):hover {
    filter: brightness(1.08);
  }
}
</style>

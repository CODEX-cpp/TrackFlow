<template lang="pug">
div.ai-chat-widget
  button.ai-chat-fab(v-if="!open" type="button" @click="apri" :title="$t('aiChat.open')")
    icon(name="comments")
  div.ai-chat-panel(v-if="open")
    div.ai-chat-header
      span.ai-chat-title {{ $t('aiChat.title') }}
      div.ai-chat-header-actions
        button.ai-chat-icon-btn(type="button" @click="nuovaConversazione" :title="$t('aiChat.newConversation')")
          icon(name="sync")
        button.ai-chat-icon-btn(type="button" @click="open = false" :title="$t('aiChat.close')")
          icon(name="times")

    div.ai-chat-messages.themed-scroll(ref="messagesEl")
      div.ai-chat-empty(v-if="messaggi.length === 0") {{ $t('aiChat.emptyHint') }}
      div.ai-chat-bubble(v-for="(m, i) in messaggi" :key="i" :class="'ai-chat-bubble-' + m.ruolo")
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

    div.ai-chat-input-row
      textarea.ai-chat-input.themed-scroll(
        ref="inputEl"
        v-model="bozza"
        :placeholder="$t('aiChat.placeholder')"
        :disabled="inviando"
        @keydown.enter.exact.prevent="invia"
        @input="adattaAltezzaInput"
      )
      button.ai-chat-send-btn(type="button" @click="invia" :disabled="inviando || !bozza.trim()")
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

marked.setOptions({ breaks: true });

interface Messaggio {
  ruolo: 'user' | 'assistant';
  testo: string;
  strumenti?: string[];
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
    };
  },
  methods: {
    apri() {
      this.open = true;
    },
    async invia() {
      const testo = this.bozza.trim();
      if (!testo || this.inviando) return;
      this.errore = '';
      this.messaggi.push({ ruolo: 'user', testo });
      this.bozza = '';
      // v-model svuota il valore ma non l'altezza impostata a mano da
      // adattaAltezzaInput() qui sotto — senza, il campo restava grande
      // anche da vuoto dopo un messaggio lungo.
      this.$nextTick(this.adattaAltezzaInput);
      this.inviando = true;
      this.scrollToBottom();
      try {
        const risposta = await invoke<RispostaAgente>('ai_agent_send_message', { testo });
        this.messaggi.push({
          ruolo: 'assistant',
          testo: risposta.testo,
          strumenti: risposta.strumenti_usati,
        });
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

.ai-chat-input-row {
  flex: none;
  display: flex;
  align-items: flex-end;
  gap: 8px;
  padding: 12px;
  border-top: 1px solid var(--color-border);
}

.ai-chat-input {
  flex: 1;
  resize: none;
  height: 46px;
  max-height: 140px;
  overflow-y: auto;
  padding: 10px 12px;
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

.ai-chat-send-btn {
  width: 46px;
  height: 46px;
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

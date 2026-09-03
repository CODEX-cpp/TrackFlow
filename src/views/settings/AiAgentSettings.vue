<template lang="pug">
div
  div.settings-row
    div
      div.settings-row-title {{ $t('settings.aiAgent.title') }}
      div.settings-row-help {{ $t('settings.aiAgent.help') }}
    div.pill-btn(@click="!saving && save()" :class="{ 'pill-btn-disabled': saving }")
      | {{ saving ? $t('settings.aiAgent.saving') : $t('settings.aiAgent.save') }}

  div.settings-alert.settings-alert-danger(v-if="error") {{ error }}
  div.settings-alert.settings-alert-success(v-if="success")
    | {{ $t('settings.aiAgent.saved') }}
    span.settings-alert-close(@click="success = false") ×

  div.settings-field-row
    label.settings-field-label {{ $t('settings.aiAgent.provider') }}
    select.settings-field.aiagent-provider-field(v-model="provider")
      option(value="anthropic") {{ $t('settings.aiAgent.providerAnthropic') }}
      option(value="claude_desktop") {{ $t('settings.aiAgent.providerClaudeDesktop') }}

  div.settings-field-row(v-if="provider === 'anthropic'")
    label.settings-field-label {{ $t('settings.aiAgent.apiKey') }}
    div
      input.settings-field(v-model="apiKey" type="password" :placeholder="$t('settings.aiAgent.apiKeyPlaceholder')")
      div.settings-row-help {{ $t('settings.aiAgent.apiKeyHint') }}

  template(v-else)
    div.settings-row-help.aiagent-claude-desktop-help {{ $t('settings.aiAgent.claudeDesktopHelp') }}
    div.settings-alert.settings-alert-danger(v-if="verificatoDisponibilita && !claudeDesktopTrovato")
      | {{ $t('settings.aiAgent.claudeDesktopNotFound') }}
    //- Installata ma non ancora autenticata (bug reale: Claude Desktop
    //- collegata non basta, il CLI bundlato ha un proprio login separato
    //- da completare una tantum in un terminale — vedi claude_subscription.rs).
    //- Istruzioni passo-passo invece di un errore generico: chi arriva
    //- qui potrebbe non aver mai usato un terminale in vita sua.
    div.settings-alert.settings-alert-warning(v-if="verificatoDisponibilita && claudeDesktopTrovato && !claudeDesktopAutenticato")
      div.aiagent-auth-title {{ $t('settings.aiAgent.claudeDesktopNotAuthenticatedTitle') }}
      div {{ $t('settings.aiAgent.claudeDesktopNotAuthenticatedBody') }}
      ol.aiagent-auth-steps
        li {{ $t('settings.aiAgent.claudeDesktopStep1') }}
        li
          | {{ $t('settings.aiAgent.claudeDesktopStep2') }}
          div.aiagent-auth-command
            code {{ comandoAutenticazione }}
            div.pill-btn-ghost.aiagent-copy-btn(@click="copiaComandoAutenticazione")
              | {{ comandoCopiato ? $t('settings.aiAgent.claudeDesktopCopyCommandCopied') : $t('settings.aiAgent.claudeDesktopCopyCommand') }}
        li {{ $t('settings.aiAgent.claudeDesktopStep3') }}
        li {{ $t('settings.aiAgent.claudeDesktopStep4') }}
      div.pill-btn-ghost(@click="verificaClaudeDesktop") {{ $t('settings.aiAgent.claudeDesktopRecheck') }}

  div.settings-field-row
    label.settings-field-label {{ $t('settings.aiAgent.model') }}
    div
      div.aiagent-model-row
        select.settings-field(v-model="model" :disabled="modelli.length === 0")
          option(v-if="modelli.length === 0" value="") {{ $t('settings.aiAgent.modelsEmpty') }}
          option(v-for="m in modelli" :key="m.id" :value="m.id") {{ m.nome }}
        div.pill-btn-ghost.aiagent-refresh-btn(
          v-if="provider === 'anthropic'"
          @click="!caricandoModelli && apiKey.trim() && aggiornaModelli()"
          :class="{ 'pill-btn-disabled': caricandoModelli || !apiKey.trim() }"
        )
          | {{ caricandoModelli ? $t('settings.aiAgent.modelsLoading') : $t('settings.aiAgent.modelsRefresh') }}
      div.settings-alert.settings-alert-danger(v-if="errorModelli") {{ errorModelli }}
      div.settings-row-help(v-else) {{ $t('settings.aiAgent.modelHint') }}
</template>

<script lang="ts">
import { invoke } from '@tauri-apps/api/core';

interface AiAgentConfig {
  provider: string;
  api_key: string;
  model: string;
}

interface ModelloDisponibile {
  id: string;
  nome: string;
}

export default {
  name: 'AiAgentSettings',
  data() {
    return {
      // Default per chi non ha ancora salvato nulla — richiesta esplicita
      // dell'utente dopo aver verificato che questa modalità funziona
      // bene: niente chiave API da procurarsi, usa subito l'abbonamento
      // Claude Desktop già attivo. Chi ha già una configurazione salvata
      // la ritrova invariata in mounted() (questi sono solo i valori di
      // partenza prima che arrivi la risposta di ai_agent_get_config).
      provider: 'claude_desktop',
      apiKey: '',
      // Vuoto apposta: aggiornaModelli() (chiamata subito in mounted()
      // per questo provider) sceglie da sola l'alias "haiku" — più
      // veloce/economico, stesso comportamento già in uso per l'altro
      // provider quando il modello non è ancora stato scelto.
      model: '',
      saving: false,
      error: '',
      success: false,
      modelli: [] as ModelloDisponibile[],
      caricandoModelli: false,
      errorModelli: '',
      // Provider "Claude (abbonamento Desktop)" — vedi
      // claude_subscription.rs. Verificato una volta all'apertura della
      // pagina (e ogni volta che l'utente sceglie questo provider), così
      // un avviso chiaro compare SUBITO se Claude Desktop non è
      // installata (o installata ma non ancora autenticata — bug reale
      // segnalato da un utente: avere l'app Desktop collegata non basta,
      // il CLI bundlato ha un proprio login separato da completare una
      // tantum), invece di scoprirlo solo al primo messaggio mandato in
      // chat.
      claudeDesktopTrovato: false,
      claudeDesktopAutenticato: false,
      claudeDesktopPercorsoExe: '',
      verificatoDisponibilita: false,
      comandoCopiato: false,
    };
  },
  computed: {
    // Comando pronto da copiare in PowerShell — usa il percorso REALE
    // trovato su questo PC (versione del CLI inclusa), non un
    // placeholder generico da adattare a mano: l'utente lo incolla e
    // basta. Racchiuso tra virgolette e preceduto da "&" per gestire
    // correttamente eventuali spazi nel percorso (es. "Program Files"),
    // sintassi PowerShell dell'operatore di chiamata.
    comandoAutenticazione(this: any): string {
      return this.claudeDesktopPercorsoExe ? `& "${this.claudeDesktopPercorsoExe}"` : '';
    },
  },
  watch: {
    async provider(nuovo: string, precedente: string) {
      // Bug reale trovato dal log diagnostico: gli ID modello dei due
      // provider vivono in spazi diversi (Anthropic usa id completi con
      // data, es. "claude-sonnet-4-5-20250929"; Claude Code CLI vuole
      // alias corti come "sonnet"/"haiku") — senza azzerarlo qui, un
      // cambio di provider si portava dietro l'id del provider
      // precedente, che l'altro non riconosce come pensato (nessun
      // errore esplicito, semplicemente non il modello inteso).
      if (precedente && nuovo !== precedente) {
        this.model = '';
        this.modelli = [];
      }
      if (nuovo === 'claude_desktop') {
        await this.verificaClaudeDesktop();
        // Nessuna chiave API per questo provider — l'elenco modelli è
        // statico (vedi ai_agent_list_models), si carica subito.
        await this.aggiornaModelli();
      }
    },
  },
  async mounted() {
    try {
      const config = await invoke<AiAgentConfig | null>('ai_agent_get_config');
      if (config) {
        this.provider = config.provider;
        this.apiKey = config.api_key;
        this.model = config.model;
      }
      if (this.provider === 'claude_desktop') {
        await this.verificaClaudeDesktop();
        await this.aggiornaModelli();
      } else if (this.apiKey.trim()) {
        // Se c'è già una chiave salvata, l'elenco modelli si ricarica da
        // solo — l'utente non deve premere "Aggiorna" solo per riaprire
        // la pagina.
        await this.aggiornaModelli();
      }
    } catch (e) {
      // L'app potrebbe girare fuori da Tauri durante lo sviluppo web puro
      // (npx vite senza il guscio nativo) — invoke() non esiste in quel
      // caso, non è un errore da mostrare all'utente.
    }
  },
  methods: {
    async verificaClaudeDesktop(this: any) {
      try {
        const stato = await invoke<{ trovato: boolean; autenticato: boolean; percorso_exe: string | null }>(
          'claude_desktop_stato',
        );
        this.claudeDesktopTrovato = stato.trovato;
        this.claudeDesktopAutenticato = stato.autenticato;
        this.claudeDesktopPercorsoExe = stato.percorso_exe ?? '';
        this.comandoCopiato = false;
      } catch (e) {
        // Fuori da Tauri — non bloccante.
      } finally {
        this.verificatoDisponibilita = true;
      }
    },
    async copiaComandoAutenticazione(this: any) {
      try {
        await navigator.clipboard.writeText(this.comandoAutenticazione);
        this.comandoCopiato = true;
        setTimeout(() => {
          this.comandoCopiato = false;
        }, 2000);
      } catch (e) {
        // Clipboard non disponibile (permesso negato, contesto non
        // sicuro) — l'utente può comunque selezionare/copiare il testo
        // mostrato a mano, non è bloccante.
      }
    },
    async aggiornaModelli() {
      this.errorModelli = '';
      this.caricandoModelli = true;
      try {
        const modelli = await invoke<ModelloDisponibile[]>('ai_agent_list_models', {
          provider: this.provider,
          apiKey: this.apiKey,
        });
        // Il modello già selezionato/salvato resta un'opzione valida
        // anche se non compare nell'elenco appena scaricato (es. un
        // modello ritirato dal provider) — non lo perdiamo silenziosamente
        // sotto ai piedi dell'utente.
        this.modelli =
          this.model && !modelli.some(m => m.id === this.model)
            ? [{ id: this.model, nome: this.model }, ...modelli]
            : modelli;
        if (!this.model && modelli.length > 0) {
          // Default esplicito richiesto dall'utente: Haiku 4.5 per l'uso
          // quotidiano (il più economico/veloce, sufficiente per query
          // semplici sui dati) — non il primo della lista qualunque esso
          // sia (spesso il modello più costoso, es. Opus). Riconosciuto
          // per sottostringa dell'id ("haiku-4-5"), non un id fisso
          // completo — Anthropic aggiunge una data alla fine dell'id
          // (es. "claude-haiku-4-5-20251001") che potrebbe cambiare.
          // Sul provider claude_desktop gli id sono già alias corti
          // ("sonnet"/"opus"/"haiku"), quindi la ricerca cerca "haiku"
          // e basta lì; su "anthropic" resta specifica per la versione
          // 4.5 come da comportamento originale.
          const chiaveRicerca = this.provider === 'claude_desktop' ? 'haiku' : 'haiku-4-5';
          const haiku45 = modelli.find(m => m.id.includes(chiaveRicerca));
          this.model = haiku45 ? haiku45.id : modelli[0].id;
        }
      } catch (e: any) {
        this.errorModelli = e?.toString?.() ?? String(e);
      } finally {
        this.caricandoModelli = false;
      }
    },
    async save() {
      this.error = '';
      this.success = false;
      this.saving = true;
      try {
        await invoke('ai_agent_save_config', {
          provider: this.provider,
          apiKey: this.apiKey,
          model: this.model,
        });
        this.success = true;
      } catch (e: any) {
        this.error = `${this.$t('settings.aiAgent.saveError')} ${e?.message ?? e}`;
      } finally {
        this.saving = false;
      }
    },
  },
};
</script>

<style scoped>
/* Stesso motivo/pattern del select del modello qui sotto: senza
   questo cresceva con flex:1 fino a riempire tutta la larghezza della
   card, un form lunghissimo per un'unica opzione ("Claude
   (Anthropic)"). Selettore più specifico di ".settings-field-row >
   .settings-field" (due classi, vedi settingsPanel.css) apposta: il
   select del provider è figlio diretto di .settings-field-row (a
   differenza di quello del modello, dentro un div in più), quindi
   .aiagent-provider-field da sola perdeva sempre contro quella regola
   — bug reale trovato in test, stesso tipo di quello del bordo dorato
   del drag&drop in Buckets.vue. */
.settings-field-row > .aiagent-provider-field {
  flex: 0 1 260px;
}

.aiagent-model-row {
  display: flex;
  gap: 8px;
  align-items: center;
}

/* Non più flex:1 (si allargava a riempire tutta la larghezza della
   card, "un form molto lungo" segnalato dall'utente) — cresce solo
   fino a un massimo ragionevole, si restringe sotto quello se lo
   spazio disponibile è minore (finestra stretta). */
.aiagent-model-row > select {
  flex: 0 1 260px;
}

.aiagent-refresh-btn {
  flex: none;
  white-space: nowrap;
}

.aiagent-claude-desktop-help {
  margin-bottom: 14px;
}

.aiagent-auth-title {
  font-weight: 600;
  margin-bottom: 4px;
}

.aiagent-auth-steps {
  margin: 10px 0;
  padding-left: 20px;
}

.aiagent-auth-steps li {
  margin-bottom: 8px;
}

.aiagent-auth-command {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 6px;
}

.aiagent-auth-command code {
  flex: 1;
  min-width: 0;
  overflow-x: auto;
  white-space: nowrap;
  background-color: var(--color-bg-inset, rgba(0, 0, 0, 0.2));
  padding: 6px 10px;
  border-radius: var(--radius-sm, 4px);
  font-size: var(--font-size-sm);
}

.aiagent-copy-btn {
  flex: none;
  white-space: nowrap;
}
</style>

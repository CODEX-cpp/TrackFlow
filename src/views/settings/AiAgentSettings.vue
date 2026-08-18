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

  div.settings-field-row
    label.settings-field-label {{ $t('settings.aiAgent.apiKey') }}
    div
      input.settings-field(v-model="apiKey" type="password" :placeholder="$t('settings.aiAgent.apiKeyPlaceholder')")
      div.settings-row-help {{ $t('settings.aiAgent.apiKeyHint') }}

  div.settings-field-row
    label.settings-field-label {{ $t('settings.aiAgent.model') }}
    div
      div.aiagent-model-row
        select.settings-field(v-model="model" :disabled="modelli.length === 0")
          option(v-if="modelli.length === 0" value="") {{ $t('settings.aiAgent.modelsEmpty') }}
          option(v-for="m in modelli" :key="m.id" :value="m.id") {{ m.nome }}
        div.pill-btn-ghost.aiagent-refresh-btn(
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
      provider: 'anthropic',
      apiKey: '',
      model: '',
      saving: false,
      error: '',
      success: false,
      modelli: [] as ModelloDisponibile[],
      caricandoModelli: false,
      errorModelli: '',
    };
  },
  async mounted() {
    try {
      const config = await invoke<AiAgentConfig | null>('ai_agent_get_config');
      if (config) {
        this.provider = config.provider;
        this.apiKey = config.api_key;
        this.model = config.model;
        // Se c'è già una chiave salvata, l'elenco modelli si ricarica da
        // solo — l'utente non deve premere "Aggiorna" solo per riaprire
        // la pagina.
        if (this.apiKey.trim()) await this.aggiornaModelli();
      }
    } catch (e) {
      // L'app potrebbe girare fuori da Tauri durante lo sviluppo web puro
      // (npx vite senza il guscio nativo) — invoke() non esiste in quel
      // caso, non è un errore da mostrare all'utente.
    }
  },
  methods: {
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
          const haiku45 = modelli.find(m => m.id.includes('haiku-4-5'));
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
</style>

<template lang="pug">
div
  div.modal-backdrop(@click="$emit('close')")
  div.edit-modal.wizard-modal.themed-scroll

    template(v-if="entryType === 'watcher'")
      div.edit-modal-title {{ $t('customModuleWizard.watcherPickerTitle') }}
      div(v-if="watcherModules.length === 0")
        div.field-hint {{ $t('customModuleWizard.watcherPickerEmpty') }}
        div.pill-btn.go-to-raw-data(@click="goToRawData") {{ $t('customModuleWizard.goToRawData') }}
      template(v-else)
        div.html-module-row(
          v-for="w in watcherModules"
          :key="w.id"
          :class="{ 'html-module-row-selected': selectedWatcherId === w.id }"
          @click="selectedWatcherId = w.id"
        )
          | {{ w.name }}
        div.edit-modal-actions
          div.pill-btn-ghost(@click="goToRawData") {{ $t('customModuleWizard.goToRawData') }}
          div.pill-btn(:class="{ 'pill-btn-disabled': !selectedWatcherId }" @click="selectWatcher") {{ $t('customModuleWizard.select') }}

    template(v-if="entryType === 'html' && htmlStep === 'picker'")
      div.edit-modal-title {{ $t('customModuleWizard.htmlPickerTitle') }}
      div(v-if="htmlModules.length === 0") {{ $t('customModuleWizard.htmlPickerEmpty') }}
      div.html-module-row(
        v-for="m in htmlModules"
        :key="m.id"
        :class="{ 'html-module-row-selected': selectedHtmlId === m.id }"
        @click="selectedHtmlId = m.id"
      )
        | {{ m.title }}
      div.edit-modal-actions
        div.pill-btn-ghost(@click="htmlStep = 'new'") {{ $t('customModuleWizard.htmlCreateNew') }}
        div.pill-btn(:class="{ 'pill-btn-disabled': !selectedHtmlId }" @click="selectHtmlModule") {{ $t('customModuleWizard.select') }}

    template(v-if="entryType === 'html' && htmlStep === 'new'")
      div.edit-modal-title {{ $t('customModuleWizard.htmlCreateNew') }}
      div.edit-field-label {{ $t('customModuleWizard.name') }}
      input.edit-field(
        v-model="newHtmlName"
        type="text"
        :placeholder="$t('customModuleWizard.namePlaceholder')"
        @keyup.enter="createNewHtmlModule"
      )
      div.field-error(v-if="newHtmlNameError") {{ $t('customModuleWizard.nameRequired') }}
      div.edit-modal-actions
        div.pill-btn-ghost(@click="htmlStep = 'picker'") {{ $t('customModuleWizard.back') }}
        div.pill-btn(@click="createNewHtmlModule") {{ $t('customModuleWizard.create') }}

</template>

<script lang="ts">
import { invoke } from '@tauri-apps/api/core';

interface ModuloPersonalizzatoInfo {
  id: string;
  title: string;
}

interface WatcherPersonalizzatoInfo {
  id: string;
  name: string;
  mode: string;
  timeline_lane: boolean;
  running: boolean;
  bucket_id: string | null;
  grid_width: number | null;
  template_id: string | null;
}

export default {
  name: 'CustomModulePicker',
  props: {
    entryType: { type: String, required: true }, // 'watcher' | 'html'
  },
  data() {
    return {
      htmlModules: [] as ModuloPersonalizzatoInfo[],
      selectedHtmlId: '',
      htmlStep: 'picker' as 'picker' | 'new',
      newHtmlName: '',
      newHtmlNameError: false,

      watcherModules: [] as WatcherPersonalizzatoInfo[],
      selectedWatcherId: '',
    };
  },
  async mounted() {
    if (this.entryType === 'html') {
      await this.loadHtmlModules();
    } else if (this.entryType === 'watcher') {
      await this.loadWatcherModules();
    }
  },
  methods: {
    async loadHtmlModules() {
      try {
        this.htmlModules = await invoke<ModuloPersonalizzatoInfo[]>('elenca_moduli_personalizzati');
      } catch {
        this.htmlModules = [];
      }
    },
    async loadWatcherModules() {
      try {
        const all = await invoke<WatcherPersonalizzatoInfo[]>('elenca_watcher_personalizzati');
        // Solo i watcher con un bucket_id noto sono collegabili a un
        // modulo — vedi custom_watchers.rs: per modalità "interval" è
        // sempre calcolabile, per "raw"/esperta solo se l'utente lo ha
        // dichiarato in fase di creazione (Dati grezzi).
        this.watcherModules = all.filter(w => !!w.bucket_id);
      } catch {
        this.watcherModules = [];
      }
    },
    goToRawData() {
      this.$emit('close');
      this.$router.push('/buckets');
    },
    selectWatcher() {
      if (!this.selectedWatcherId) return;
      const chosen = this.watcherModules.find(w => w.id === this.selectedWatcherId);
      if (!chosen) return;
      this.$emit('created', {
        type: 'custom_watcher_view',
        // Larghezza e modello vivono nel manifest del watcher
        // (grid_width/template_id), non solo nelle props del modulo
        // Home — così riaggiungerlo dopo averlo tolto per sbaglio
        // ripristina la stessa dimensione/visualizzazione scelta alla
        // creazione, invece di ripartire sempre dai default.
        props: {
          bucketId: chosen.bucket_id,
          title: chosen.name,
          gridWidth: chosen.grid_width || 1,
          templateId: chosen.template_id || undefined,
        },
      });
    },
    async createNewHtmlModule() {
      const nome = this.newHtmlName.trim();
      if (!nome) {
        this.newHtmlNameError = true;
        return;
      }
      this.newHtmlNameError = false;
      try {
        await invoke('crea_modulo_personalizzato', { nome });
        await this.loadHtmlModules();
        const created = this.htmlModules.find(m => m.title === nome) || this.htmlModules[this.htmlModules.length - 1];
        if (created) {
          this.selectedHtmlId = created.id;
          await invoke('apri_cartella_custom_modulo', { id: created.id });
        }
        this.newHtmlName = '';
        this.htmlStep = 'picker';
      } catch {
        this.newHtmlNameError = true;
      }
    },
    selectHtmlModule() {
      if (!this.selectedHtmlId) return;
      const chosen = this.htmlModules.find(m => m.id === this.selectedHtmlId);
      this.$emit('created', {
        type: 'custom_html_module',
        props: { visname: this.selectedHtmlId, title: chosen ? chosen.title : this.selectedHtmlId },
      });
    },
  },
};
</script>

<style lang="scss" scoped>
@import '../style/theme.css';
@import '../style/modals.css';

.wizard-modal {
  width: 420px;
}

.field-error {
  font-size: var(--font-size-xs);
  color: #d9534f;
  margin-top: 4px;
}

.field-hint {
  font-size: var(--font-size-xs);
  color: var(--color-text-faint);
  margin-top: 4px;
}

.html-module-row {
  padding: 8px 10px;
  border-radius: var(--radius-md);
  cursor: pointer;
  font-size: var(--font-size-sm);
  color: var(--color-text);
}

.html-module-row:hover {
  background-color: var(--color-surface2);
}

.html-module-row-selected {
  background-color: var(--color-surface2);
  outline: 1px solid var(--color-accent1);
}

.go-to-raw-data {
  margin-top: 10px;
}
</style>

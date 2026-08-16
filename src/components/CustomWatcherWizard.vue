<template lang="pug">
div
  div.modal-backdrop(@click="$emit('close')")
  div.edit-modal.wizard-modal.themed-scroll

    template(v-if="step === 'mode'")
      div.edit-modal-title {{ $t('customModuleWizard.chooseType') }}
      div.mode-grid
        div.mode-card(@click="step = 'simple'")
          div.mode-card-title {{ $t('customModuleWizard.simpleTitle') }}
          div.mode-card-desc {{ $t('customModuleWizard.simpleDesc') }}
        div.mode-card(@click="step = 'expert'")
          div.mode-card-title {{ $t('customModuleWizard.expertTitle') }}
          div.mode-card-desc {{ $t('customModuleWizard.expertDesc') }}
      div.edit-modal-actions
        div.pill-btn-ghost(@click="$emit('close')") {{ $t('customModuleWizard.close') }}

    template(v-if="step === 'simple'")
      div.edit-modal-title {{ $t('customModuleWizard.simpleTitle') }}
      div.edit-field-label {{ $t('customModuleWizard.name') }}
      input.edit-field(
        v-model="name"
        type="text"
        :placeholder="$t('customModuleWizard.namePlaceholder')"
      )
      div.field-error(v-if="nameError") {{ $t('customModuleWizard.nameRequired') }}

      div.edit-field-label {{ $t('customModuleWizard.interval') }}
      select.edit-field(v-model.number="intervalSeconds")
        option(:value="30") {{ $t('customModuleWizard.interval30') }}
        option(:value="60") {{ $t('customModuleWizard.interval60') }}
        option(:value="300") {{ $t('customModuleWizard.interval300') }}
        option(:value="900") {{ $t('customModuleWizard.interval900') }}

      div.toggle-row
        div {{ $t('customModuleWizard.showOnTimeline') }}
        div.settings-toggle(:class="{ 'settings-toggle-on': showOnTimeline }" @click="showOnTimeline = !showOnTimeline")
          div.settings-toggle-thumb

      div.edit-modal-actions
        div.pill-btn-ghost(@click="step = 'mode'") {{ $t('customModuleWizard.back') }}
        div.pill-btn(@click="goToSizeStep") {{ $t('customModuleWizard.continueButton') }}

    template(v-if="step === 'size'")
      div.edit-modal-title {{ $t('customModuleWizard.chooseSize') }}
      div.field-hint {{ $t('customModuleWizard.chooseSizeHint') }}
      div.size-grid
        div.size-card(
          v-for="preset in sizePresets"
          :key="preset.width"
          :class="{ 'size-card-selected': selectedSize === preset }"
          @click="selectedSize = preset"
        )
          div.size-rect(:style="sizeRectStyle(preset)")
          div.size-card-label {{ preset.width }}
      div.edit-modal-actions
        div.pill-btn-ghost(@click="step = 'simple'") {{ $t('customModuleWizard.back') }}
        div.pill-btn(@click="step = 'template'") {{ $t('customModuleWizard.continueButton') }}

    template(v-if="step === 'template'")
      div.edit-modal-title {{ $t('customModuleWizard.chooseTemplate') }}
      div.field-hint {{ $t('customModuleWizard.chooseTemplateHint') }}
      div.template-grid
        div.template-card(
          :class="{ 'template-card-selected': selectedTemplateId === null }"
          @click="selectedTemplateId = null"
        )
          div.template-card-name {{ $t('customModuleWizard.templateDefault') }}
          div.template-card-preview.template-card-preview-generic
            div.template-generic-row(v-for="n in 2" :key="n")
        div.template-card(
          v-for="modello in templates"
          :key="modello.id"
          :class="{ 'template-card-selected': selectedTemplateId === modello.id }"
          @click="selectedTemplateId = modello.id"
        )
          div.template-card-name {{ nomeModello(modello) }}
          iframe.template-card-preview(
            :src="templatePreviewSrc(modello)"
            frameborder="0"
            scrolling="no"
            @load="onPreviewLoad"
          )
      div.edit-modal-actions
        div.pill-btn-ghost(@click="step = 'size'") {{ $t('customModuleWizard.back') }}
        div.pill-btn(@click="submitSimple") {{ $t('customModuleWizard.create') }}

    template(v-if="step === 'expert'")
      div.edit-modal-title {{ $t('customModuleWizard.expertTitle') }}
      div.edit-field-label {{ $t('customModuleWizard.expertName') }}
      input.edit-field(v-model="expertName" type="text" :placeholder="$t('customModuleWizard.namePlaceholder')")
      div.field-error(v-if="expertNameError") {{ $t('customModuleWizard.nameRequired') }}

      div.edit-field-label {{ $t('customModuleWizard.expertCommand') }}
      input.edit-field(v-model="expertCommand" type="text" :placeholder="$t('customModuleWizard.expertCommandPlaceholder')")
      div.field-error(v-if="expertCommandError") {{ $t('customModuleWizard.nameRequired') }}

      div.edit-field-label {{ $t('customModuleWizard.expertBucket') }}
      input.edit-field(v-model="expertBucketId" type="text" :placeholder="$t('customModuleWizard.expertBucketPlaceholder')")
      div.field-hint {{ $t('customModuleWizard.expertBucketHint') }}

      div.field-hint.expert-contract-hint {{ $t('customModuleWizard.expertHint') }}

      div.edit-modal-actions
        div.pill-btn-ghost(@click="step = 'mode'") {{ $t('customModuleWizard.back') }}
        div.pill-btn(@click="submitExpert") {{ $t('customModuleWizard.createWatcher') }}

    template(v-if="step === 'confirm'")
      div.confirm-box
        div.confirm-box-title {{ $t('customModuleWizard.folderCreated') }}
        div.confirm-box-path {{ confirmFolderPath }}
      div.field-hint {{ confirmContractText }}
      pre.confirm-json(v-if="confirmMode === 'simple'") {{ '{"minuti_attivi": 12, "progetto": "Logo Cliente X"}' }}
      div.edit-modal-actions
        div.pill-btn-ghost(@click="openFolder") {{ $t('customModuleWizard.openFolder') }}
        div.pill-btn(@click="$emit('done')") {{ $t('customModuleWizard.finish') }}

</template>

<script lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { useViewsStore } from '~/stores/views';

interface ModelloVisualizzazioneInfo {
  id: string;
  nome_it: string;
  nome_en: string;
}

interface SizePreset {
  width: number;
}

// Dimensioni preimpostate offerte nello step "size" — solo la larghezza
// (in colonne, 1-4) ha un effetto reale sul modulo creato in Home;
// richiesta esplicita: niente più altezza variabile nei rettangoli
// (i moduli restano comunque ad altezza automatica come tutti gli
// altri in Home), i rettangoli si distinguono solo per larghezza.
const SIZE_PRESETS: SizePreset[] = [{ width: 1 }, { width: 2 }, { width: 3 }, { width: 4 }];

export default {
  name: 'CustomWatcherWizard',
  data() {
    return {
      step: 'mode' as 'mode' | 'simple' | 'size' | 'template' | 'expert' | 'confirm',
      sizePresets: SIZE_PRESETS,
      selectedSize: SIZE_PRESETS[1] as SizePreset,
      // Elenco letto da watcher-templates/templates.json (risorsa
      // bundlata con l'app) — `null` selezionato di default: tabella
      // chiave/valore generica, comportamento di sempre.
      templates: [] as ModelloVisualizzazioneInfo[],
      selectedTemplateId: null as string | null,
      name: '',
      nameError: false,
      intervalSeconds: 60,
      showOnTimeline: false,

      expertName: '',
      expertNameError: false,
      expertCommand: '',
      expertCommandError: false,
      expertBucketId: '',

      confirmFolderPath: '',
      confirmContractText: '',
      confirmMode: 'simple' as 'simple' | 'expert',
      confirmWatcherId: '',
    };
  },
  async mounted() {
    try {
      this.templates = await invoke<ModelloVisualizzazioneInfo[]>(
        'elenca_modelli_visualizzazione_watcher'
      );
    } catch {
      this.templates = [];
    }
  },
  methods: {
    // Valida il nome e passa allo step di scelta dimensione, invece di
    // creare subito il watcher — richiesta esplicita: la dimensione del
    // modulo Home va scelta PRIMA che il watcher venga effettivamente
    // creato, ma dopo aver compilato il form.
    goToSizeStep() {
      const name = this.name.trim();
      if (!name) {
        this.nameError = true;
        return;
      }
      this.nameError = false;
      this.step = 'size';
    },
    sizeRectStyle(preset: SizePreset) {
      const unit = 36;
      return { width: preset.width * unit + 'px', height: unit + 'px' };
    },
    // Anteprima statica del modello (preview.html, dati d'esempio fissi
    // scritti da chi ha creato il modello — non c'è ancora un bucket
    // reale in questa fase del wizard) — stessa route/servita dallo
    // stesso meccanismo del modello vero (vedi watcher_template_page
    // lato Rust), stessa risoluzione dell'origine già usata da
    // SelectableVisualization.vue per il modello vero.
    templatePreviewSrc(modello: ModelloVisualizzazioneInfo) {
      let origin = document.location.origin;
      if (document.location.port === '27180') {
        origin = 'http://localhost:5666';
      }
      return origin + '/pages/watcher-templates/' + modello.id + '/preview.html';
    },
    // Nome del modello nella lingua attuale dell'app — templates.json
    // porta entrambe le traduzioni fin dalla risorsa bundlata, così
    // aggiungere un nuovo modello resta "solo file + json", nessun
    // codice da toccare per la traduzione.
    nomeModello(modello: ModelloVisualizzazioneInfo) {
      return this.$i18n.locale === 'it' ? modello.nome_it : modello.nome_en;
    },
    // Se l'anteprima del modello è più alta del riquadro di default
    // (es. "Classifica" con più righe), il riquadro si allarga per
    // mostrarla per intero invece di tagliarla — richiesta esplicita.
    // Stesso documento/origine dell'iframe (verificato più sopra),
    // quindi leggerne l'altezza reale del contenuto è sempre permesso.
    onPreviewLoad(event: Event) {
      try {
        const iframe = event.target as HTMLIFrameElement;
        const doc = iframe.contentDocument || iframe.contentWindow?.document;
        if (!doc) return;
        const altezzaContenuto = doc.body.scrollHeight;
        if (altezzaContenuto > 40) {
          iframe.style.height = Math.min(altezzaContenuto, 200) + 'px';
        }
      } catch (e) {
        // L'anteprima resta alla dimensione di default.
      }
    },
    async submitSimple() {
      const name = this.name.trim();
      if (!name) {
        this.nameError = true;
        return;
      }
      this.nameError = false;

      let folderPath = '';
      try {
        folderPath = await invoke<string>('crea_watcher_personalizzato_semplice', {
          nome: name,
          intervalloSecondi: this.intervalSeconds,
          mostraTimeline: this.showOnTimeline,
          larghezzaGriglia: this.selectedSize.width,
          modelloId: this.selectedTemplateId,
        });
      } catch (e) {
        this.nameError = true;
        return;
      }
      // Fa partire subito il processo del watcher appena creato — senza
      // questo resterebbe scritto solo su disco, in attesa del prossimo
      // riavvio dell'app (spawn_custom_watchers gira anche in .setup()).
      try {
        await invoke('ricarica_watcher_personalizzati');
      } catch {
        // Non bloccante: la cartella/manifest esistono comunque, partirà
        // al prossimo riavvio se questo tentativo fallisce.
      }

      // Crea subito il modulo in Home per questo watcher, alla
      // dimensione scelta nello step precedente — richiesta esplicita:
      // niente passaggio manuale da "+ Aggiungi modulo", il modulo
      // (vuoto finché non arrivano dati) compare da solo.
      try {
        const id = folderPath.split(/[\\/]/).filter(Boolean).pop() || '';
        const lista = await invoke<{ id: string; bucket_id: string | null }[]>(
          'elenca_watcher_personalizzati'
        );
        const bucketId = (lista || []).find(w => w.id === id)?.bucket_id;
        if (bucketId) {
          const viewsStore = useViewsStore();
          await viewsStore.load();
          const view = viewsStore.views[0];
          if (view) {
            viewsStore.addVisualizationWithProps({
              view_id: view.id,
              type: 'custom_watcher_view',
              props: {
                bucketId,
                title: name,
                gridWidth: this.selectedSize.width,
                templateId: this.selectedTemplateId || undefined,
              },
            });
            await viewsStore.save();
          }
        }
      } catch (e) {
        // Non bloccante: il watcher è comunque creato, l'utente può
        // sempre aggiungere il modulo a mano da "+ Aggiungi modulo".
        console.error(e);
      }

      this.confirmWatcherId = folderPath.split(/[\\/]/).filter(Boolean).pop() || '';
      this.confirmFolderPath = folderPath;
      this.confirmContractText = this.$t('customModuleWizard.simpleContract') as string;
      this.confirmMode = 'simple';
      this.step = 'confirm';
    },
    async submitExpert() {
      const name = this.expertName.trim();
      const command = this.expertCommand.trim();
      this.expertNameError = !name;
      this.expertCommandError = !command;
      if (!name || !command) return;

      let folderPath = '';
      try {
        folderPath = await invoke<string>('crea_watcher_personalizzato_esperto', {
          nome: name,
          comando: command,
          bucketId: this.expertBucketId.trim() || null,
        });
      } catch (e) {
        this.expertCommandError = true;
        return;
      }
      try {
        await invoke('ricarica_watcher_personalizzati');
      } catch {
        // Non bloccante: partirà al prossimo riavvio se questo tentativo fallisce.
      }

      this.confirmWatcherId = folderPath.split(/[\\/]/).filter(Boolean).pop() || '';
      this.confirmFolderPath = folderPath;
      this.confirmContractText = this.$t('customModuleWizard.expertContract') as string;
      this.confirmMode = 'expert';
      this.step = 'confirm';
    },
    async openFolder() {
      try {
        await invoke('apri_cartella_custom_watcher', { id: this.confirmWatcherId });
      } catch {
        // Non bloccante: l'utente vede comunque il percorso a schermo.
      }
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

.mode-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
}

.mode-card {
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  padding: 12px;
  cursor: pointer;
  background-color: var(--color-surface2);
}

.mode-card:hover {
  filter: brightness(1.1);
}

.size-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
  margin-top: 12px;
}

.size-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  padding: 20px 10px;
  border-radius: var(--radius-md);
  border: 1px solid var(--color-border);
  background-color: var(--color-surface2);
  cursor: pointer;
}

.size-card:hover {
  filter: brightness(1.1);
}

.size-card-selected {
  border-color: var(--color-accent1);
}

.size-rect {
  background-color: var(--color-accent1);
  border-radius: var(--radius-sm);
  opacity: 0.5;
}

.size-card-selected .size-rect {
  opacity: 1;
}

.size-card-label {
  font-size: var(--font-size-xs);
  color: var(--color-text-dim);
  text-align: center;
  max-width: 90px;
}

.mode-card-title {
  font-weight: var(--font-weight-semibold);
  color: var(--color-text);
  margin-bottom: 4px;
  font-size: var(--font-size-sm);
}

.mode-card-desc {
  font-size: var(--font-size-xs);
  color: var(--color-text-faint);
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

.template-grid {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin-top: 12px;
}

.template-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 10px 14px;
  border-radius: var(--radius-md);
  border: 1px solid var(--color-border);
  background-color: var(--color-surface2);
  cursor: pointer;
}

.template-card:hover {
  filter: brightness(1.1);
}

.template-card-selected {
  border-color: var(--color-accent1);
}

.template-card-name {
  font-size: var(--font-size-sm);
  color: var(--color-text);
  flex-shrink: 0;
}

.template-card-preview {
  width: 140px;
  height: 40px;
  border: none;
  pointer-events: none;
  flex-shrink: 0;
}

.template-card-preview-generic {
  display: flex;
  flex-direction: column;
  justify-content: center;
  gap: 4px;
  padding: 0 6px;
}

.template-generic-row {
  height: 6px;
  border-radius: 3px;
  background-color: var(--color-border);
}

.toggle-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 0;
  border-top: 1px solid var(--color-border);
  font-size: var(--font-size-sm);
  color: var(--color-text);
}

.settings-toggle {
  width: 36px;
  height: 20px;
  border-radius: var(--radius-pill);
  background-color: var(--color-surface2);
  border: 1px solid var(--color-border);
  position: relative;
  cursor: pointer;
  flex-shrink: 0;
}

.settings-toggle-thumb {
  position: absolute;
  top: 1px;
  left: 1px;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background-color: var(--color-text-faint);
  transition: left 0.15s ease, background-color 0.15s ease;
}

.settings-toggle-on {
  background-color: var(--color-accent1);
}

.settings-toggle-on .settings-toggle-thumb {
  left: 17px;
  background-color: #241a12;
}

.confirm-box {
  border-radius: var(--radius-md);
  padding: 12px;
  background-color: var(--color-surface2);
  margin-bottom: 10px;
}

.confirm-box-title {
  font-weight: var(--font-weight-semibold);
  color: var(--color-text);
  font-size: var(--font-size-sm);
  margin-bottom: 4px;
}

.confirm-box-path {
  font-size: var(--font-size-xs);
  color: var(--color-text-dim);
  word-break: break-all;
}

.confirm-json {
  background-color: var(--color-surface2);
  border-radius: var(--radius-md);
  padding: 10px;
  font-size: var(--font-size-xs);
  color: var(--color-text);
  white-space: pre-wrap;
  word-break: break-all;
}

.expert-contract-hint {
  margin-top: 10px;
}
</style>

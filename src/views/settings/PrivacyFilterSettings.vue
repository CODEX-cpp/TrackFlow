<template lang="pug">
div
  div.settings-row
    div
      div.settings-row-title {{ $t('settings.privacyFilters.title') }}
      div.settings-row-help {{ $t('settings.privacyFilters.help') }}
    div.settings-actions
      div.pill-btn-ghost(@click="resetEditor()" :class="{ 'pill-btn-disabled': !hasUnsavedChanges || isSaving }") {{ $t('settings.privacyFilters.discard') }}
      div.pill-btn(@click="canSave && savePrivacyFilters()" :class="{ 'pill-btn-disabled': !canSave }") {{ $t('settings.privacyFilters.save') }}

  div.settings-alert.settings-alert-danger(v-if="saveError !== ''") {{ saveError }}

  div.settings-alert.settings-alert-danger(v-if="validationErrors.length > 0")
    div(v-for="error in validationErrors" :key="error") {{ error }}

  div.settings-alert.settings-alert-warning(v-if="hasUnsavedChanges && validationErrors.length === 0")
    | {{ $t('settings.privacyFilters.unsavedChanges') }}

  div.pf-layout
    div.pf-editor-col
      textarea.settings-field.settings-textarea(
        v-model="editorText"
        rows="14"
        :class="{ 'settings-field-invalid': editorState === false }"
      )
      div.settings-row-help.pf-editor-hint {{ $t('settings.privacyFilters.disableHint') }}
      div.settings-row-help.pf-editor-hint {{ $t('settings.privacyFilters.regexHint') }}

    div.pf-guide-col(v-if="showGuide")
      div.pf-guide-markdown(v-html="guideHtml")

  div.pf-toolbar
    div.pill-btn-ghost(@click="showGuide = !showGuide" :class="{ 'pf-guide-toggle-active': showGuide }")
      icon(name="question")
      |  {{ $t('settings.privacyFilters.infoButton') }}
</template>

<script lang="ts">
import 'vue-awesome/icons/question';
import { marked } from 'marked';
import DOMPurify from 'dompurify';

import { useSettingsStore } from '~/stores/settings';
import { formatPrivacyFilters, validatePrivacyFiltersInput } from '~/util/privacyFilters';

export default {
  name: 'PrivacyFilterSettings',
  data() {
    return {
      settingsStore: useSettingsStore(),
      editorText: '',
      savedText: '',
      saveError: '',
      isSaving: false,
      showGuide: false,
    };
  },
  computed: {
    validationResult() {
      return validatePrivacyFiltersInput(this.editorText);
    },
    validationErrors() {
      return this.validationResult.errors;
    },
    canSave() {
      return this.hasUnsavedChanges && this.validationErrors.length === 0 && !this.isSaving;
    },
    hasUnsavedChanges() {
      return this.editorText !== this.savedText;
    },
    editorState() {
      if (!this.hasUnsavedChanges) {
        return null;
      }
      return this.validationErrors.length === 0;
    },
    // Guida scritta in Markdown (settings.privacyFilters.guideMarkdown) e
    // renderizzata qui — stessa coppia marked+DOMPurify già usata da
    // AiChatWidget.vue per lo stesso motivo (v-html va sempre sanitizzato).
    // Qui il testo viene dai file di traduzione (contenuto nostro, non da
    // un'API esterna), ma passarlo comunque da DOMPurify costa nulla ed
    // evita di doverci ricordare di riconsiderarlo se in futuro diventasse
    // configurabile.
    guideHtml(): string {
      const markdown = this.$t('settings.privacyFilters.guideMarkdown') as string;
      const html = marked.parse(markdown, { async: false }) as string;
      return DOMPurify.sanitize(html);
    },
  },
  watch: {
    'settingsStore.privacy_filters': {
      deep: true,
      handler() {
        this.syncFromStore(false);
      },
    },
  },
  mounted() {
    this.syncFromStore(true);
  },
  methods: {
    syncFromStore(overwriteEditor: boolean) {
      const serialized = formatPrivacyFilters(this.settingsStore.privacy_filters);
      if (overwriteEditor || this.editorText === this.savedText) {
        this.editorText = serialized;
      }
      this.savedText = serialized;
    },
    resetEditor() {
      if (!this.hasUnsavedChanges || this.isSaving) return;
      this.saveError = '';
      this.syncFromStore(true);
    },
    async savePrivacyFilters() {
      if (!this.canSave) {
        return;
      }

      this.isSaving = true;
      this.saveError = '';
      try {
        await this.settingsStore.update({
          privacy_filters: this.validationResult.rules || [],
        });
        this.syncFromStore(true);
      } catch (error) {
        this.saveError = error instanceof Error ? error.message : String(error);
      } finally {
        this.isSaving = false;
      }
    },
  },
};
</script>

<style scoped>
.settings-actions {
  display: flex;
  gap: 8px;
  flex-shrink: 0;
}

.pf-toolbar {
  display: flex;
  justify-content: flex-end;
  margin-top: 14px;
}

.pf-guide-toggle-active {
  border-color: var(--color-accent1);
  color: var(--color-accent1);
}

.settings-textarea {
  display: block;
  width: 100%;
  font-family: 'SFMono-Regular', Consolas, monospace;
  resize: vertical;
}

.settings-field-invalid {
  border-color: #d9534f;
}

.settings-alert {
  margin-top: 12px;
  padding: 10px 14px;
  border-radius: var(--radius-md);
  border: 1px solid var(--color-border);
  font-size: var(--font-size-sm);
}

.settings-alert-danger {
  background-color: rgba(217, 83, 79, 0.15);
  color: var(--color-text);
}

.settings-alert-warning {
  background-color: rgba(211, 163, 85, 0.15);
  color: var(--color-text);
}

.pf-layout {
  display: flex;
  gap: 20px;
  margin-top: 12px;
  align-items: flex-start;
}

.pf-editor-col {
  flex: 1 1 0;
  min-width: 0;
}

.pf-editor-hint {
  margin-top: 6px;
}

.pf-guide-col {
  flex: 1 1 0;
  min-width: 0;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  padding: 14px 18px;
  background-color: var(--color-surface2);
  max-height: 480px;
  overflow-y: auto;
  font-size: var(--font-size-sm);
}

/* Vue 2: ::v-deep, non :deep() — v-html inietta markup che non porta
   l'attributo dello scoping, stesso pattern già usato in
   AiChatWidget.vue per lo stesso identico problema (HTML da marked). */
.pf-guide-markdown ::v-deep h2 {
  margin: 0 0 10px 0;
  font-size: var(--font-size-md);
  color: var(--color-text);
}

.pf-guide-markdown ::v-deep h3 {
  margin: 18px 0 8px 0;
  font-size: var(--font-size-sm);
  color: var(--color-text);
}

.pf-guide-markdown ::v-deep h3:first-child {
  margin-top: 0;
}

.pf-guide-markdown ::v-deep p {
  margin: 0 0 10px 0;
  color: var(--color-text-dim);
  line-height: 1.5;
}

.pf-guide-markdown ::v-deep ul {
  margin: 0 0 10px 0;
  padding-left: 18px;
  color: var(--color-text-dim);
  line-height: 1.6;
}

.pf-guide-markdown ::v-deep li {
  margin-bottom: 4px;
}

.pf-guide-markdown ::v-deep strong {
  color: var(--color-text);
  font-weight: var(--font-weight-semibold, 600);
}

.pf-guide-markdown ::v-deep code {
  font-family: 'SFMono-Regular', Consolas, 'Liberation Mono', Menlo, monospace;
  font-size: 0.9em;
  background-color: var(--color-surface);
  border-radius: var(--radius-sm);
  padding: 1px 5px;
  color: var(--color-text-dim);
}

.pf-guide-markdown ::v-deep pre {
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  padding: 10px 12px;
  overflow-x: auto;
  margin: 0 0 10px 0;
}

.pf-guide-markdown ::v-deep pre code {
  background: none;
  padding: 0;
  font-size: var(--font-size-xs);
}
</style>

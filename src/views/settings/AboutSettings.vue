
<template lang="pug">
div
  div.about-header
    img.about-logo(:src="logoUrl" alt="TrackFlow")
    div
      div.about-title TrackFlow
      div.about-version(v-if="versione") {{ $t('settings.about.versionPrefix') }} {{ versione }}

  div.about-changelog-title {{ $t('settings.about.changelogTitle') }}
  div.settings-row-help(v-if="caricamentoFallito") {{ $t('settings.about.changelogError') }}
  div.about-changelog(v-else v-html="changelogHtml")
</template>

<script lang="ts">
import { marked } from 'marked';
import DOMPurify from 'dompurify';
import { invoke } from '@tauri-apps/api/core';
import { getVersion } from '@tauri-apps/api/app';
import logoUrl from '~/assets/logo.png';

marked.setOptions({ breaks: true });

export default {
  name: 'AboutSettings',
  data() {
    return {
      logoUrl,
      versione: '',
      changelogHtml: '',
      caricamentoFallito: false,
    };
  },
  async mounted(this: any) {
    try {
      this.versione = await getVersion();
    } catch (e) {
      // Fuori da Tauri (dev server puro nel browser) — stesso pattern
      // già usato altrove (vedi CategorizationSettings.vue).
    }
    try {
      const markdown = await invoke<string>('leggi_changelog', {
        versione: this.versione,
        lingua: this.$i18n.locale,
      });
      // Stesso trattamento già usato per il markdown dell'agente AI
      // (AiChatWidget.vue): parsato con marked, sanitizzato con
      // DOMPurify prima di iniettarlo con v-html — il file viene da un
      // pacchetto scaricato, non va mai trattato come HTML sicuro senza
      // passarci prima.
      const html = marked.parse(markdown, { async: false }) as string;
      this.changelogHtml = DOMPurify.sanitize(html);
    } catch (e) {
      this.caricamentoFallito = true;
    }
  },
};
</script>

<style scoped>
.about-header {
  display: flex;
  align-items: center;
  gap: 14px;
  margin-bottom: 20px;
}

.about-logo {
  width: 48px;
  height: 48px;
  object-fit: contain;
}

.about-title {
  font-size: var(--font-size-xl);
  font-weight: var(--font-weight-bold);
  color: var(--color-text);
}

.about-version {
  font-size: var(--font-size-sm);
  color: var(--color-text-faint);
}

.about-changelog-title {
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-semibold);
  color: var(--color-text);
  margin-bottom: 8px;
}

.about-changelog {
  font-size: var(--font-size-sm);
  color: var(--color-text-dim);
  line-height: 1.6;
}

.about-changelog :deep(h1),
.about-changelog :deep(h2) {
  font-size: var(--font-size-base);
  font-weight: var(--font-weight-semibold);
  color: var(--color-text);
  margin: 16px 0 6px 0;
}

.about-changelog :deep(h1:first-child),
.about-changelog :deep(h2:first-child) {
  margin-top: 0;
}

.about-changelog :deep(ul) {
  margin: 0 0 8px 0;
  padding-left: 20px;
}

.about-changelog :deep(li) {
  margin: 2px 0;
}

.about-changelog :deep(code) {
  background-color: var(--color-surface2);
  border-radius: var(--radius-sm);
  padding: 1px 5px;
  font-family: 'SFMono-Regular', Consolas, 'Liberation Mono', Menlo, monospace;
  font-size: var(--font-size-xs);
}
</style>

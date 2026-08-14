<template lang="pug">
div
  div.settings-row
    div
      div.settings-row-title {{ $t('settings.activePattern.title') }}
      div.settings-row-help {{ $t('settings.activePattern.help') }}

  div.ap-controls
    input.settings-field.ap-search-input(
      v-model="filtroApp"
      type="text"
      :placeholder="$t('settings.activePattern.searchPlaceholder')"
    )

  div(v-if="caricandoApp") {{ $t('settings.activePattern.loading') }}
  div.settings-row-help(v-else-if="appConosciute.length === 0") {{ $t('settings.activePattern.noApps') }}
  div.settings-row-help(v-else-if="appFiltrate.length === 0") {{ $t('settings.activePattern.noAppsMatch') }}
  div.ap-app-list.themed-scroll(v-else)
    label.ap-app-row(v-for="a in appFiltrate" :key="a.app")
      input.ap-app-checkbox.custom-checkbox(type="checkbox" :checked="isAlwaysActive(a.app)" @change="toggleApp(a.app)")
      img.ap-app-icon(
        v-if="!iconFalliti[a.app]"
        :src="iconUrlForApp(a.app)"
        @error="iconFalliti = { ...iconFalliti, [a.app]: true }"
        alt=""
      )
      span.ap-app-icon-fallback(v-else) {{ fallbackIconForApp(a.app) }}
      div.ap-app-names
        div.ap-app-name {{ a.nome_leggibile || a.app }}
        div.ap-app-raw-name(v-if="a.nome_leggibile") {{ a.app }}
</template>

<script lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { useSettingsStore } from '~/stores/settings';
import { iconUrlForApp, fallbackIconForApp } from '~/util/appNames';

interface AppConosciuta {
  app: string;
  nome_leggibile: string | null;
}

export default {
  name: 'ActivePatternSettings',
  data() {
    return {
      settingsStore: useSettingsStore(),
      appConosciute: [] as AppConosciuta[],
      caricandoApp: true,
      filtroApp: '',
      iconFalliti: {} as Record<string, boolean>,
    };
  },
  computed: {
    appFiltrate(): AppConosciuta[] {
      const query = this.filtroApp.trim().toLowerCase();
      if (!query) return this.appConosciute;
      return this.appConosciute.filter(
        a =>
          a.app.toLowerCase().includes(query) ||
          (a.nome_leggibile || '').toLowerCase().includes(query)
      );
    },
  },
  async mounted() {
    try {
      this.appConosciute = await invoke<AppConosciuta[]>('elenca_app_conosciute');
    } catch (e) {
      // L'app potrebbe girare fuori da Tauri durante lo sviluppo web puro
      // (npx vite senza il guscio nativo) — invoke() non esiste in quel
      // caso, non è un errore da mostrare all'utente. Stesso pattern già
      // usato in CategorizationSettings.vue/AiAgentSettings.vue.
    } finally {
      this.caricandoApp = false;
    }
  },
  methods: {
    iconUrlForApp,
    fallbackIconForApp,
    isAlwaysActive(app: string): boolean {
      return this.settingsStore.always_active_apps.includes(app);
    },
    toggleApp(app: string) {
      const current = this.settingsStore.always_active_apps;
      const next = current.includes(app) ? current.filter(a => a !== app) : [...current, app];
      this.settingsStore.update({ always_active_apps: next });
    },
  },
};
</script>

<style scoped>
.ap-controls {
  display: flex;
  margin-top: 4px;
  margin-bottom: 12px;
}

.ap-search-input {
  width: 260px;
}

.ap-app-list {
  max-height: 420px;
  overflow-y: auto;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
}

.ap-app-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  border-bottom: 1px solid var(--color-border);
  cursor: pointer;
}

.ap-app-row:last-child {
  border-bottom: none;
}

.ap-app-row:hover {
  background-color: var(--color-surface2);
}

.ap-app-checkbox {
  flex-shrink: 0;
  cursor: pointer;
}

.ap-app-icon,
.ap-app-icon-fallback {
  width: 24px;
  height: 24px;
  flex-shrink: 0;
  border-radius: var(--radius-sm);
  object-fit: contain;
}

.ap-app-icon-fallback {
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 15px;
}

.ap-app-names {
  flex: 1;
  min-width: 0;
}

.ap-app-name {
  font-size: var(--font-size-sm);
  color: var(--color-text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.ap-app-raw-name {
  font-size: var(--font-size-xs);
  color: var(--color-text-faint);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>

<template lang="pug">
div
  div.settings-row
    div
      div.settings-row-title {{ $t('settings.screenshot.intervalTitle') }}
      div.settings-row-help {{ $t('settings.screenshot.intervalHelp') }}
    div.settings-range
      input.settings-field(type="number" min="5" v-model.number="screenshotIntervalSeconds" style="width: 90px;")
      span.settings-row-help {{ $t('settings.screenshot.seconds') }}

  div.settings-row
    div
      div.settings-row-title {{ $t('settings.screenshot.retentionTitle') }}
      div.settings-row-help {{ $t('settings.screenshot.retentionHelp') }}
    div.settings-range
      input.settings-field(type="number" min="1" v-model.number="screenshotRetentionDays" style="width: 90px;")
      span.settings-row-help {{ $t('settings.screenshot.days') }}

  div.settings-row
    div
      div.settings-row-title {{ $t('settings.screenshot.onlyActiveWindowTitle') }}
      div.settings-row-help {{ $t('settings.screenshot.onlyActiveWindowHelp') }}
    div.settings-toggle(:class="{ 'settings-toggle-on': screenshotOnlyActiveWindow }" @click="screenshotOnlyActiveWindow = !screenshotOnlyActiveWindow")
      div.settings-toggle-thumb

  div.settings-row
    div
      div.settings-row-title {{ $t('settings.screenshot.manageTitle') }}
      div.settings-row-help
        | {{ caricandoDimensione ? $t('settings.screenshot.manageSizeLoading') : $t('settings.screenshot.manageSize', { size: dimensioneFormattata }) }}
    div.settings-screenshot-actions
      div.pill-btn-ghost(@click="apriCartellaScreenshot") {{ $t('settings.screenshot.openFolder') }}
      div.pill-btn-danger(@click="mostraConfermaElimina = true" :class="{ 'pill-btn-disabled': eliminandoTutto }") {{ $t('settings.screenshot.deleteAll') }}

  div.settings-alert.settings-alert-danger(v-if="erroreOperazione") {{ erroreOperazione }}

  confirm-modal(
    v-if="mostraConfermaElimina"
    :title="$t('settings.screenshot.deleteConfirmTitle')"
    :confirm-label="$t('settings.screenshot.deleteConfirm')"
    :cancel-label="$t('common.cancel')"
    @confirm="eliminaTuttoConfermato"
    @cancel="mostraConfermaElimina = false"
  )
    div {{ $t('settings.screenshot.deleteConfirmText') }}
</template>

<script lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { useSettingsStore } from '~/stores/settings';

/// Formattazione leggibile della dimensione in byte restituita da
/// `dimensione_cartella_screenshot` — stessa unità/precisione con cui
/// l'utente ragiona di solito (KB/MB/GB, non byte grezzi).
function formattaByte(byte: number): string {
  if (byte < 1024) return `${byte} B`;
  const unita = ['KB', 'MB', 'GB', 'TB'];
  let valore = byte / 1024;
  let i = 0;
  while (valore >= 1024 && i < unita.length - 1) {
    valore /= 1024;
    i++;
  }
  return `${valore.toFixed(valore < 10 ? 1 : 0)} ${unita[i]}`;
}

export default {
  name: 'ScreenshotSettings',
  components: {
    'confirm-modal': () => import('~/components/ConfirmModal.vue'),
  },
  data() {
    return {
      settingsStore: useSettingsStore(),
      dimensioneBytes: 0,
      caricandoDimensione: true,
      mostraConfermaElimina: false,
      eliminandoTutto: false,
      erroreOperazione: '',
    };
  },
  mounted() {
    this.caricaDimensione();
  },
  computed: {
    dimensioneFormattata(this: any): string {
      return formattaByte(this.dimensioneBytes);
    },
    screenshotIntervalSeconds: {
      get(): number {
        return this.settingsStore.screenshotIntervalSeconds;
      },
      set(value: number) {
        const seconds = Number.isFinite(value) ? Math.max(5, Math.round(value)) : 30;
        this.settingsStore.update({ screenshotIntervalSeconds: seconds });
      },
    },
    screenshotRetentionDays: {
      get(): number {
        return this.settingsStore.screenshotRetentionDays;
      },
      set(value: number) {
        const days = Number.isFinite(value) ? Math.max(1, Math.round(value)) : 14;
        this.settingsStore.update({ screenshotRetentionDays: days });
      },
    },
    screenshotOnlyActiveWindow: {
      get(): boolean {
        return this.settingsStore.screenshotOnlyActiveWindow;
      },
      set(value: boolean) {
        this.settingsStore.update({ screenshotOnlyActiveWindow: value });
      },
    },
  },
  methods: {
    async caricaDimensione(this: any) {
      this.caricandoDimensione = true;
      try {
        this.dimensioneBytes = await invoke<number>('dimensione_cartella_screenshot');
      } catch (e) {
        // Fuori da Tauri (dev server puro nel browser) invoke() non
        // esiste — stesso pattern già usato in CategorizationSettings.vue.
      } finally {
        this.caricandoDimensione = false;
      }
    },
    async apriCartellaScreenshot(this: any) {
      this.erroreOperazione = '';
      try {
        await invoke('apri_cartella_screenshot');
      } catch (e: any) {
        this.erroreOperazione = `${this.$t('settings.screenshot.openFolderError')} ${e?.message ?? e}`;
      }
    },
    async eliminaTuttoConfermato(this: any) {
      this.mostraConfermaElimina = false;
      this.erroreOperazione = '';
      this.eliminandoTutto = true;
      try {
        await invoke('elimina_tutti_screenshot');
        await this.caricaDimensione();
      } catch (e: any) {
        this.erroreOperazione = `${this.$t('settings.screenshot.deleteAllError')} ${e?.message ?? e}`;
      } finally {
        this.eliminandoTutto = false;
      }
    },
  },
};
</script>

<style scoped>
.settings-range {
  display: flex;
  align-items: center;
  gap: 8px;
}

.settings-screenshot-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}
</style>

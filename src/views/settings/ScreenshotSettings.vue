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
</template>

<script lang="ts">
import { useSettingsStore } from '~/stores/settings';

export default {
  name: 'ScreenshotSettings',
  data() {
    return {
      settingsStore: useSettingsStore(),
    };
  },
  computed: {
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
  },
};
</script>

<style scoped>
.settings-range {
  display: flex;
  align-items: center;
  gap: 8px;
}
</style>

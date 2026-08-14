<template lang="pug">
div.settings-row
  div
    div.settings-row-title {{ $t('settings.lunchBreak.title') }}
    div.settings-row-help {{ $t('settings.lunchBreak.help') }}
  div.settings-range
    input.settings-field(type="time" v-model="lunchBreakStart")
    span.settings-range-sep –
    input.settings-field(type="time" v-model="lunchBreakEnd")
</template>

<script lang="ts">
import { useSettingsStore } from '~/stores/settings';

export default {
  name: 'LunchBreakSettings',
  data() {
    return {
      settingsStore: useSettingsStore(),
    };
  },
  computed: {
    lunchBreakStart: {
      get(): string {
        return this.settingsStore.lunchBreakStart;
      },
      set(value: string) {
        this.settingsStore.update({ lunchBreakStart: value });
      },
    },
    lunchBreakEnd: {
      get(): string {
        return this.settingsStore.lunchBreakEnd;
      },
      set(value: string) {
        this.settingsStore.update({ lunchBreakEnd: value });
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

.settings-range-sep {
  color: var(--color-text-faint);
}
</style>

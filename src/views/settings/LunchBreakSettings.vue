<template lang="pug">
div
  div.settings-row
    div
      div.settings-row-title {{ $t('settings.lunchBreak.title') }}
      div.settings-row-help {{ $t('settings.lunchBreak.help') }}
    div.settings-range
      input.settings-field(type="time" v-model="lunchBreakStart")
      span.settings-range-sep –
      input.settings-field(type="time" v-model="lunchBreakEnd")

  div.settings-row
    div
      div.settings-row-title {{ $t('settings.lunchBreak.dailyBudgetTitle') }}
      div.settings-row-help {{ $t('settings.lunchBreak.dailyBudgetHelp') }}
    div.settings-range
      input.settings-field(type="number" min="0.5" step="0.5" v-model.number="dailyWorkHoursBudget" style="width: 90px;")
      span.settings-row-help {{ $t('settings.lunchBreak.hoursUnit') }}
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
    dailyWorkHoursBudget: {
      get(): number {
        return this.settingsStore.dailyWorkHoursBudget;
      },
      set(value: number) {
        const ore = Number.isFinite(value) ? Math.max(0.5, value) : 8;
        this.settingsStore.update({ dailyWorkHoursBudget: ore });
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

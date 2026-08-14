<template lang="pug">
div
  div.settings-row
    div
      div.settings-row-title {{ $t('settings.daystart.startOfDay') }}
      div.settings-row-help {{ $t('settings.daystart.startOfDayHelp') }}
    input.settings-field(type="time" :value="startOfDay" @change="startOfDay = $event.target.value")

  div.settings-row
    div
      div.settings-row-title {{ $t('settings.daystart.startOfWeek') }}
      div.settings-row-help {{ $t('settings.daystart.startOfWeekHelp') }}
    select.settings-field(v-model="startOfWeek")
      option(value="Saturday") {{ $t('settings.daystart.saturday') }}
      option(value="Sunday") {{ $t('settings.daystart.sunday') }}
      option(value="Monday") {{ $t('settings.daystart.monday') }}
</template>
<script lang="ts">
import { useSettingsStore } from '~/stores/settings';

export default {
  name: 'DaystartSettings',
  data() {
    return {
      settingsStore: useSettingsStore(),
    };
  },
  computed: {
    startOfDay: {
      get: function () {
        return this.settingsStore.startOfDay;
      },
      set: function (value) {
        console.log('Set start of day to ' + value);
        this.settingsStore.update({ startOfDay: value });
      },
    },
    startOfWeek: {
      get: function () {
        return this.settingsStore.startOfWeek;
      },
      set: function (value) {
        console.log('Set start of week to ' + value);
        this.settingsStore.update({ startOfWeek: value });
      },
    },
  },
};
</script>

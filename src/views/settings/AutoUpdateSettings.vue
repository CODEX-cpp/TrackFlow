
<template lang="pug">
div
  div.settings-row
    div
      div.settings-row-title {{ $t('settings.updates.autoUpdate') }}
      div.settings-row-help {{ $t('settings.updates.autoUpdateHelp') }}
    div.settings-toggle(:class="{ 'settings-toggle-on': autoUpdateEnabled }" @click="autoUpdateEnabled = !autoUpdateEnabled")
      div.settings-toggle-thumb
</template>

<script lang="ts">
import { useSettingsStore } from '~/stores/settings';

export default {
  name: 'AutoUpdateSettings',
  computed: {
    autoUpdateEnabled: {
      get() {
        return useSettingsStore().autoUpdateEnabled;
      },
      set(autoUpdateEnabled: boolean) {
        useSettingsStore().update({ autoUpdateEnabled });
      },
    },
  },
};
</script>

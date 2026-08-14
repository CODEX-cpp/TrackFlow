
<template lang="pug">
div
  div.settings-row
    div
      div.settings-row-title {{ $t('settings.homeVisibility.hideModules') }}
      div.settings-row-help {{ $t('settings.homeVisibility.hideModulesHelp') }}
    div.settings-toggle(:class="{ 'settings-toggle-on': hideEmptyModules }" @click="hideEmptyModules = !hideEmptyModules")
      div.settings-toggle-thumb

  div.settings-row
    div
      div.settings-row-title {{ $t('settings.homeVisibility.hideTimeline') }}
      div.settings-row-help {{ $t('settings.homeVisibility.hideTimelineHelp') }}
    div.settings-toggle(:class="{ 'settings-toggle-on': hideEmptyTimelineLanes }" @click="hideEmptyTimelineLanes = !hideEmptyTimelineLanes")
      div.settings-toggle-thumb
</template>

<script lang="ts">
import { useSettingsStore } from '~/stores/settings';

// Due impostazioni indipendenti per decidere se nascondere del tutto
// moduli/corsie vuoti nella Home, invece di lasciarli sempre visibili
// col loro stato vuoto. Richiesta esplicita dell'utente: entrando in
// "Modifica moduli" le card nascoste ricompaiono (necessario, per
// poterle rimuovere/spostare), causando uno spostamento percepibile
// delle altre card — questa scelta gli permette di disattivare il
// comportamento invece di doverlo subire, vedi
// SelectableVisualization.vue's `visibile` e
// HomeTimelineSection.vue's rebuildLanes().
export default {
  name: 'HomeVisibilitySettings',
  computed: {
    hideEmptyModules: {
      get() {
        return useSettingsStore().hideEmptyModules;
      },
      set(hideEmptyModules: boolean) {
        useSettingsStore().update({ hideEmptyModules });
      },
    },
    hideEmptyTimelineLanes: {
      get() {
        return useSettingsStore().hideEmptyTimelineLanes;
      },
      set(hideEmptyTimelineLanes: boolean) {
        useSettingsStore().update({ hideEmptyTimelineLanes });
      },
    },
  },
};
</script>

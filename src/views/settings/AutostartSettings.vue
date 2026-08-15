
<template lang="pug">
div
  div.settings-row
    div
      div.settings-row-title {{ $t('settings.autostart.title') }}
      div.settings-row-help {{ $t('settings.autostart.help') }}
    div.settings-toggle(:class="{ 'settings-toggle-on': abilitato }" @click="alterna")
      div.settings-toggle-thumb
</template>

<script lang="ts">
import { invoke } from '@tauri-apps/api/core';

// Stato reale in HKCU\...\Run (vedi src-tauri/src/autostart.rs), non in
// settingsStore — letto/scritto dal vivo via invoke, non persistito
// insieme al resto delle impostazioni.
export default {
  name: 'AutostartSettings',
  data() {
    return {
      abilitato: false,
    };
  },
  async mounted(this: any) {
    try {
      this.abilitato = await invoke<boolean>('avvio_automatico_abilitato');
    } catch (e) {
      // Fuori da Tauri (dev server puro nel browser) — stesso pattern
      // già usato altrove (vedi CategorizationSettings.vue).
    }
  },
  methods: {
    async alterna(this: any) {
      const nuovo = !this.abilitato;
      try {
        await invoke('imposta_avvio_automatico', { abilita: nuovo });
        this.abilitato = nuovo;
      } catch (e) {
        // Fuori da Tauri, o scrittura registro fallita — stato non
        // aggiornato, il toggle resta al valore precedente.
      }
    },
  },
};
</script>

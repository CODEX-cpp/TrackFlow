<template lang="pug">
div(v-if="visible")
  div.modal-backdrop.first-run-backdrop
  div.edit-modal.first-run-modal
    div.edit-modal-title {{ $t('firstRunSetup.title') }}
    p.first-run-subtitle {{ $t('firstRunSetup.subtitle') }}

    label.first-run-checkbox(v-for="w in watchers" :key="w.name")
      input(type="checkbox" v-model="enabled[w.name]")
      span {{ $t(`firstRunSetup.modules.${w.name}`) }}

    div.edit-modal-actions
      div.pill-btn(@click="confirm") {{ $t('firstRunSetup.confirm') }}
</template>

<style lang="scss" scoped>
@import '../style/theme.css';
@import '../style/modals.css';

.first-run-modal {
  width: 380px;
}

// Copre solo l'area FUORI da .app-shell (in pratica il bordo attorno,
// visto che il popup è centrato sull'intera finestra) — sidebar e
// topbar vengono sfocate/scurite a parte, direttamente in App.vue
// (.first-run-blur su .app-shell): un overlay fixed qui non le
// raggiungerebbe comunque, perché usano position:sticky con un proprio
// contesto di rendering che uno z-index più alto su un elemento esterno
// non riesce a sovrastare (visto empiricamente: persino z-index:999 non
// bastava). Resta comunque opaco, non solo per coerenza visiva ma
// perché qualcosa deve pur riempire lo sfondo dietro alla card stessa.
.first-run-backdrop {
  background-color: #000000;
}

.first-run-subtitle {
  font-size: var(--font-size-sm);
  color: var(--color-text-faint);
  margin: -8px 0 16px;
}

.first-run-checkbox {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 0;
  font-size: var(--font-size-sm);
  color: var(--color-text);
  cursor: pointer;

  input[type='checkbox'] {
    accent-color: var(--color-accent1);
    width: 16px;
    height: 16px;
    flex-shrink: 0;
  }
}
</style>

<script lang="ts">
// Popup mostrato una sola volta, al primissimo avvio dell'app dopo
// l'installazione: lascia scegliere subito quali watcher tenere accesi,
// invece di dover andare a spegnerli uno per uno dal menu Moduli dopo
// aver scoperto che partono tutti di default (vedi lib.rs's
// load_modules_config()). Non blocca il caricamento della Home dietro
// di sé — semplice overlay sopra, coerente con come l'utente ha chiesto
// esplicitamente questo comportamento (vede già la sua Home, sfumata,
// mentre sceglie).
//
// Elenco/etichette qui sotto — solo i moduli che sono DAVVERO un
// eseguibile sidecar installato (stesso set di externalBin in
// tauri.conf.json, 9 voci) — non includono VoiSpeed/Categorizzazione AI
// (nessun processo da avviare, non hanno senso in un popup "quali
// watcher avviare"), a differenza del menu Moduli della tray che invece
// li elenca entrambi.
import { invoke } from '@tauri-apps/api/core';
import { useFirstRunStore } from '~/stores/firstRun';

const WATCHER_NAMES = [
  'aw-watcher-afk',
  'aw-watcher-window',
  'aw-watcher-vpn',
  'aw-watcher-claude-code',
  'aw-watcher-screenshot',
  'aw-watcher-app-icons',
  'aw-watcher-tray',
  'aw-watcher-vscode',
  'aw-watcher-excel',
] as const;

export default {
  name: 'FirstRunWatcherSetup',
  data(this: any) {
    return {
      firstRunStore: useFirstRunStore(),
      watchers: WATCHER_NAMES.map(name => ({ name })),
      enabled: {
        'aw-watcher-afk': true,
        'aw-watcher-window': true,
        // Spenti di default — stessi motivi del default lato Rust, vedi
        // load_modules_config() in lib.rs: VPN/Claude Code/Excel non
        // rilevanti per la maggior parte di chi installa per la prima
        // volta, la lettura delle icone nascoste della tray non è
        // ancora affidabile.
        'aw-watcher-vpn': false,
        'aw-watcher-claude-code': false,
        'aw-watcher-screenshot': true,
        'aw-watcher-app-icons': true,
        'aw-watcher-tray': false,
        'aw-watcher-vscode': true,
        'aw-watcher-excel': false,
      } as Record<string, boolean>,
    };
  },
  computed: {
    visible(this: any) {
      return this.firstRunStore.visible;
    },
  },
  async mounted(this: any) {
    try {
      const giaConfigurati = await invoke('moduli_gia_configurati');
      this.firstRunStore.visible = !giaConfigurati;
    } catch (e) {
      // Fuori da Tauri (dev server puro nel browser) invoke() non
      // esiste — stesso pattern già usato altrove (vedi
      // CategorizationSettings.vue): nessun popup, non c'è nulla da
      // avviare/fermare comunque.
    }
  },
  methods: {
    async confirm(this: any) {
      // L'app è già partita con i moduli di default attivi (vedi
      // load_modules_config()) — questo comando si occupa di fermare
      // quelli deselezionati qui, non solo di confermare quelli già
      // in esecuzione.
      await invoke('imposta_moduli_iniziali', { moduli: this.enabled });
      this.firstRunStore.visible = false;
    },
  },
};
</script>

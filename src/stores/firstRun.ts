import { defineStore } from 'pinia';

// Stato condiviso minimo tra FirstRunWatcherSetup.vue (che decide se il
// popup va mostrato) e App.vue (che deve sfocare/scurire .app-shell,
// sidebar e topbar comprese, mentre il popup è aperto — vedi commento
// in App.vue sul perché un overlay fixed da solo non bastava: sidebar/
// topbar usano position:sticky con un proprio contesto di rendering,
// invisibile a un overlay esterno indipendentemente dallo z-index).
export const useFirstRunStore = defineStore('firstRun', {
  state: () => ({
    visible: false,
  }),
});

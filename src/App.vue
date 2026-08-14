<template lang="pug">
div#wrapper(v-if="loaded")
  div.app-shell(:class="{ 'first-run-blur': firstRunStore.visible }")
    aw-sidebar
    div.app-main.themed-scroll
      aw-topbar
      //- Home renders entirely via these three components, already
      //- themed via --color-bg — no router-view at all here (the old
      //- Bootstrap Activity.vue/ActivityView.vue page that used to sit
      //- below them has been removed, explicit request). The route
      //- still exists (see route.js) purely so /activity/:host/... URLs
      //- keep resolving; its component is never rendered on this branch.
      template(v-if="isHomePage")
        home-timeline-section
        home-modules-section
        active-projects-section
      //- Everything else not yet redesigned (Impostazioni, Timeline, Raw
      //- Data, ...) still lives in the old dark.css-styled container.
      template(v-else-if="bareLayout")
        error-boundary
          router-view
      template(v-else)
        div(:class="{'container': !fullContainer, 'container-fluid': fullContainer}").px-0.px-md-2
          div.aw-container.my-sm-3.p-3
            error-boundary
              router-view

  //- Fuori da .app-shell apposta: usa position:fixed relativo al
  //- viewport, non a un contenitore con scroll proprio (vedi la sticky
  //- topbar in Topbar.vue per lo stesso genere di bug già trovato una
  //- volta con .app-main).
  ai-chat-widget
  first-run-watcher-setup
</template>

<script lang="ts">
import { useSettingsStore } from '~/stores/settings';
import { useServerStore } from '~/stores/server';
import { useFirstRunStore } from '~/stores/firstRun';
import { detectPreferredTheme } from '~/util/theme';
import { valutaRegoleNotifica } from '~/util/notifyRulesEngine';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export default {
  components: {
    'active-projects-section': () => import('~/components/ActiveProjectsSection.vue'),
    'home-timeline-section': () => import('~/components/HomeTimelineSection.vue'),
    'home-modules-section': () => import('~/components/HomeModulesSection.vue'),
    'ai-chat-widget': () => import('~/components/AiChatWidget.vue'),
    'first-run-watcher-setup': () => import('~/components/FirstRunWatcherSetup.vue'),
  },
  data: function () {
    return {
      activityViews: [],
      loaded: false,
      firstRunStore: useFirstRunStore(),
    };
  },

  computed: {
    fullContainer(this: any) {
      return this.$route.meta.fullContainer;
    },
    bareLayout(this: any) {
      return this.$route.meta.bareLayout;
    },
    isHomePage(this: any) {
      return this.$route.path.startsWith('/activity');
    },
  },

  async beforeCreate(this: any) {
    // Get Theme From LocalStorage
    const settingsStore = useSettingsStore();
    await settingsStore.ensureLoaded();
    const theme = settingsStore.theme;
    const detectedTheme = theme === 'auto' ? detectPreferredTheme() : theme;
    document.body.classList.toggle('theme-dark', detectedTheme === 'dark');
    document.body.classList.toggle('theme-light', detectedTheme !== 'dark');
    this.loaded = true;

    // Se il toggle DevTools (Impostazioni → Sviluppatore) è rimasto
    // acceso dall'ultima sessione, riapre subito i DevTools all'avvio —
    // stesso comportamento di quando lo si accende a mano dalle
    // Impostazioni, vedi DeveloperSettings.vue.
    if (settingsStore.developerModeEnabled && settingsStore.devtoolsEnabled) {
      try {
        await invoke('apri_devtools');
      } catch (e) {
        // Fuori da Tauri (dev server puro nel browser) invoke() non
        // esiste — stesso pattern già usato altrove (vedi
        // CategorizationSettings.vue).
      }
    }
  },

  mounted: async function (this: any) {
    const serverStore = useServerStore();
    await serverStore.getInfo();

    // Emesso dal backend Rust quando rileva una sessione VPN con
    // cliente non mappato (vedi vpn_notify.rs) — la notifica di sistema
    // parte in parallelo, ma il click su di essa (comportamento di
    // default di Windows, non personalizzabile senza pacchettizzare
    // l'app in MSIX con un vero activator) porta solo l'app in primo
    // piano, senza poter indicare a QUALE pagina navigare. Navigando
    // subito qui, appena l'evento arriva — non solo al click — quando
    // l'utente riporta in primo piano TrackFlow (dal click sulla
    // notifica, dalla tray, o da sé) si trova già sulla pagina giusta.
    try {
      await listen<string>('vpn-notifica-apri-impostazioni', event => {
        this.$router.push(`/settings/${event.payload}`);
      });
    } catch (e) {
      // Fuori da Tauri (dev server puro nel browser) — stesso pattern
      // già usato altrove (vedi CategorizationSettings.vue).
    }

    // Regole di notifica personalizzate (Impostazioni → Notifiche) —
    // vedi util/notifyRulesEngine.ts. Gira qui (componente radice, mai
    // smontato finché l'app resta aperta, finestra nascosta in tray
    // compresa) invece che in un mixin legato a una pagina specifica,
    // così valuta le regole indipendentemente da quale pagina è aperta.
    // Un giro subito all'avvio, poi ogni 60s — non serve la stessa
    // cadenza di 30s usata per i dati della Home, queste condizioni
    // cambiano lentamente (minuti di utilizzo, non secondi).
    valutaRegoleNotifica();
    setInterval(() => valutaRegoleNotifica(), 60000);
  },
};
</script>

<style scoped>
/* Only for .themed-scroll below — the same scrollbar-theming rule
   already used on every popup/scrollable panel (Timeline, "show
   everything" lists), applied here to .app-main so the page's own
   main scrollbar matches the theme too instead of the OS default. */
@import './style/modals.css';

#wrapper {
  height: 100%;
}
.app-shell {
  display: flex;
  height: 100%;
}
.app-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  overflow-y: auto;
  background-color: var(--color-bg);
}

/* Sfoca/scurisce sidebar + topbar + contenuto insieme mentre il popup
   di primo avvio (FirstRunWatcherSetup.vue) è aperto — un overlay
   position:fixed esterno non le raggiungerebbe: sidebar e topbar usano
   position:sticky, che le mette in un proprio contesto di rendering
   indipendente dallo z-index di un elemento fuori da .app-shell
   (verificato empiricamente: nemmeno z-index:999 bastava). Applicare il
   filter direttamente qui, sull'elemento che le contiene entrambe,
   funziona sempre perché agisce sui pixel già renderizzati di tutto
   quel sottoalbero, non su un layer separato. pointer-events:none
   impedisce anche i click "attraverso" mentre il popup è aperto. */
.first-run-blur {
  filter: blur(14px) brightness(1.2);
  pointer-events: none;
  transition: filter 0.15s ease;
}
</style>

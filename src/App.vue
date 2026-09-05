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
import { useClockStore } from '~/stores/clock';
import { useServerStore } from '~/stores/server';
import { useFirstRunStore } from '~/stores/firstRun';
import { useUpdaterStore } from '~/stores/updater';
import { detectPreferredTheme } from '~/util/theme';
import { valutaRegoleNotifica } from '~/util/notifyRulesEngine';
// Log diagnostico avanzato, disattivato di default — vedi
// util/diagnostics.ts e Impostazioni → Sviluppatore.
import { setAbilitata as impostaDiagnosticaAbilitata } from '~/util/diagnostics';
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

    // Avvio automatico con Windows: ACCESO di default, ma solo la
    // primissima volta — richiesta esplicita dell'utente. Il vero stato
    // vive nel registro (vedi autostart.rs), autostartDefaultApplied
    // qui serve solo a non riaccenderlo ad ogni avvio se l'utente lo
    // spegne da Impostazioni in seguito.
    if (!settingsStore.autostartDefaultApplied) {
      try {
        await invoke('imposta_avvio_automatico', { abilita: true });
      } catch (e) {
        // Fuori da Tauri, o scrittura registro fallita — non blocca
        // l'avvio dell'app, l'utente può comunque accenderlo a mano
        // dalle Impostazioni.
      }
      await settingsStore.update({ autostartDefaultApplied: true });
    }
  },

  mounted: async function (this: any) {
    // Log diagnostico avanzato — disattivato di default (vedi
    // Impostazioni → Sviluppatore), quindi non aggancia nessun listener
    // di performance/memoria né spedisce eventi via IPC se l'utente non
    // l'ha attivato esplicitamente.
    impostaDiagnosticaAbilitata(useSettingsStore().diagnosticsLoggingEnabled);

    // Bug reale segnalato dall'utente: con l'app rimasta aperta a
    // cavallo del cambio giorno, Home/Timeline/Moduli restavano bloccati
    // sui dati di ieri mentre la Topbar mostrava già "oggi" — vedi
    // stores/clock.ts per la causa (get_today_with_offset() legge l'ora
    // reale, che Vue non traccia come dipendenza) e il fix.
    useClockStore().avvia();

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

    // Un controllo aggiornamenti all'avvio, poi di nuovo ogni volta che
    // la finestra torna in primo piano dopo essere stata nascosta (tray,
    // Alt+Tab) — richiesta esplicita dell'utente: la finestra non viene
    // mai smontata/ricreata quando si nasconde/mostra (resta lo stesso
    // processo, stesso mounted() già passato), quindi senza questo un
    // controllo fatto una volta sola al lancio non scattava mai più per
    // tutta la durata di una sessione, anche lasciando l'app aperta per
    // giorni in tray. Un minimo di 60 secondi tra un controllo e l'altro
    // evita solo di interrogare GitHub più volte per un singolo rapido
    // andirivieni dalla finestra (es. doppio click accidentale), non di
    // ricontrollare quando l'utente torna sull'app dopo un po'; niente
    // ricontrollo se un download/installazione è già in corso (status
    // non-idle) — non avrebbe senso ripartire da capo.
    const settingsStore = useSettingsStore();
    const updaterStore = useUpdaterStore();
    let ultimoControllo = 0;
    const MIN_INTERVALLO_MS = 60 * 1000;
    const controllaSeOpportuno = () => {
      if (updaterStore.status !== 'idle') return;
      const adesso = Date.now();
      if (adesso - ultimoControllo < MIN_INTERVALLO_MS) return;
      ultimoControllo = adesso;
      updaterStore.controllaAggiornamenti(settingsStore.autoUpdateEnabled);
    };
    controllaSeOpportuno();
    try {
      // Emesso da lib.rs (tray, doppio click, seconda istanza rilanciata)
      // ogni volta che la finestra torna in primo piano da nascosta —
      // preferito a onFocusChanged: nascondere la finestra con hide()
      // non genera sempre un vero evento di perdita del focus lato
      // Tauri, quindi mostrarla di nuovo non generava sempre un evento
      // di "cambio" (bug reale segnalato dall'utente: lasciando l'app in
      // tray, il controllo aggiornamenti non ripartiva mai).
      await listen('trackflow://finestra-mostrata', () => {
        controllaSeOpportuno();
        // Richiesta esplicita dell'utente: chiudere l'app nella tray
        // mentre si è su un'altra pagina (es. Impostazioni) e riaprirla
        // dopo la riportava esattamente dov'era rimasta — nascondere la
        // finestra con hide() non distrugge mai la webview, quindi lo
        // stato del router (l'ultima rotta visitata) sopravvive da solo
        // finché non lo si azzera esplicitamente. "/" fa scattare da
        // sola la stessa logica di redirect già usata all'avvio
        // dell'app (route.js: verso l'Activity view se ci sono dati,
        // altrimenti verso Watchers), non un semplice "torna alla home
        // e basta". Nessun effetto se si è già su Home (stessa rotta,
        // nessuna navigazione reale).
        if (this.$route.path !== '/') this.$router.push('/');
      });
    } catch (e) {
      // Fuori da Tauri (dev server puro nel browser) — stesso pattern
      // già usato altrove (vedi CategorizationSettings.vue).
    }
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

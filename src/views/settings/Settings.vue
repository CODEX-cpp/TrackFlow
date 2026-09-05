<template lang="pug">
div.settings-page
  div.settings-content
    section.settings-section(
      v-for="group in groups"
      :key="group.id"
      :id="'settings-section-' + group.id"
    )
      h4.settings-section-title {{ group.label }}
      p.settings-section-help(v-if="group.help") {{ group.help }}
      div.settings-card(v-for="comp in group.components" :key="comp.name")
        component(:is="comp.name")
</template>

<script lang="ts">
import { useSettingsStore } from '~/stores/settings';
import { isWatcherEnabled, refreshModulesConfig } from '~/util/modulesConfig';

import LanguageSettings from '~/views/settings/LanguageSettings.vue';
import DaystartSettings from '~/views/settings/DaystartSettings.vue';
import CategorizationSettings from '~/views/settings/CategorizationSettings.vue';
import DeveloperSettings from '~/views/settings/DeveloperSettings.vue';
import Theme from '~/views/settings/Theme.vue';
import ActivePatternSettings from '~/views/settings/ActivePatternSettings.vue';
import PrivacyFilterSettings from '~/views/settings/PrivacyFilterSettings.vue';
import LunchBreakSettings from '~/views/settings/LunchBreakSettings.vue';
import ScreenshotSettings from '~/views/settings/ScreenshotSettings.vue';
import HomeVisibilitySettings from '~/views/settings/HomeVisibilitySettings.vue';
import VoiSpeedSettings from '~/views/settings/VoiSpeedSettings.vue';
import AiAgentSettings from '~/views/settings/AiAgentSettings.vue';
import VpnMappingSettings from '~/views/settings/VpnMappingSettings.vue';
import NotificationRulesSettings from '~/views/settings/NotificationRulesSettings.vue';
import AutoUpdateSettings from '~/views/settings/AutoUpdateSettings.vue';
import AutostartSettings from '~/views/settings/AutostartSettings.vue';
import AboutSettings from '~/views/settings/AboutSettings.vue';

interface Group {
  id: string;
  label: string;
  help?: string;
  components: { name: string }[];
}

export default {
  name: 'Settings',
  components: {
    LanguageSettings,
    DaystartSettings,
    CategorizationSettings,
    Theme,
    DeveloperSettings,
    ActivePatternSettings,
    PrivacyFilterSettings,
    LunchBreakSettings,
    ScreenshotSettings,
    HomeVisibilitySettings,
    VoiSpeedSettings,
    AiAgentSettings,
    VpnMappingSettings,
    NotificationRulesSettings,
    AutoUpdateSettings,
    AutostartSettings,
    AboutSettings,
  },
  props: {
    group: { type: String, default: '' },
  },
  data() {
    return {
      // All groups render in one continuous page now (explicit
      // request), so "navigating" from the sidebar submenu means
      // scrolling to that section rather than swapping visibility —
      // see the `group` watcher and scrollToSection() below.
      observer: null as IntersectionObserver | null,
      // Set right before this component updates the route itself (see
      // the observer callback) so the `group` watcher below can tell
      // "the URL changed because the user scrolled past a section" from
      // "the URL changed because they clicked a different submenu
      // link" — only the latter should trigger scrollIntoView(), or
      // scrolling and the resulting route update would keep re-
      // triggering each other.
      suppressScrollOnGroupChange: false,
      // True for the whole duration of a programmatic scroll (see
      // scrollToSection/waitForScrollSettle) — keeps the observer below
      // from reacting to sections it only passes through on the way to
      // the actual target.
      suppressScrollSpy: false,
      // Riflette il modulo "VoiSpeed" del menu Moduli della tray —
      // riletto periodicamente sotto (non è reattivo di suo, è un file
      // letto via fetch, vedi util/modulesConfig.ts) così la sezione
      // Integrazioni sparisce da sola se l'utente lo spegne da lì
      // mentre questa pagina è aperta, richiesta esplicita dell'utente.
      voispeedModuleEnabled: true,
      // Stesso identico motivo/meccanismo di voispeedModuleEnabled qui
      // sopra, per il modulo "Sessioni VPN" — richiesta esplicita:
      // niente impostazione di mapping cliente per un watcher che
      // l'utente ha scelto di spegnere dal menu Moduli.
      vpnModuleEnabled: true,
      moduleCheckInterval: null as ReturnType<typeof setInterval> | null,
    };
  },
  computed: {
    activeGroup(): string {
      const requested = this.group || 'general';
      return this.groups.some(g => g.id === requested) ? requested : 'general';
    },
    groups(): Group[] {
      const general: Group = {
        id: 'general',
        label: this.$t('settings.groups.general'),
        help: this.$t('settings.groups.generalHelp'),
        components: [
          { name: 'LanguageSettings' },
          { name: 'DaystartSettings' },
          { name: 'AutoUpdateSettings' },
          { name: 'AutostartSettings' },
        ],
      };
      // Impostazioni della Home aggiunte durante il lavoro sulla
      // Timeline/screenshot ma finora senza una UI vera — vivevano solo
      // nello store (settingsStore.lunchBreakStart/End,
      // screenshotIntervalSeconds), modificabili solo a mano via API.
      const home: Group = {
        id: 'home',
        label: this.$t('settings.groups.home'),
        help: this.$t('settings.groups.homeHelp'),
        components: [
          { name: 'LunchBreakSettings' },
          { name: 'ScreenshotSettings' },
          { name: 'HomeVisibilitySettings' },
        ],
      };
      const appearance: Group = {
        id: 'appearance',
        label: this.$t('settings.groups.appearance'),
        help: this.$t('settings.groups.appearanceHelp'),
        components: [{ name: 'Theme' }],
      };
      const categorization: Group = {
        id: 'categorization',
        label: this.$t('settings.groups.categorization'),
        help: this.$t('settings.groups.categorizationHelp'),
        components: [{ name: 'CategorizationSettings' }, { name: 'ActivePatternSettings' }],
      };
      const privacy: Group = {
        id: 'privacy',
        label: this.$t('settings.groups.privacy'),
        help: this.$t('settings.groups.privacyHelp'),
        components: [{ name: 'PrivacyFilterSettings' }],
      };
      const developer: Group = {
        id: 'developer',
        label: this.$t('settings.groups.developer'),
        help: this.$t('settings.groups.developerHelp'),
        components: [{ name: 'DeveloperSettings' }],
      };
      const about: Group = {
        id: 'about',
        label: this.$t('settings.groups.about'),
        components: [{ name: 'AboutSettings' }],
      };
      const notifications: Group = {
        id: 'notifications',
        label: this.$t('settings.groups.notifications'),
        help: this.$t('settings.groups.notificationsHelp'),
        components: [{ name: 'NotificationRulesSettings' }],
      };

      const groups = [general, home, appearance, categorization, notifications];
      // Il gruppo Integrazioni resta sempre visibile ora (Claude non ha
      // un modulo/toggle nel menu della tray, a differenza di VoiSpeed) —
      // solo la card VoiSpeed al suo interno si nasconde quando quel
      // modulo è spento, non l'intero gruppo come prima (quando
      // conteneva solo VoiSpeed).
      const integrationsComponents: { name: string }[] = [];
      if (this.vpnModuleEnabled) {
        integrationsComponents.push({ name: 'VpnMappingSettings' });
      }
      if (this.voispeedModuleEnabled) {
        integrationsComponents.push({ name: 'VoiSpeedSettings' });
      }
      integrationsComponents.push({ name: 'AiAgentSettings' });
      groups.push(
        {
          id: 'integrations',
          label: this.$t('settings.groups.integrations'),
          help: this.$t('settings.groups.integrationsHelp'),
          components: integrationsComponents,
        },
        privacy,
        developer,
        about
      );
      return groups;
    },
  },
  async created() {
    await Promise.all([this.init(), this.refreshVoispeedModuleFlag(), this.refreshVpnModuleFlag()]);
  },
  mounted() {
    // Lands directly on the requested section for a direct URL load
    // (e.g. bookmarked /settings/appearance) — instant, not smooth, so
    // it reads as "this is where the page starts", not an animated jump
    // right after arriving.
    this.$nextTick(() => {
      if (this.group && this.group !== this.groups[0].id) {
        this.scrollToSection(this.group, false);
      }
      this.setupScrollSpy();
    });
    // Rilegge modules-config.json ogni tanto mentre la pagina resta
    // aperta, così le card VoiSpeed/Mapping VPN dentro Integrazioni
    // spariscono/riappaiono senza dover ricaricare la pagina se
    // l'utente accende/spegne quei moduli dal menu Moduli della tray
    // nel frattempo (il resto del gruppo Integrazioni, es. Claude, non
    // dipende da questo toggle).
    this.moduleCheckInterval = setInterval(() => {
      this.refreshVoispeedModuleFlag();
      this.refreshVpnModuleFlag();
    }, 10000);
  },
  beforeDestroy() {
    if (this.observer) this.observer.disconnect();
    if (this.moduleCheckInterval) clearInterval(this.moduleCheckInterval);
  },
  watch: {
    // Explicit request: clicking a different group in the sidebar
    // submenu scrolls the (now single, continuously stacked) page to
    // that section instead of swapping which one is visible.
    group(newGroup: string) {
      if (this.suppressScrollOnGroupChange) {
        this.suppressScrollOnGroupChange = false;
        return;
      }
      this.scrollToSection(newGroup, true);
    },
  },
  methods: {
    scrollToSection(id: string, smooth: boolean) {
      const el = document.getElementById('settings-section-' + id);
      if (!el) return;
      if (smooth) {
        // A long programmatic jump (e.g. Generale → Sviluppatore, the
        // last group) scrolls past several sections on the way there —
        // without this guard, the scroll-spy observer below briefly
        // sees each of them cross into view and "corrects" the URL to
        // whichever one it caught mid-flight, so the page can settle on
        // the wrong section entirely. Real bug found live while wiring
        // up settings search (clicking a distant result landed on
        // Privacy instead of Sviluppatore) — reproduces identically via
        // the sidebar's own group links, so not specific to search.
        this.suppressScrollSpy = true;
        this.waitForScrollSettle(() => {
          this.suppressScrollSpy = false;
        });
      }
      el.scrollIntoView({ behavior: smooth ? 'smooth' : 'auto', block: 'start' });
    },
    // Polls the actual scrolling container (`.app-main`, not window —
    // this page's scroll lives on that wrapper) until its position stops
    // moving, then calls back. Used instead of the 'scrollend' event so
    // this doesn't depend on WebView2's Chromium version supporting it.
    waitForScrollSettle(callback: () => void) {
      const container = document.querySelector('.app-main') as HTMLElement | null;
      if (!container) {
        callback();
        return;
      }
      let lastTop = container.scrollTop;
      let stableFrames = 0;
      const deadline = Date.now() + 1500;
      const tick = () => {
        const top = container.scrollTop;
        if (Math.abs(top - lastTop) < 1) {
          stableFrames++;
        } else {
          stableFrames = 0;
          lastTop = top;
        }
        if (stableFrames >= 5 || Date.now() > deadline) {
          callback();
          return;
        }
        requestAnimationFrame(tick);
      };
      requestAnimationFrame(tick);
    },
    // Explicit request: as you scroll through the stacked sections, the
    // sidebar's highlighted submenu item should follow along, not stay
    // pinned to whichever one you last clicked. Keeps a thin band near
    // the top of the viewport (rootMargin's -78% bottom cut) — a
    // section counts as "current" once its heading crosses into that
    // band, the same convention most scroll-spy table-of-contents use.
    // Updates the URL (router.replace, no history entry) rather than
    // component state directly, since Sidebar.vue's own highlighting is
    // already driven by the route — one source of truth instead of a
    // second one to keep in sync.
    setupScrollSpy() {
      this.observer = new IntersectionObserver(
        entries => {
          if (this.suppressScrollSpy) return;
          const visible = entries.filter(e => e.isIntersecting);
          if (!visible.length) return;
          visible.sort((a, b) => a.boundingClientRect.top - b.boundingClientRect.top);
          const id = visible[0].target.id.replace('settings-section-', '');
          if (id && id !== this.activeGroup) {
            this.suppressScrollOnGroupChange = true;
            this.$router.replace(`/settings/${id}`).catch(() => undefined);
          }
        },
        { rootMargin: '0px 0px -78% 0px', threshold: 0 }
      );
      for (const group of this.groups) {
        const el = document.getElementById('settings-section-' + group.id);
        if (el) this.observer.observe(el);
      }
    },
    async init() {
      const settingsStore = useSettingsStore();
      return settingsStore.load();
    },
    async refreshVoispeedModuleFlag() {
      await refreshModulesConfig();
      this.voispeedModuleEnabled = isWatcherEnabled('aw-watcher-voispeed');
    },
    async refreshVpnModuleFlag() {
      await refreshModulesConfig();
      this.vpnModuleEnabled = isWatcherEnabled('aw-watcher-vpn');
    },
  },
};
</script>

<style lang="scss">
@import '../../style/theme.css';
@import '../../style/settingsPanel.css';
@import '../../style/modals.css';
</style>

<style scoped lang="scss">
.settings-page {
  // Explicit request: the page used to sit almost flush against the
  // sidebar (4px) — same padding convention as the other bareLayout
  // pages (Progetti.vue).
  padding: 24px 28px;
}

.settings-content {
  min-width: 0;
  max-width: 900px;
  // Richiesta esplicita: su schermi larghi il blocco restava incollato
  // a sinistra (comportamento di default di un elemento block con
  // max-width) invece di stare centrato nello spazio disponibile.
  margin: 0 auto;
}

.settings-section {
  // Explicit request: every group now stacks in one continuous page
  // instead of swapping visibility — needs real separation between
  // sections, and a landing offset so scrollIntoView()/the scroll-spy
  // highlight don't land a section's heading flush against the top
  // edge of the scroll container.
  margin-bottom: 40px;
  scroll-margin-top: 20px;
}

.settings-section:last-child {
  margin-bottom: 0;
}

.settings-section-title {
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-bold);
  color: var(--color-text);
  margin-bottom: 4px;
}

.settings-section-help {
  font-size: var(--font-size-sm);
  color: var(--color-text-faint);
  margin-bottom: 18px;
  max-width: 640px;
}

// Bug reale segnalato dall'utente: il titolo di sezione ha solo 4px di
// margine (pensato per stare vicino al testo di aiuto subito sotto, che
// poi fornisce il vero distacco con i suoi 18px) — un gruppo senza
// `group.help` (es. "Informazioni") salta quel paragrafo del tutto,
// lasciando il riquadro attaccato al titolo. Il selettore `+` scatta
// SOLO quando .settings-card segue il titolo direttamente (nessun
// paragrafo di aiuto in mezzo), quindi non tocca i gruppi che ce l'hanno
// già — stesso distacco già usato tra card multiple nello stesso gruppo.
.settings-section-title + .settings-card {
  margin-top: 14px;
}
</style>

<template lang="pug">
nav.aw-sidebar
  div.sidebar-top
    router-link.brand(to="/" data-tauri-drag-region)
      img.brand-mark(:src="logoUrl" alt="TrackFlow")
      div.brand-text
        div.brand-name TrackFlow
        div.brand-sub fork ActivityWatch

    div.nav-list
      //- "Home" nel mockup corrisponde alla pagina Activity del
      //- progetto originale (rotta /activity/..., mono-client — niente
      //- più hostname nell'URL). La vecchia pagina di benvenuto statica
      //- (rotta /home) è stata rimossa: "/" ora atterra sempre qui.
      router-link.sidebar-nav-item(to="/activity/")
        icon.nav-icon(name="calendar-day")
        span.nav-label {{ $t('nav.home') }}

      router-link.sidebar-nav-item(to="/stopwatch")
        icon.nav-icon(name="stopwatch")
        span.nav-label {{ $t('nav.projects') }}

      router-link.sidebar-nav-item(to="/buckets")
        icon.nav-icon(name="database")
        span.nav-label {{ $t('nav.rawData') }}

      div.sidebar-settings-group
        div.sidebar-nav-item.sidebar-nav-item-split(:class="{ 'router-link-active': isOnSettingsRoute }")
          router-link.sidebar-nav-item-link(to="/settings")
            icon.nav-icon(name="cog")
            span.nav-label {{ $t('nav.settings') }}
          div.sidebar-expand-btn(@click="settingsExpanded = !settingsExpanded")
            icon(:name="settingsExpanded ? 'angle-up' : 'angle-down'" scale="0.85")

        transition(name="submenu")
          div.sidebar-submenu(v-if="settingsExpanded")
            router-link.sidebar-subnav-item(
              v-for="group in settingsGroups"
              :key="group.id"
              :to="'/settings/' + group.id"
            ) {{ group.label }}

  div.sidebar-bottom
    update-popup
    div.sidebar-divider
    div.theme-switch(@click="toggleTheme")
      span.theme-label {{ $t('sidebar.themePrefix') }} {{ themeLabel }}
      div.switch-track(:class="{ 'switch-track-on': isDark }")
        div.switch-thumb
</template>

<style lang="scss" scoped>
@import '../style/theme.css';

.aw-sidebar {
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  width: 220px;
  height: 100vh;
  position: sticky;
  top: 0;
  background-color: var(--color-bg-elev);
  border-right: 1px solid var(--color-border);
  padding: 16px 12px;
  box-sizing: border-box;
  flex-shrink: 0;
}

.brand {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 4px 8px 20px 8px;
  text-decoration: none;
}

.brand-mark {
  width: 28px;
  height: 28px;
  flex-shrink: 0;
  object-fit: contain;
}

.brand-name {
  font-weight: var(--font-weight-bold);
  font-size: var(--font-size-md);
  letter-spacing: var(--letter-spacing-tight);
  color: var(--color-text);
}

.brand-sub {
  font-size: var(--font-size-xs);
  color: var(--color-text-faint);
}

.nav-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.sidebar-nav-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 9px 10px;
  border-radius: var(--radius-md);
  color: var(--color-text-dim);
  text-decoration: none;
  font-size: var(--font-size-base);

  &:hover {
    background-color: var(--color-surface2);
    color: var(--color-text);
  }

  // Vue Router aggiunge questa classe automaticamente alla voce
  // che corrisponde alla pagina aperta in questo momento.
  &.router-link-active {
    background-color: var(--color-surface2);
    color: var(--color-text);
    font-weight: var(--font-weight-semibold);
  }
}

.nav-icon {
  width: 16px;
  flex-shrink: 0;
}

.nav-label {
  flex: 1;
}

// The Impostazioni row splits into a link (icon+label, navigates to
// /settings) and a separate expand button (chevron, toggles the
// submenu) — sharing one .sidebar-nav-item shell so hover/active
// styling stays identical to every other single-link item above it.
.sidebar-nav-item-split {
  padding: 0;
  gap: 0;
}

.sidebar-nav-item-link {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 9px 10px;
  color: inherit;
  text-decoration: none;
}

.sidebar-expand-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  align-self: stretch;
  color: inherit;
  cursor: pointer;
  border-radius: var(--radius-md);

  &:hover {
    background-color: var(--color-surface2);
  }
}

.sidebar-submenu {
  display: flex;
  flex-direction: column;
  gap: 1px;
  margin: 2px 0 2px 44px;
  padding-left: 10px;
  border-left: 1px solid var(--color-border);
}

// Open/close animation for the submenu (explicit request) — max-height
// instead of height since the real height depends on how many settings
// groups there are; 300px is just a generous cap well above the actual
// content (7 items today) so it never visibly clips mid-transition.
.submenu-enter-active,
.submenu-leave-active {
  transition: max-height 0.2s ease, opacity 0.2s ease;
  overflow: hidden;
}

.submenu-enter,
.submenu-leave-to {
  max-height: 0;
  opacity: 0;
}

.submenu-enter-to,
.submenu-leave {
  max-height: 300px;
  opacity: 1;
}

.sidebar-subnav-item {
  padding: 7px 10px;
  border-radius: var(--radius-md);
  color: var(--color-text-faint);
  text-decoration: none;
  font-size: var(--font-size-sm);

  &:hover {
    background-color: var(--color-surface2);
    color: var(--color-text);
  }

  &.router-link-active {
    background-color: var(--color-surface2);
    color: var(--color-text);
    font-weight: var(--font-weight-semibold);
  }
}

.sidebar-bottom {
  padding-top: 12px;
}

.sidebar-divider {
  border-top: 1px solid var(--color-border);
  margin-bottom: 12px;
}

.theme-switch {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 10px;
  border-radius: var(--radius-md);
  cursor: pointer;
  color: var(--color-text-dim);
  font-size: var(--font-size-sm);

  &:hover {
    background-color: var(--color-surface2);
  }
}

.switch-track {
  width: 32px;
  height: 18px;
  border-radius: var(--radius-pill);
  background-color: var(--color-surface2);
  position: relative;
  transition: background-color 0.15s ease;
}

.switch-track-on {
  background-color: var(--color-accent1);
}

.switch-thumb {
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background-color: var(--color-bg-elev);
  position: absolute;
  top: 2px;
  left: 2px;
  transition: left 0.15s ease;

  .switch-track-on & {
    left: 16px;
  }
}
</style>

<script lang="ts">
import _ from 'lodash';

import 'vue-awesome/icons/calendar-day';
import 'vue-awesome/icons/stopwatch';
import 'vue-awesome/icons/database';
import 'vue-awesome/icons/cog';
import 'vue-awesome/icons/angle-down';
import 'vue-awesome/icons/angle-up';

import { useSettingsStore } from '~/stores/settings';
import logoUrl from '~/assets/logo.png';

export default {
  name: 'Sidebar',
  components: {
    'update-popup': () => import('~/components/UpdatePopup.vue'),
  },
  data() {
    return {
      settingsStore: useSettingsStore(),
      logoUrl,
      // Starts expanded when landing directly on a /settings/* URL
      // (see the immediate route watcher below) so the active submenu
      // item is visible right away instead of hidden behind a click.
      settingsExpanded: false,
    };
  },
  computed: {
    // Mirrors Settings.vue's own groups() list — kept here as a
    // second copy (not imported) since Settings.vue's version reads
    // $isAndroid/i18n through the page instance itself; duplicating
    // just the id+label pairs is simpler than restructuring that for
    // one shared list. Order matches the tabs inside the page. Reads
    // the same settings.groups.* i18n keys as Settings.vue so the two
    // never drift out of sync when the language changes.
    settingsGroups(): { id: string; label: string }[] {
      const groups = [
        { id: 'general', label: this.$t('settings.groups.general') as string },
        { id: 'home', label: this.$t('settings.groups.home') as string },
        { id: 'appearance', label: this.$t('settings.groups.appearance') as string },
        { id: 'categorization', label: this.$t('settings.groups.categorization') as string },
      ];
      // Integrazioni resta sempre in elenco ora (ospita anche Claude, che
      // non dipende dal menu Moduli) — solo la card VoiSpeed al suo
      // interno si nasconde quando quel modulo è spento, non l'intera
      // voce del sottomenu come prima.
      groups.push(
        { id: 'integrations', label: this.$t('settings.groups.integrations') as string },
        { id: 'privacy', label: this.$t('settings.groups.privacy') as string },
        { id: 'developer', label: this.$t('settings.groups.developer') as string },
        { id: 'about', label: this.$t('settings.groups.about') as string }
      );
      return groups;
    },
    // Stessa logica di Header.vue per capire se siamo su tema scuro,
    // semplificata: non gestiamo ancora "auto" qui, solo il valore
    // esplicito salvato.
    isDark() {
      return this.settingsStore.theme === 'dark';
    },
    themeLabel() {
      return this.isDark ? this.$t('sidebar.themeDark') : this.$t('sidebar.themeLight');
    },
    isOnSettingsRoute(): boolean {
      return this.$route.path.startsWith('/settings');
    },
  },
  watch: {
    // Only ever forces the submenu OPEN on arrival at a settings URL —
    // never auto-closes it, so a manual collapse (chevron click) while
    // already on the page sticks instead of springing back open on the
    // next in-page settings navigation.
    isOnSettingsRoute: {
      immediate: true,
      handler(onSettings: boolean) {
        if (onSettings) this.settingsExpanded = true;
      },
    },
  },
  mounted() {
    // Applica subito la classe giusta al body in base al tema
    // salvato, così la sidebar (e in futuro le altre pagine
    // ricollegate a theme.css) parte già col colore corretto.
    this.applyThemeClass();
  },
  methods: {
    toggleTheme() {
      // update() (non un'assegnazione diretta) è quello che fa anche
      // Theme.vue nelle Impostazioni — senza, il cambio si vedeva
      // subito (stato Pinia condiviso, si riflette ovunque) ma non
      // veniva mai salvato: al riavvio successivo tornava al valore
      // precedente (bug segnalato dall'utente).
      //
      // Il nuovo valore va calcolato QUI e applicato al body con
      // QUESTO valore — non richiamando applyThemeClass() subito dopo,
      // che rilegge this.isDark dallo store. update() è async e fa
      // `await this.ensureLoaded()` prima del $patch che aggiorna
      // davvero settingsStore.theme, quindi quel $patch arriva un
      // microtask dopo, non nello stesso giro sincrono: applyThemeClass()
      // chiamato qui subito dopo leggerebbe ancora il valore VECCHIO,
      // mentre lo switch (bindato reattivamente a isDark nel template)
      // si aggiorna correttamente un istante più tardi da solo — da qui
      // il bug segnalato dall'utente (switch cambia, tema no, finché non
      // si clicca una seconda volta). Stessa soluzione già corretta in
      // Theme.vue: usare il valore appena scelto, non rileggerlo.
      const nuovoTema = this.isDark ? 'light' : 'dark';
      this.settingsStore.update({ theme: nuovoTema });
      document.body.classList.toggle('theme-dark', nuovoTema === 'dark');
      document.body.classList.toggle('theme-light', nuovoTema !== 'dark');
    },
    applyThemeClass() {
      document.body.classList.toggle('theme-dark', this.isDark);
      document.body.classList.toggle('theme-light', !this.isDark);
    },
  },
};
</script>

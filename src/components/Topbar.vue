<template lang="pug">
div.topbar(data-tauri-drag-region)
  div.topbar-title(data-tauri-drag-region) {{ pageTitle }}

  //- Zona dedicata solo a trascinare la finestra (niente contenuto,
  //- niente click) — prima lo spazio "libero" per trascinare era solo
  //- quello che restava per caso tra il titolo e i controlli a destra,
  //- stretto o largo a seconda della pagina/larghezza finestra. Con
  //- flex:1 qui e topbar-right non più elastica (vedi sotto), questa
  //- striscia si prende sempre tutto lo spazio davvero vuoto, il più
  //- possibile per trascinare senza mai sovrapporsi a filtri/pulsanti.
  div.topbar-drag-spacer(data-tauri-drag-region)

  div.topbar-right(data-tauri-drag-region)
    //- Data e Filtri compaiono SOLO sulla pagina Activity (Home) —
    //- sulle altre pagine (Progetti, Impostazioni, ecc.) non c'è
    //- nessun periodo da sfogliare, quindi li nascondiamo invece di
    //- mostrarli finti/inattivi.
    template(v-if="isActivityPage")
      div.date-wrap
        div.date-chip
          span.date-arrow(@click="goPrevDay")
            span ‹
          span.date-chip-clickable(@click="openCalendar") {{ dateLabel }}
          span.date-arrow(@click="goNextDay")
            span ›

        div(v-if="calendarOpen")
          div.popover-backdrop(@click="calendarOpen = false")
          div.calendar-anchor
            calendar-picker(
              :selected-date="currentDate"
              :initial-view-date="currentDate"
              :is-date-disabled="isDayDisabled"
              @select="onSelectDay"
              @month-change="onCalendarMonthChange"
            )

      //- Zoom della Timeline (Giorno/4h/1h) — spostato qui dalla card
      //- della Timeline stessa su richiesta esplicita, condiviso tramite
      //- lo store timelineHighlight (vedi stores/timelineHighlight.ts)
      //- perché Topbar e HomeTimelineSection sono componenti "fratelli",
      //- non c'è una relazione diretta padre/figlio.
      div.zoom-tabs
        div.zoom-tab(:class="{ 'zoom-tab-active': timelineStore.zoom === 'day' }" @click="timelineStore.setZoom('day')") {{ $t('topbar.zoomDay') }}
        div.zoom-tab(:class="{ 'zoom-tab-active': timelineStore.zoom === '4h' }" @click="timelineStore.setZoom('4h')") 4h
        div.zoom-tab(:class="{ 'zoom-tab-active': timelineStore.zoom === '1h' }" @click="timelineStore.setZoom('1h')") 1h

      //- Normale/Background — nascosto temporaneamente su richiesta
      //- esplicita (2026-08-11): la lettura delle icone nascoste dietro
      //- "Mostra icone nascoste" non è ancora affidabile (vedi
      //- BLUEPRINT.md sezione 7), quindi il toggle resta commentato e
      //- la Home mostra sempre "Normale" finché non si risolve. Codice
      //- e store (timelineStore.timelineMode) lasciati intatti per
      //- riattivarlo con un semplice uncomment quando pronto.
      //-
      //-   div.zoom-tabs
      //-     div.zoom-tab(:class="{ 'zoom-tab-active': timelineStore.timelineMode === 'normal' }" @click="timelineStore.setTimelineMode('normal')") Normale
      //-     div.zoom-tab(:class="{ 'zoom-tab-active': timelineStore.timelineMode === 'background' }" @click="timelineStore.setTimelineMode('background')") Background

      //- FILTRI — nascosto su richiesta esplicita dell'utente
      //- (2026-08-12): erano ancora finti/decorativi (lista di
      //- domini/app di esempio, non filtravano dati veri), e
      //- l'utente ha deciso di nasconderli invece di costruire la
      //- logica di filtro reale (vedi BLUEPRINT.md sezione "Prossimi
      //- passi", voce 8, marcata risolta così). Codice sotto e nello
      //- script (fakeDomains/fakeApps/filtersOpen/fakeFiltersCount/
      //- clearFakeFilters) lasciato intatto per riattivarlo con un
      //- semplice uncomment se in futuro si costruisce un filtro vero.
      //-
      //- div.filters-wrap
      //-   div.filter-chip(@click="filtersOpen = !filtersOpen")
      //-     | {{ $t('topbar.filters') }}
      //-     span.filter-badge(v-if="fakeFiltersCount > 0") {{ fakeFiltersCount }}
      //-
      //-   div(v-if="filtersOpen")
      //-     div.popover-backdrop(@click="filtersOpen = false")
      //-     div.filters-popover
      //-       div.filter-section-label {{ $t('topbar.filterTopDomains') }}
      //-       div.filter-check-row(
      //-         v-for="f in fakeDomains"
      //-         :key="f.name"
      //-         @click="f.checked = !f.checked"
      //-       )
      //-         div.filter-checkbox(:class="{ 'filter-checkbox-on': f.checked }")
      //-         span {{ f.name }}
      //-
      //-       div.filter-divider
      //-
      //-       div.filter-section-label {{ $t('topbar.filterTopApps') }}
      //-       div.filter-check-row(
      //-         v-for="f in fakeApps"
      //-         :key="f.name"
      //-         @click="f.checked = !f.checked"
      //-       )
      //-         div.filter-checkbox(:class="{ 'filter-checkbox-on': f.checked }")
      //-         span {{ f.name }}
      //-
      //-       div.filter-clear-btn(@click="clearFakeFilters") {{ $t('topbar.clearFilters') }}

    //- SEARCH — real on Projects/Home (matching project names; click
    //- opens the detail popup, navigating to /stopwatch first if
    //- triggered from Home) and on Settings (matching setting titles/
    //- help text against settingsSearchIndex.ts; click scrolls to that
    //- section, see Topbar.vue::pickSuggestion). Still a no-op field on
    //- every other page, until we wire up search for that page's data.
    search-dropdown(
      :open="searchOpen"
      :value="searchQuery"
      @input="searchQuery = $event"
      :suggestions="searchSuggestionItems"
      :show-suggestions="showSuggestions"
      :placeholder="isSettingsPage ? $t('topbar.searchSettingsPlaceholder') : $t('topbar.searchPlaceholder')"
      :empty-label="isSettingsPage ? $t('topbar.searchSettingsEmpty') : $t('topbar.searchEmpty')"
      @select="pickSuggestion"
      @close="closeSearch"
    )
    div.filter-chip(v-if="!isBucketsPage" @click="toggleSearch") {{ $t('topbar.search') }}

  //- Pulsanti finestra (riduci/ingrandisci/chiudi) — sostituiscono la
  //- barra del titolo nativa di Windows, tolta lato Rust con
  //- `.decorations(false)` (vedi src-tauri/src/lib.rs). Vivono qui
  //- nella topbar invece che in una barra a parte (prima versione,
  //- rivista su richiesta esplicita: il nome app in una barra dedicata
  //- era ridondante, e una barra in più introduceva un doppio
  //- scrollbar/riquadro visivo indesiderato). Icone disegnate a mano
  //- (SVG minimale), non icone a libreria: replicano lo stile sottile
  //- di Windows 11 invece dello stile più "pieno" delle icone
  //- Font Awesome usate altrove nell'app.
  div.window-controls
    button.window-btn(type="button" :title="$t('topbar.minimize')" @click="minimizeWindow")
      svg(viewBox="0 0 10 10" width="10" height="10")
        line(x1="0" y1="5" x2="10" y2="5" stroke="currentColor" stroke-width="1")
    button.window-btn(type="button" :title="isMaximized ? $t('topbar.restore') : $t('topbar.maximize')" @click="toggleMaximizeWindow")
      svg(v-if="!isMaximized" viewBox="0 0 10 10" width="10" height="10")
        rect(x="0.5" y="0.5" width="9" height="9" fill="none" stroke="currentColor" stroke-width="1")
      svg(v-else viewBox="0 0 10 10" width="10" height="10")
        rect(x="0.5" y="2.5" width="7" height="7" fill="none" stroke="currentColor" stroke-width="1")
        path(d="M2.5 2.5 V0.5 H9.5 V7.5 H7.5" fill="none" stroke="currentColor" stroke-width="1")
    button.window-btn.window-btn-close(type="button" :title="$t('topbar.close')" @click="closeWindow")
      svg(viewBox="0 0 10 10" width="10" height="10")
        line(x1="0" y1="0" x2="10" y2="10" stroke="currentColor" stroke-width="1")
        line(x1="10" y1="0" x2="0" y2="10" stroke="currentColor" stroke-width="1")
</template>

<style lang="scss" scoped>
@import '../style/theme.css';

// Colore fisso per il testo sopra al bottone AI (sfondo dorato/accento):
// il mockup lo tiene scuro sempre, indipendentemente dal tema.
$onAccentText: #241a12;

.topbar {
  height: 64px;
  flex: none;
  display: flex;
  align-items: center;
  padding: 0 0 0 28px;
  border-bottom: 1px solid var(--color-border);
  background-color: var(--color-bg-elev);
  // Fissa in cima allo scroll di .app-main (vedi App.vue) — senza,
  // scorrendo in basso in pagine lunghe (es. Impostazioni) la topbar
  // spariva insieme al resto: dato che contiene sia la zona di
  // trascinamento finestra (data-tauri-drag-region) sia i pulsanti
  // riduci/ingrandisci/chiudi (nessuna barra del titolo nativa, vedi
  // .decorations(false) in lib.rs), sparire la rendeva impossibile da
  // trascinare o chiudere finché non si tornava in cima. Bug segnalato
  // dall'utente, 2026-08-11. sticky invece di fixed: resta un elemento
  // normale nel flusso (niente offset manuale da compensare sotto), si
  // limita a "restare incollata" quando il suo posto normale
  // scorrerebbe fuori dalla vista.
  position: sticky;
  top: 0;
  z-index: 20;
}

.topbar-title {
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-bold);
  color: var(--color-text);
  // L'area vuota della topbar è trascinabile (data-tauri-drag-region
  // sull'elemento padre) — il testo del titolo ricade lì sopra ma non
  // è mai grande abbastanza da essere fastidioso da evitare quando si
  // vuole spostare la finestra.
}

// Non più elastica (era flex:1) — si allarga solo quanto serve al suo
// contenuto (data/zoom/filtri/cerca), lasciando il resto dello spazio
// libero davvero alla striscia dedicata al trascinamento qui sopra,
// invece di conteso/ambiguo tra le due.
.topbar-right {
  flex: none;
  display: flex;
  align-items: center;
  gap: 10px;
  padding-right: 20px;
}

.topbar-drag-spacer {
  flex: 1;
  min-width: 40px;
  align-self: stretch;
}

.window-controls {
  display: flex;
  align-items: center;
  gap: 2px;
  flex: none;
  margin-left: 6px;
  margin-right: 10px;
}

// Prima si allungavano su tutta l'altezza della topbar (align-self:
// stretch, 64px) — molto più alti che larghi, sproporzionati rispetto
// agli altri pulsanti/chip della topbar. Dimensione fissa e compatta
// invece, centrata verticalmente come tutto il resto — la topbar torna
// a sembrare della stessa altezza di prima anche se il numero in px
// non è cambiato.
.window-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--color-text-dim);
  cursor: pointer;
}

.window-btn:hover {
  background-color: var(--color-surface2);
}

.window-btn-close:hover {
  background-color: var(--color-danger);
  color: white;
}

.date-chip,
.filter-chip {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 14px;
  border-radius: var(--radius-md);
  background-color: var(--color-surface2);
  font-size: var(--font-size-sm);
  color: var(--color-text-dim);
  cursor: pointer;
  transition: filter 0.1s ease;

  &:hover {
    filter: brightness(1.1);
  }
}

.zoom-tabs {
  display: flex;
  gap: 4px;
  background-color: var(--color-surface2);
  border-radius: var(--radius-md);
  padding: 3px;
}

.zoom-tab {
  padding: 5px 12px;
  border-radius: var(--radius-sm);
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-semibold);
  color: var(--color-text-faint);
  cursor: pointer;
}

.zoom-tab:hover {
  color: var(--color-text);
}

.zoom-tab-active {
  background-color: var(--color-accent1);
  color: #241a12;
}

.zoom-tab-active:hover {
  color: #241a12;
}

.date-wrap,
.filters-wrap {
  position: relative;
}

.date-arrow {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  color: var(--color-text-faint);
  font-size: var(--font-size-md);
  line-height: 1;
  cursor: pointer;
  border-radius: var(--radius-sm);

  span {
    margin-top: -1px;
  }

  &:hover {
    // Qui usiamo --color-bg (più scuro di surface2) invece di
    // surface2: il contenitore attorno (.date-chip) è già colorato
    // surface2 di base, quindi usare lo stesso colore per l'hover
    // sarebbe invisibile — serve un colore diverso da quello che lo
    // circonda già, non necessariamente lo stesso del calendario.
    background-color: var(--color-bg);
    color: var(--color-text);
  }
}

.date-chip-clickable {
  cursor: pointer;
  min-width: 74px;
  text-align: center;
}

.filter-badge {
  background-color: $onAccentText;
  color: var(--color-accent1);
  font-size: 10px;
  font-weight: 800;
  border-radius: var(--radius-md);
  padding: 1px 6px;
}

.popover-backdrop {
  position: fixed;
  inset: 0;
  z-index: 39;
}

.calendar-anchor,
.filters-popover {
  position: absolute;
  top: calc(100% + 8px);
  left: 0;
  z-index: 40;
}

.filters-popover {
  background-color: var(--color-bg-elev);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-elevated);
  width: 260px;
  padding: 16px;
}

.filter-section-label {
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-bold);
  color: var(--color-text-faint);
  text-transform: uppercase;
  letter-spacing: var(--letter-spacing-wide);
  margin: 0 0 8px;
}

.filter-check-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 4px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  font-size: 12.5px;
  color: var(--color-text-dim);

  &:hover {
    background-color: var(--color-surface2);
  }
}

.filter-checkbox {
  width: 15px;
  height: 15px;
  flex: none;
  border-radius: var(--radius-sm);
  border: 1.5px solid var(--color-border);
}

.filter-checkbox-on {
  border: none;
  background-color: var(--color-accent1);
}

.filter-divider {
  height: 1px;
  background-color: var(--color-border);
  margin: 10px 0;
}

.filter-clear-btn {
  text-align: center;
  padding: 8px 0;
  margin-top: 10px;
  border-radius: var(--radius-md);
  background-color: var(--color-surface2);
  color: var(--color-text-dim);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-semibold);
  cursor: pointer;

  &:hover {
    filter: brightness(1.15);
  }
}

</style>

<script lang="ts">
import moment from 'moment';
import { useProjectsStore, Project } from '~/stores/projects';
import { useTimelineHighlightStore } from '~/stores/timelineHighlight';
import { useBucketsStore } from '~/stores/buckets';
import { useSettingsStore } from '~/stores/settings';
import { get_today_with_offset } from '~/util/time';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { SETTINGS_SEARCH_INDEX } from '~/util/settingsSearchIndex';

// Stessa mappa gruppo→chiave i18n del label usata da Settings.vue nel
// suo groups() — duplicata qui (piccola, 8 righe) invece di importarla
// da lì per non creare un accoppiamento diretto fra Topbar e la pagina
// Impostazioni solo per questo. "notifications" fa eccezione: usa
// settings.notifications.title invece di settings.groups.notifications,
// stessa scelta già fatta in Settings.vue.
const SETTINGS_GROUP_LABEL_KEY: Record<string, string> = {
  general: 'settings.groups.general',
  home: 'settings.groups.home',
  appearance: 'settings.groups.appearance',
  categorization: 'settings.groups.categorization',
  notifications: 'settings.notifications.title',
  integrations: 'settings.groups.integrations',
  privacy: 'settings.groups.privacy',
  developer: 'settings.groups.developer',
};

export default {
  name: 'Topbar',
  components: {
    'calendar-picker': () => import('./CalendarPicker.vue'),
    'search-dropdown': () => import('./SearchDropdown.vue'),
  },
  data() {
    return {
      projectsStore: useProjectsStore(),
      timelineStore: useTimelineHighlightStore(),
      bucketsStore: useBucketsStore(),
      settingsStore: useSettingsStore(),
      // Pulsanti finestra (vedi window-controls nel template) — nessuna
      // barra del titolo nativa di Windows, tolta lato Rust
      // (`.decorations(false)`). "Chiudi" chiama semplicemente
      // `window.close()`: Tauri lo tratta come un vero WM_CLOSE, quindi
      // finisce nello stesso gestore già registrato in lib.rs che
      // nasconde la finestra invece di uscire (comportamento invariato).
      isMaximized: false,
      unlistenResize: null as (() => void) | null,
      calendarOpen: false,
      calendarViewMonth: moment(),
      // Giorni (YYYY-MM-DD) del mese attualmente mostrato nel
      // calendario che hanno almeno un evento registrato — null
      // finché non li abbiamo ancora caricati (in quel caso non
      // disabilitiamo nulla, per non "lampeggiare" prima del tempo).
      daysWithData: null as Set<string> | null,
      filtersOpen: false,
      searchOpen: false,
      searchQuery: '',

      fakeDomains: [
        { name: 'github.com', checked: false },
        { name: 'youtube.com', checked: false },
        { name: 'gmail.com', checked: false },
      ],
      fakeApps: [
        { name: 'Visual Studio Code', checked: false },
        { name: 'Firefox', checked: false },
        { name: 'Excel', checked: false },
      ],
    };
  },

  computed: {
    isActivityPage(): boolean {
      return this.$route.path.startsWith('/activity');
    },
    isProjectsPage(): boolean {
      return this.$route.path.startsWith('/stopwatch');
    },
    isSettingsPage(): boolean {
      return this.$route.path.startsWith('/settings');
    },
    isBucketsPage(): boolean {
      return this.$route.path.startsWith('/buckets');
    },
    // Real suggestions: on the Projects page and on Home (project
    // names), and on Settings (titles/help text of every setting). On
    // every other page the field stays fake — there's no real data
    // wired up to search yet.
    canSearch(): boolean {
      return this.isProjectsPage || this.isActivityPage || this.isSettingsPage;
    },
    searchSuggestionItems(): { id: string; label: string; sublabel?: string; groupId?: string }[] {
      if (!this.canSearch) return [];
      const q = this.searchQuery.trim().toLowerCase();
      if (!q) return [];
      if (this.isSettingsPage) {
        return SETTINGS_SEARCH_INDEX.map(entry => ({
          entry,
          title: this.$t(entry.titleKey) as string,
          help: entry.helpKey ? (this.$t(entry.helpKey) as string) : '',
        }))
          .filter(
            ({ title, help }) => title.toLowerCase().includes(q) || help.toLowerCase().includes(q)
          )
          .slice(0, 8)
          .map(({ entry, title }) => ({
            id: entry.titleKey,
            label: title,
            sublabel: this.$t(SETTINGS_GROUP_LABEL_KEY[entry.groupId]) as string,
            groupId: entry.groupId,
          }));
      }
      return this.projectsStore.projects
        .filter((p: Project) => p.name.toLowerCase().includes(q))
        .slice(0, 6)
        .map((p: Project) => ({ id: p.id, label: p.name }));
    },
    showSuggestions(): boolean {
      return this.canSearch && this.searchQuery.trim().length > 0;
    },
    // Titolo mostrato a sinistra, in base a dove ti trovi — stessi
    // nomi scelti per le voci della sidebar.
    pageTitle(): string {
      const path = this.$route.path;
      if (path.startsWith('/activity')) return this.$t('nav.home') as string;
      if (path.startsWith('/stopwatch')) return this.$t('nav.projects') as string;
      if (path.startsWith('/buckets')) return this.$t('nav.rawData') as string;
      if (path.startsWith('/settings')) return this.$t('nav.settings') as string;
      return this.$t('app.name') as string;
    },
    // TrackFlow è mono-client: l'host non è mai un parametro di rotta,
    // si risolve sempre dal buckets store.
    host(): string {
      return this.bucketsStore.host;
    },
    // Fallback offset-aware ("inizio giornata" nelle Impostazioni, es.
    // 04:00): senza, tra mezzanotte e l'orario di inizio giornata questo
    // valore risultava disallineato rispetto a quello usato dai Moduli
    // (HomeModulesSection.vue), che è sempre stato offset-aware — la
    // Topbar mostrava "Oggi" riferendosi al giorno di calendario
    // letterale, i Moduli calcolavano ancora il giorno precedente,
    // interpretavano la data esplicita che la Topbar scriveva nell'URL
    // (sempre calendario letterale) come se fosse già oggi-con-offset e
    // finivano per interrogare un periodo nel futuro (04:00 di oggi -
    // 04:00 di domani) — da qui moduli vuoti dopo un giro ieri->oggi.
    currentDate(): string {
      return (
        (this.$route.params.date as string) || get_today_with_offset(this.settingsStore.startOfDay)
      );
    },
    dateLabel(): string {
      const oggi = get_today_with_offset(this.settingsStore.startOfDay);
      if (this.currentDate === oggi) return this.$t('topbar.today') as string;
      const ieri = moment(oggi, 'YYYY-MM-DD').subtract(1, 'day').format('YYYY-MM-DD');
      if (this.currentDate === ieri) return this.$t('topbar.yesterday') as string;
      return moment(this.currentDate).format('ddd D MMM');
    },
    fakeFiltersCount(): number {
      return (
        this.fakeDomains.filter(f => f.checked).length + this.fakeApps.filter(f => f.checked).length
      );
    },
  },

  async mounted() {
    const win = getCurrentWindow();
    this.isMaximized = await win.isMaximized();
    this.unlistenResize = await win.onResized(async () => {
      this.isMaximized = await win.isMaximized();
    });
  },
  beforeDestroy() {
    if (this.unlistenResize) this.unlistenResize();
  },

  methods: {
    minimizeWindow() {
      getCurrentWindow().minimize();
    },
    async toggleMaximizeWindow() {
      await getCurrentWindow().toggleMaximize();
      this.isMaximized = await getCurrentWindow().isMaximized();
    },
    closeWindow() {
      getCurrentWindow().close();
    },
    openCalendar() {
      this.calendarViewMonth = moment(this.currentDate);
      this.calendarOpen = !this.calendarOpen;
      this.loadDaysWithData();
    },
    onCalendarMonthChange(monthStr: string) {
      this.calendarViewMonth = moment(monthStr, 'YYYY-MM');
      this.loadDaysWithData();
    },
    isDayDisabled(dateStr: string): boolean {
      return this.daysWithData !== null && !this.daysWithData.has(dateStr);
    },
    async loadDaysWithData() {
      this.daysWithData = null;

      const bucketId = 'aw-watcher-window';
      const inizioGriglia = this.calendarViewMonth.clone().startOf('month').startOf('isoWeek');
      const fineGriglia = inizioGriglia.clone().add(42, 'days');

      try {
        const events = await (this as any).$aw.getEvents(bucketId, {
          start: inizioGriglia.toDate(),
          end: fineGriglia.toDate(),
          limit: -1,
        });
        const giorni = new Set<string>();
        for (const e of events) {
          giorni.add(moment(e.timestamp).format('YYYY-MM-DD'));
        }
        this.daysWithData = giorni;
      } catch (err) {
        // Se la richiesta fallisce (es. bucket non esistente per
        // quell'host), non blocchiamo il calendario: lasciamo tutti i
        // 42 giorni della griglia cliccabili invece di disabilitarli
        // per errore.
        console.error(this.$t('topbar.loadDaysError'), err);
        const giorni = new Set<string>();
        for (let i = 0; i < 42; i++) {
          giorni.add(inizioGriglia.clone().add(i, 'days').format('YYYY-MM-DD'));
        }
        this.daysWithData = giorni;
      }
    },
    // La navigazione qui usa sempre il periodo "day" — i pulsanti
    // 7gg/30gg/settimana/mese del vecchio toolbar non sono più
    // raggiungibili da qui (il mockup non li prevede). Se ti servono
    // ancora spesso, li reintegriamo altrove.
    goToDate(dateStr: string) {
      this.$router.push(`/activity/day/${dateStr}/view/`);
    },
    goPrevDay() {
      this.goToDate(moment(this.currentDate).subtract(1, 'days').format('YYYY-MM-DD'));
    },
    goNextDay() {
      this.goToDate(moment(this.currentDate).add(1, 'days').format('YYYY-MM-DD'));
    },
    onSelectDay(dateStr: string) {
      this.goToDate(dateStr);
      this.calendarOpen = false;
    },
    clearFakeFilters() {
      this.fakeDomains.forEach(f => (f.checked = false));
      this.fakeApps.forEach(f => (f.checked = false));
    },
    toggleSearch() {
      this.searchOpen = !this.searchOpen;
      if (!this.searchOpen) this.searchQuery = '';
    },
    closeSearch() {
      this.searchOpen = false;
      this.searchQuery = '';
    },
    // Asks Progetti.vue to open the detail popup for this project —
    // same requestedDetailProjectId channel the Home "Progetti Attivi"
    // grid already uses to do the exact same thing on name click. When
    // triggered from Home (Progetti.vue isn't mounted there), navigate
    // to /stopwatch first so the popup actually appears somewhere
    // instead of the request just sitting unread in the store.
    pickSuggestion(item: { id: string; label: string; groupId?: string }) {
      if (item.groupId) {
        // Risultato di ricerca impostazioni — Settings.vue fa già da
        // solo scroll-to-section reagendo al cambio del parametro
        // :group nella rotta (sia che la pagina sia già montata, sia
        // che venga montata ora), vedi Settings.vue::scrollToSection.
        this.$router.push('/settings/' + item.groupId).catch(() => undefined);
        this.closeSearch();
        return;
      }
      this.projectsStore.requestProjectDetail(item.id);
      if (!this.isProjectsPage) this.$router.push('/stopwatch');
      this.closeSearch();
    },
  },
};
</script>

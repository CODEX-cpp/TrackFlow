<template lang="pug">
div.vis-card(v-if="visibile")
  h5.vis-card-title
    icon.handle(name="bars" v-if="editable" style="opacity: 0.6; cursor: grab;")
    | {{ cardTitle }}
  div(v-if="editable").vis-style-dropdown-btn
    b-dropdown.mr-1(size="sm" variant="outline-secondary" right)
      template(v-slot:button-content)
        icon(name="cog")
      b-dropdown-item(v-for="t in types" :key="t" variant="outline-secondary" @click="$emit('onTypeChange', id, t)")
        | {{ visualizations[t].title }} #[span.small.text-warning(v-if="!visualizations[t].available") {{ $t('visualizations.noData') }}]
    b-button.p-0(size="sm", variant="outline-danger" @click="$emit('onRemove', id)")
      icon(name="times")

  div(v-if="!supports_period")
    div.vis-alert.vis-alert-warning
      | {{ $t('visualizations.unsupportedPeriod') }}

  div(v-if="activityStore.buckets.loaded")
    // Check data prerequisites
    div(v-if="!has_prerequisites")
      div.vis-alert.vis-alert-warning
        | {{ $t('visualizations.missingWatcher') }}
        | {{ $t('visualizations.missingWatcherHint') }} #[a.vis-alert-link(href="https://docs.activitywatch.net/en/latest/watchers.html") {{ $t('visualizations.docLink') }}].

    div(v-if="type == 'top_apps'")
      aw-top-summary(:fields="top_apps_filtered",
                 :namefunc="e => e.data.app",
                 :displayfunc="e => appDisplayName(e.data.app)",
                 :icon-url-func="e => appIconUrl(e.data.app)",
                 :iconfunc="e => appIcon(e.data.app)",
                 :colorfunc="e => e.data.app",
                 :raw-color-func="e => appIconColor(e.data.app)",
                 :title="visualizations[type].title",
                 highlightable
                 :highlighted-key="highlightStore.highlightedKey"
                 @select="onSelectRow"
                 with_limit)
    div(v-if="type == 'top_titles' && !activityStore.android.available")
      aw-top-summary(:fields="top_titles_filtered",
                 :namefunc="e => e.data.title",
                 :icon-url-func="e => appIconUrl(e.data.app)",
                 :iconfunc="e => appIcon(e.data.app)",
                 :colorfunc="e => e.data.app",
                 :raw-color-func="e => appIconColor(e.data.app)",
                 :secondary-key-func="e => e.data.app",
                 :title="visualizations[type].title",
                 highlightable
                 :highlighted-key="highlightStore.highlightedTitle"
                 @select="onSelectTitleRow"
                 with_limit)
    div(v-if="type == 'top_domains'")
      aw-top-summary(:fields="activityStore.browser.top_domains",
                 :namefunc="e => e.data.$domain",
                 :colorfunc="e => e.data.$domain",
                 :title="visualizations[type].title",
                 highlightable
                 :highlighted-key="highlightStore.highlightedKey"
                 @select="onSelectRow"
                 with_limit)
    div(v-if="type == 'top_vpn_clients'")
      aw-top-summary(:fields="vpn_top_clients",
                 :namefunc="e => e.data.cliente",
                 :colorfunc="e => e.data.cliente",
                 :title="visualizations[type].title",
                 highlightable
                 :highlighted-key="highlightStore.highlightedKey"
                 @select="onSelectRow"
                 with_limit)
    div(v-if="type == 'top_claude_usage'")
      aw-top-summary(:fields="claude_top_usage",
                 :namefunc="e => e.data.sorgente",
                 :colorfunc="e => e.data.sorgente",
                 :title="visualizations[type].title",
                 highlightable
                 :highlighted-key="highlightStore.highlightedKey"
                 @select="onSelectRow"
                 with_limit)
    div(v-if="type == 'top_urls'")
      aw-summary(:fields="activityStore.browser.top_urls",
                 :namefunc="e => e.data.url",
                 :colorfunc="e => e.data.$domain",
                 with_limit)
    div(v-if="type == 'top_browser_titles'")
      aw-summary(:fields="activityStore.browser.top_titles",
                 :namefunc="e => e.data.title",
                 :colorfunc="e => e.data.$domain",
                 with_limit)
    div(v-if="type == 'top_editor_files'")
      aw-top-summary(:fields="activityStore.editor.top_files",
                 :namefunc="top_editor_files_namefunc",
                 :hoverfunc="top_editor_files_hoverfunc",
                 :colorfunc="e => e.data.language",
                 :secondary-key-func="top_editor_files_projectfunc",
                 :title="visualizations[type].title",
                 highlightable
                 :highlighted-key="highlightStore.highlightedFile"
                 @select="onSelectEditorFileRow"
                 with_limit)
    div(v-if="type == 'top_excel_files'")
      aw-top-summary(:fields="activityStore.excel.top_files",
                 :namefunc="e => e.data.file",
                 :colorfunc="e => e.data.file",
                 :title="visualizations[type].title",
                 highlightable
                 :highlighted-key="highlightStore.highlightedKey"
                 @select="onSelectRow"
                 with_limit)
    div(v-if="type == 'top_voispeed_contacts'")
      aw-top-summary(:fields="activityStore.voispeed.top_contacts",
                 :namefunc="e => e.data.cliente",
                 :colorfunc="e => e.data.cliente",
                 :title="visualizations[type].title",
                 highlightable
                 :highlighted-key="highlightStore.highlightedKey"
                 @select="onSelectRow"
                 with_limit)
    div(v-if="type == 'top_editor_projects'")
      aw-top-summary(:fields="activityStore.editor.top_projects",
                 :namefunc="top_editor_projects_namefunc",
                 :hoverfunc="top_editor_projects_hoverfunc",
                 :colorfunc="e => e.data.language",
                 :title="visualizations[type].title",
                 highlightable
                 :highlighted-key="highlightStore.highlightedKey"
                 @select="onSelectRow"
                 with_limit)
    div(v-if="type == 'top_categories'")
      aw-category-bar(:apps="top_apps_filtered")
    div(v-if="type == 'category_treemap'")
      aw-category-treemap(:apps="top_apps_filtered")
    div(v-if="type == 'activity_heatmap'")
      aw-activity-heatmap
    div(v-if="type == 'workflow_grid'")
      aw-workflow-grid
    div(v-if="type == 'custom_watcher_view' && !props.templateId")
      aw-custom-watcher-view(:bucket-id="props.bucketId" :title="props.title")
    div(v-if="type == 'custom_watcher_view' && props.templateId")
      iframe.watcher-template-frame(:src="watcherTemplateSrc" frameborder="0")
    div(v-if="type == 'custom_html_module'")
      aw-custom-vis(:visname="props.visname" :title="props.title")
</template>

<style lang="scss">
@import '../style/theme.css';

// Card chrome for every visualization module (top_apps, top_domains,
// top_vpn_clients, sunburst clock, editor stats, ...) — this file's
// root element is shared by all of them, so styling it here reskins
// the whole "+ Add visualization" grid uniformly in one place. Not
// scoped on purpose (matches how this file already worked before —
// .vis-style-dropdown-btn needs to reach Bootstrap-portaled dropdown
// content), so the class name stays specific to avoid leaking.
.vis-card {
  position: relative;
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-xl);
  padding: 16px 18px;
  height: 100%;
  box-sizing: border-box;
}

.vis-card-title {
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-bold);
  color: var(--color-text-faint);
  text-transform: uppercase;
  letter-spacing: var(--letter-spacing-wide);
  margin-bottom: 14px;
}

.vis-style-dropdown-btn {
  position: absolute;
  top: 0.8em;
  right: 0.8em;
  display: flex;
  gap: 6px;

  // Overrides Bootstrap-vue's default button chrome (blue outline,
  // square corners) to match the rest of the module-edit UI (pill
  // buttons in modals.css) instead of standing out as clearly
  // un-themed — same reasoning as the .dropdown-menu override below.
  // !important throughout this block: Bootstrap-vue's own variant
  // classes (.btn-outline-secondary/.btn-outline-danger) turned out to
  // win the cascade over a plain .btn override — verified in the
  // browser (computed color was still Bootstrap's light grey, not
  // theme.css's), so these need to force the point rather than lose a
  // specificity tie-break silently.
  // Icon-only style — no button-chip background, on request: just the
  // glyph, color change is the only feedback (rest vs. hover).
  .btn {
    border: 0 !important;
    background-color: transparent !important;
    color: var(--color-text-dim) !important;
    box-shadow: none !important;
    fill: var(--color-text-dim);

    &:hover,
    &:focus,
    &:active {
      background-color: transparent !important;
      color: var(--color-text) !important;
      box-shadow: none !important;
      fill: var(--color-text);
    }
  }

  .btn-outline-danger {
    color: #d9534f !important;
    fill: #d9534f;

    &:hover,
    &:focus,
    &:active {
      background-color: transparent !important;
      color: #c9302c !important;
      fill: #c9302c;
    }
  }

  // The type-picker dropdown menu — a plain child of the same
  // .dropdown wrapper (not portaled elsewhere in the DOM), so scoping
  // the override to .vis-style-dropdown-btn keeps it from leaking to
  // any other b-dropdown in the app.
  .dropdown-menu {
    background-color: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-elevated);
    padding: 6px;
  }

  .dropdown-item {
    color: var(--color-text-dim);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-sm);

    &:hover,
    &:focus,
    &.active {
      background-color: var(--color-surface2);
      color: var(--color-text);
    }
  }
}

// Themed replacement for Bootstrap's own <b-alert variant="warning">
// (bright yellow, clashed with the dark theme) — same rgba-tinted-
// background pattern already used for warning/danger banners elsewhere
// (e.g. the danger/warning banners used across the Settings views).
.vis-alert {
  margin-top: 8px;
  padding: 10px 14px;
  border-radius: var(--radius-md);
  border: 1px solid var(--color-border);
  font-size: var(--font-size-sm);
  color: var(--color-text);
}

.vis-alert-warning {
  background-color: rgba(211, 163, 85, 0.15);
}

.vis-alert-link {
  color: var(--color-accent1);
}

// Nessuna sincronizzazione automatica dell'altezza col contenuto reale
// dell'iframe (richiederebbe il modello stesso a segnalare la propria
// altezza via postMessage) — stessa limitazione già accettata dai
// "Moduli HTML personalizzati" (CustomVisualization.vue), che non
// stilizzano affatto il proprio iframe. Un'altezza fissa modesta basta
// per un indicatore compatto come "Stato acceso/spento"; un modello
// futuro più corposo potrebbe aver bisogno di un valore diverso qui.
.watcher-template-frame {
  width: 100%;
  height: 60px;
  border: none;
}
</style>

<script lang="ts">
import 'vue-awesome/icons/cog';
import 'vue-awesome/icons/times';
import 'vue-awesome/icons/bars';

import { domainForEvent } from '~/util/browserDomain';
import { getHomeClient } from '~/util/awclient';
import { eventListSignature } from '~/util/timelineBlocks';
import {
  displayNameForApp,
  fallbackIconForApp,
  iconUrlForApp,
  iconColorForApp,
  isHiddenSystemApp,
  isVSCodeApp,
} from '~/util/appNames';
import { projectDisplayName, fileDisplayName } from '~/util/editorNames';

// Claude Desktop already has its own dedicated coverage (the "Uso
// Claude" panel and the Claude Timeline lane, both reading the real
// claude-code-sessions bucket) — excluded here so it doesn't also show
// up as "just another app" in Top Applications/Titles, which would
// both double-count it and break click-to-highlight (its Timeline
// blocks live under a different key than the raw exe name once
// classified into the Claude lane — see HomeTimelineSection.vue).
const isClaudeApp = (app: string) => /claude/i.test(app || '');

import { useActivityStore } from '~/stores/activity';
import { useViewsStore } from '~/stores/views';
import { useSettingsStore } from '~/stores/settings';
import { useTimelineHighlightStore } from '~/stores/timelineHighlight';
import { useAppCategoriesStore } from '~/stores/appCategories';

import moment from 'moment';

export default {
  name: 'aw-selectable-vis',
  props: {
    id: Number,
    type: String,
    props: Object,
    viewId: { type: String, default: '' },
    editable: { type: Boolean, default: true },
  },
  data: function () {
    return {
      activityStore: useActivityStore(),
      settingsStore: useSettingsStore(),
      highlightStore: useTimelineHighlightStore(),
      appCategoriesStore: useAppCategoriesStore(),

      types: [
        'top_apps',
        'top_titles',
        'top_domains',
        'top_urls',
        'top_browser_titles',
        'top_editor_files',
        'top_editor_projects',
        'top_excel_files',
        'top_voispeed_contacts',
        'top_categories',
        'category_treemap',
        'activity_heatmap',
        'workflow_grid',
        'custom_watcher_view',
        'custom_html_module',
        'top_vpn_clients',
        'top_claude_usage',
      ],
      top_editor_files_namefunc: e => fileDisplayName(e.data.file),
      top_editor_files_hoverfunc: e => {
        return 'file: ' + e.data.file + '\n' + 'project: ' + e.data.project;
      },
      // Secondary key for click-to-highlight (see TopSummary.vue's
      // secondaryKeyFunc): the file's owning project, in the same
      // basename form as the VSCode Timeline lane's block key, so
      // toggleFile() sets highlightedKey to something the lane can
      // actually match.
      top_editor_files_projectfunc: e => projectDisplayName(e.data.project),
      top_editor_projects_namefunc: e => projectDisplayName(e.data.project),
      top_editor_projects_hoverfunc: e => e.data.project,
      // Aggregated (summed-per-client) vpn-sessions events for the
      // current query period — no equivalent store field exists for
      // this since VPN tracking is TrackFlow's own addition, not part
      // of upstream ActivityWatch (see BLUEPRINT.md section 7.3).
      vpn_top_clients: [],
      // Aggregated (summed-per-source) Claude usage for the current
      // query period — classified from the existing window/browser
      // buckets, no dedicated watcher (see getClaudeUsage below).
      claude_top_usage: [],
      // Auto-refresh (explicit request, 30s) for the two module types
      // that fetch their own raw events independently of the old
      // activityStore (top_vpn_clients/top_claude_usage — see
      // getVpnTopClients/getClaudeUsage). One interval per module
      // instance (this component is instantiated once per module in
      // the grid, same as its own mounted()/watch already do per-type
      // dispatch), cleared on unmount.
      refreshInterval: null as ReturnType<typeof setInterval> | null,
      // Cheap count+last-id fingerprints (see eventListSignature in
      // HomeTimelineSection.vue for the same pattern/reasoning) — both
      // vpn-sessions and claude-code-sessions are append-only in normal
      // use, so this safely skips re-aggregating on a poll that found
      // nothing new.
      lastVpnSignature: null as string | null,
      lastClaudeSignature: null as string | null,
    };
  },
  computed: {
    visualizations: function () {
      return {
        top_apps: {
          title: this.$t('visualizations.topApps'),
          available: this.activityStore.window.available || this.activityStore.android.available,
        },
        top_titles: {
          title: this.$t('visualizations.topTitles'),
          available: this.activityStore.window.available,
        },
        top_domains: {
          title: this.$t('visualizations.topDomains'),
          available: this.activityStore.browser.available,
        },
        top_urls: {
          title: this.$t('visualizations.topUrls'),
          available: this.activityStore.browser.available,
        },
        top_browser_titles: {
          title: this.$t('visualizations.topBrowserTitles'),
          available: this.activityStore.browser.available,
        },
        top_editor_files: {
          title: this.$t('visualizations.topEditorFiles'),
          // Sempre "disponibile" come top_vpn_clients/top_claude_usage
          // sotto: aw-watcher-vscode è un watcher integrato come
          // qualunque altro (non un'estensione/app esterna opzionale),
          // quindi un bucket assente significa solo "non ancora usato
          // oggi", non "watcher mancante". Il banner
          // missingWatcher/missingWatcherHint sotto era pensato per
          // watcher davvero opzionali (estensione browser, app
          // Android...) — su Editor/Excel/VoiSpeed risultava un falso
          // allarme, mentre l'utente si aspetta lo stesso "Nessun dato
          // per questo periodo" pulito di aw-top-summary che già vede
          // per VPN. hasNoDataForPeriod sotto resta comunque la fonte di
          // verità per il nascondi-se-vuoto.
          available: true,
        },
        top_editor_projects: {
          title: this.$t('visualizations.topEditorProjects'),
          available: true,
        },
        top_excel_files: {
          title: this.$t('visualizations.topExcelFiles'),
          // Stesso ragionamento di top_editor_files sopra.
          available: true,
        },
        top_voispeed_contacts: {
          title: this.$t('visualizations.topVoispeedContacts'),
          // Stesso ragionamento di top_editor_files sopra.
          available: true,
        },
        top_categories: {
          title: this.$t('visualizations.topCategories'),
          // Sempre "disponibile" come VPN/Editor/Excel/VoiSpeed sopra —
          // usa gli stessi eventi finestra di Top Applications (nessun
          // watcher/bucket dedicato), quindi non ha senso un banner
          // "watcher mancante" per questo tipo.
          available: true,
        },
        activity_heatmap: {
          title: this.$t('visualizations.activityHeatmap.title'),
          // Sempre "disponibile" come top_categories sopra — usa gli
          // eventi AFK direttamente, un bucket assente mostra solo lo
          // stato vuoto onesto del componente invece di questo banner.
          available: true,
        },
        category_treemap: {
          title: this.$t('visualizations.categoryTreemap.title'),
          // Stesso ragionamento di top_categories — usa gli stessi
          // eventi finestra di Top Applications, nessun bucket/watcher
          // dedicato.
          available: true,
        },
        workflow_grid: {
          title: this.$t('visualizations.workflowGrid.title'),
          // Sempre "disponibile" come activity_heatmap sopra — si
          // scarica da sé gli eventi finestra/AFK del giorno corrente,
          // un bucket assente mostra solo lo stato vuoto onesto del
          // componente invece di questo banner.
          available: true,
        },
        custom_watcher_view: {
          title: this.$t('visualizations.customWatcherView'),
          available: true,
        },
        custom_html_module: {
          title: this.$t('visualizations.customHtmlModule'),
          available: true,
        },
        top_vpn_clients: {
          title: this.$t('visualizations.topVpnClients'),
          // Always shown as available — unlike the built-in watchers,
          // there's no activityStore.* flag tracking whether the
          // vpn-sessions bucket exists; an empty result just renders
          // aw-top-summary's own honest empty state instead.
          available: true,
        },
        top_claude_usage: {
          title: this.$t('visualizations.topClaudeUsage'),
          // Same reasoning as top_vpn_clients: classified from the
          // window/browser buckets that already exist, not a flag on
          // activityStore, so it's always "available" and just shows
          // an honest empty state if nothing matches.
          available: true,
        },
      };
    },
    has_prerequisites() {
      return this.visualizations[this.type].available;
    },
    // Watcher personalizzati e moduli HTML personalizzati sono creati
    // dall'utente in più copie dello stesso "tipo" — l'etichetta
    // generica per tipo (visualizations[type].title, es. "Watcher
    // personalizzato") non li distingue. Se questa istanza ha un titolo
    // proprio (props.title, impostato alla creazione/selezione), usa
    // quello nell'intestazione della card; altrimenti resta il
    // generico, es. per uno slot appena aggiunto e non ancora
    // configurato.
    cardTitle() {
      if (
        (this.type === 'custom_watcher_view' || this.type === 'custom_html_module') &&
        this.props &&
        this.props.title
      ) {
        return this.props.title;
      }
      return this.visualizations[this.type].title;
    },
    // Stessa risoluzione dell'origine già usata da CustomVisualization.vue
    // (moduli HTML personalizzati) per il proprio iframe — il server
    // aw-server-rust serve sia le API sia queste pagine statiche sulla
    // stessa origine della finestra, tranne in dev mode (porta 27180),
    // dove serve un fallback esplicito.
    watcherTemplateSrc() {
      let origin = document.location.origin;
      if (document.location.port == '27180') {
        origin = 'http://localhost:5666';
      }
      const params = new URLSearchParams({ bucket_id: this.props.bucketId || '' });
      return origin + '/pages/watcher-templates/' + this.props.templateId + '/?' + params.toString();
    },
    // Alcuni moduli restano visibili ma vuoti quando non ci sono dati da
    // mostrare per il periodo — richiesta esplicita dell'utente: in quel
    // caso nascondere del tutto la card invece di mostrarla con lo stato
    // vuoto di aw-top-summary o con il banner "watcher mancante". Copre
    // ENTRAMBI i casi che portano a "niente da mostrare": il bucket non
    // esiste affatto (es. Excel mai aperto su questa macchina) E il
    // bucket esiste ma non ha eventi per il periodo mostrato (es. VS
    // Code usato ieri ma non oggi) — stessa identica logica già usata
    // dalla Timeline per nascondere le corsie senza dati nel giorno
    // mostrato (vedi HomeTimelineSection.vue).
    //
    // Editor (Progetti/File/Lingue) inclusi qui dal 2026-08-12, su
    // segnalazione esplicita dell'utente: prima erano volutamente
    // esclusi ("un bucket assente è un segnale utile da mostrare"), ma
    // l'utente ha fatto notare l'incoerenza con la Timeline (che già
    // nascondeva la corsia VS Code quando non usato in giornata) — Top
    // Domini/URL/Titoli Browser hanno lo stesso identico problema ma
    // restano deliberatamente FUORI da questo fix (stesso utente,
    // stessa sessione: solo Editor per ora, Browser rimandato). Editabile
    // a parte (v-if sotto): mentre modifichi i Moduli, la card resta
    // comunque visibile per poterla rimuovere/spostare.
    hasNoDataForPeriod() {
      if (this.type === 'top_excel_files') {
        return (this.activityStore.excel.top_files || []).length === 0;
      }
      if (this.type === 'top_voispeed_contacts') {
        return (this.activityStore.voispeed.top_contacts || []).length === 0;
      }
      if (this.type === 'top_vpn_clients') {
        return this.vpn_top_clients.length === 0;
      }
      if (this.type === 'top_editor_files') {
        return (this.activityStore.editor.top_files || []).length === 0;
      }
      if (this.type === 'top_editor_projects') {
        return (this.activityStore.editor.top_projects || []).length === 0;
      }
      if (this.type === 'top_categories') {
        return this.top_apps_filtered.length === 0;
      }
      if (this.type === 'category_treemap') {
        // A differenza di top_categories (che mostra comunque una riga
        // "Non categorizzato"), questo modulo ha senso solo se ALMENO
        // un'app ha davvero una categoria assegnata — richiesta
        // esplicita dell'utente: "funzionerà solo se ci sono app
        // categorizzate". Un treemap con una sola categoria enorme
        // "Non categorizzato" non aggiungerebbe nulla a Top Applications.
        return !this.top_apps_filtered.some(
          (e: any) => e.data.app && this.appCategoriesStore.categoryForApp(e.data.app)
        );
      }
      return false;
    },
    // Estratta dal v-if in cima al template (era inline lì) perché ora
    // serve anche fuori dal template: HomeModulesSection.vue distribuiva
    // i moduli sulle colonne "masonry" PRIMA di sapere se questa card si
    // sarebbe nascosta da sola per mancanza di dati (Excel/VPN/VoiSpeed
    // — vedi hasNoDataForPeriod sopra) — il div.modules-grid-item
    // wrapper restava comunque nel v-for, occupando uno slot vuoto e
    // lasciando un buco visibile nella colonna. Il watcher sotto avvisa
    // il genitore appena questo valore cambia, così può escludere
    // l'elemento dalla distribuzione invece di limitarsi a nascondere
    // la card al suo interno.
    visibile() {
      return (
        this.editable ||
        ((!this.activityStore.buckets.loaded ||
          this.has_prerequisites ||
          !this.settingsStore.hideUnsupportedVisualizations) &&
          !(
            this.settingsStore.hideEmptyModules &&
            this.activityStore.buckets.loaded &&
            this.hasNoDataForPeriod
          ))
      );
    },
    // sunburst_clock/vis_timeline (gli unici due tipi che richiedevano un
    // singolo giorno) sono stati rimossi del tutto — nessun tipo rimasto
    // ha più bisogno di questo vincolo, quindi resta sempre vero.
    supports_period: function () {
      return true;
    },
    // Generic [start, end] for whatever period Activity's toolbar has
    // selected (day/week/month/last7d/...) — unlike timeline_daterange,
    // not forced to a single day, since a "top X" list should follow
    // the same period as every other top_* visualization. Shared by
    // both top_vpn_clients and top_claude_usage.
    usage_query_range: function () {
      if (!this.activityStore.query_options) return null;
      const tp = this.activityStore.query_options.timeperiod;
      const start = moment(tp.start);
      const end = start.clone().add(tp.length[0], tp.length[1]);
      return [start, end];
    },
    // VS Code (e Claude, vedi sotto) hanno una corsia/pannello dedicato
    // (Top Editor Files/Languages/Projects) che copre la loro attività
    // in modo più dettagliato (per progetto/file) — erano entrambi
    // esclusi qui per lo stesso motivo "evita il doppio conteggio", ma
    // richiesta esplicita: mostrarli comunque anche in Top Applications
    // (il totale "l'app aveva il focus" è comunque un dato utile in sé,
    // oltre al dettaglio per progetto/file). Restano esclusi da Top
    // Window Titles sotto, dove il dettaglio per titolo duplicherebbe
    // davvero Uso Claude/Top Editor Files.
    top_apps_filtered: function () {
      // A blank app name means the watcher couldn't identify the
      // process at all — nothing useful to show or click, so it's
      // dropped instead of appearing as a nameless row.
      return (this.activityStore.window.top_apps || []).filter(
        e => e.data.app && !isHiddenSystemApp(e.data.app)
      );
    },
    top_titles_filtered: function () {
      return (this.activityStore.window.top_titles || []).filter(
        e => !isClaudeApp(e.data.app) && !isVSCodeApp(e.data.app) && !isHiddenSystemApp(e.data.app)
      );
    },
  },
  watch: {
    // Avvisa il genitore (HomeModulesSection.vue) appena questa card
    // decide di nascondersi da sola per mancanza di dati, così può
    // escluderla dalla distribuzione "masonry" invece di lasciare uno
    // slot vuoto — vedi il commento su `visibile` più sopra. `immediate`
    // perché il primo giro conta quanto i successivi (activityStore può
    // già avere i dati caricati al mount, non solo dopo un cambiamento).
    visibile: {
      immediate: true,
      handler(v) {
        this.$emit('onVisibilityChange', this.id, v);
      },
    },
    usage_query_range: async function () {
      // Reset both fingerprints first — a different period's events
      // could in theory coincidentally produce the same signature as
      // the old period's, which would wrongly skip the reload (same
      // reasoning as HomeTimelineSection.vue's date/host watchers).
      this.lastVpnSignature = null;
      this.lastClaudeSignature = null;
      await this.getVpnTopClients();
      await this.getClaudeUsage();
    },
    type: async function (newType) {
      if (newType == 'top_vpn_clients') await this.getVpnTopClients();
      if (newType == 'top_claude_usage') await this.getClaudeUsage();
    },
  },
  mounted: async function () {
    if (this.type == 'top_vpn_clients') {
      await this.getVpnTopClients();
    }
    if (this.type == 'top_claude_usage') {
      await this.getClaudeUsage();
    }
    this.refreshInterval = setInterval(() => {
      this.getVpnTopClients();
      this.getClaudeUsage();
    }, 30000);
  },
  beforeDestroy: function () {
    if (this.refreshInterval) clearInterval(this.refreshInterval);
  },
  methods: {
    appDisplayName: displayNameForApp,
    appIcon: fallbackIconForApp,
    appIconUrl: iconUrlForApp,
    appIconColor: iconColorForApp,
    // Toggling from a summary-panel row into the shared highlight
    // store, read by HomeTimelineSection to dim every block except the
    // ones matching this key (see stores/timelineHighlight.ts).
    onSelectRow(key: string) {
      this.highlightStore.toggle(key);
    },
    onSelectTitleRow(payload: { key: string; secondaryKey: string }) {
      this.highlightStore.toggleTitle(payload.secondaryKey, payload.key);
    },
    onSelectEditorFileRow(payload: { key: string; secondaryKey: string }) {
      this.highlightStore.toggleFile(payload.secondaryKey, payload.key);
    },
    onWatcherPropsChange(newProps) {
      if (!this.viewId) return;
      const mergedProps = { ...(this.props || {}), ...newProps };
      useViewsStore().editView({
        view_id: this.viewId,
        el_id: this.id,
        type: this.type,
        props: mergedProps,
      });
    },
    // Fetches vpn-sessions for the current period and sums duration
    // per client, producing the same {duration, data} shape as the
    // store's pre-aggregated top_apps/top_domains fields so it can
    // feed into aw-top-summary unchanged.
    getVpnTopClients: async function () {
      if (this.type != 'top_vpn_clients') return;
      if (!this.usage_query_range) return;
      const [start, end] = this.usage_query_range;

      let events = [];
      try {
        // Detached client — see getHomeClient() for why: the shared
        // $aw instance gets its in-flight requests aborted wholesale
        // whenever the old Activity page's date/period changes.
        events = await getHomeClient().getEvents('vpn-sessions', {
          start: start.toDate(),
          end: end.toDate(),
          limit: -1,
        });
      } catch (e) {
        // Bucket doesn't exist on this server — empty, not an error.
        events = [];
      }

      const signature = eventListSignature([events]);
      if (signature === this.lastVpnSignature) return;
      this.lastVpnSignature = signature;

      const totals = new Map();
      for (const ev of events) {
        // `cliente` is the wire field name written by aw-watcher-vpn
        // (Python side) — kept as-is on the `data` output object below
        // since every consumer (TopSummary/Timeline) reads `.data.cliente`,
        // but the local variable itself can be plain English.
        const client = (ev.data && ev.data.cliente) || 'Sconosciuto';
        totals.set(client, (totals.get(client) || 0) + (ev.duration || 0));
      }
      this.vpn_top_clients = [...totals.entries()]
        .sort((a, b) => b[1] - a[1])
        .map(([client, duration]) => ({ duration, data: { cliente: client } }));
    },
    // Classifies Claude usage across every surface it happens on.
    // CLI *and* Desktop both write the exact same transcript log format
    // (confirmed directly — aw-watcher-claude-code doesn't discriminate
    // by entrypoint, it just reads every ~/.claude/projects/*/*.jsonl),
    // so both come from the real claude-code-sessions bucket now —
    // exact, no guessing. The bucket's own "cliente" field already
    // carries the full label the watcher built ("Claude Desktop: ..." /
    // "Claude Code (CLI): ..."), so it's used as-is here, not
    // re-prefixed. Only claude.ai in the browser stays heuristic
    // (domain match) — the only surface with no local log to read at
    // all, since that chat lives server-side.
    getClaudeUsage: async function () {
      if (this.type != 'top_claude_usage') return;
      if (!this.usage_query_range) return;
      const [start, end] = this.usage_query_range;

      const fetchBucket = async (bucketId: string) => {
        if (!bucketId) return [];
        try {
          // Detached client — see getHomeClient() for why.
          return await getHomeClient().getEvents(bucketId, {
            start: start.toDate(),
            end: end.toDate(),
            limit: -1,
          });
        } catch (e) {
          // Bucket doesn't exist on this host — empty, not an error.
          return [];
        }
      };

      const browserBuckets = this.activityStore.buckets.browser || [];
      const windowBucket = this.activityStore.buckets.window[0];
      const [claudeCodeEvents, windowEvents, ...browserEventLists] = await Promise.all([
        fetchBucket('claude-code-sessions'),
        fetchBucket(windowBucket),
        ...browserBuckets.map(fetchBucket),
      ]);
      const browserEvents = browserEventLists.flat();

      const signature = eventListSignature([claudeCodeEvents, windowEvents, browserEvents]);
      if (signature === this.lastClaudeSignature) return;
      this.lastClaudeSignature = signature;

      const totals = new Map();
      const add = (source: string, duration: number) =>
        totals.set(source, (totals.get(source) || 0) + duration);

      for (const ev of claudeCodeEvents) {
        // `cliente` here is the label aw-watcher-claude-code already
        // built ("Claude Desktop: ..." / "Claude Code (CLI): ...") —
        // same wire field as VPN's, just a different watcher.
        const label = String((ev.data && ev.data.cliente) || 'Sconosciuto');
        add(label, ev.duration || 0);
      }

      // Same fallback signal the Timeline's Claude lane already counts
      // (HomeTimelineSection.vue's claudeWindowEvents) — the Claude
      // Desktop window being focused, with no real session log behind
      // it. Counted under the identical label so this panel's total
      // matches what the Timeline shows instead of silently omitting it
      // (explicit fix: they used to disagree whenever there was window-
      // focus time but no claude-code-sessions event, e.g. Claude
      // Desktop open without an active logged session).
      for (const ev of windowEvents) {
        if (isClaudeApp(String((ev.data && ev.data.app) || ''))) {
          add('Claude Desktop (finestra)', ev.duration || 0);
        }
      }

      for (const ev of browserEvents) {
        const domain = domainForEvent(ev);
        if (domain === 'claude.ai' || domain.endsWith('.claude.ai')) {
          add('Claude.ai (Browser)', ev.duration || 0);
        }
      }

      // `sorgente` is the field name every top_claude_usage consumer
      // (TopSummary's namefunc/colorfunc, Timeline's Claude lane) reads
      // as `.data.sorgente` — kept on the output object, `source` is
      // just the local variable name for it.
      this.claude_top_usage = [...totals.entries()]
        .sort((a, b) => b[1] - a[1])
        .map(([source, duration]) => ({ duration, data: { sorgente: source } }));
    },
  },
};
</script>

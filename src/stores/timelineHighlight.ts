// Small in-memory (never persisted) cross-component signals for the
// Home Timeline, shared with sibling components under App.vue (no
// direct parent/child relation, so a store is needed instead of props —
// same reasoning as `requestedDetailProjectId` in stores/projects.ts):
//
// - `highlightedKey`: which key is currently "selected" from a summary
//   panel (Top Applications, Top Clienti VPN, Uso Claude, ...), so the
//   Timeline can dim every block except the ones matching it.
// - `highlightedTitle`: set together with `highlightedKey` only when the
//   selection came from a Top Window Titles row — `highlightedKey` holds
//   that title's owning app (so the existing per-lane dimming keeps
//   working unchanged), this holds the raw title string. Only the
//   Generale lane knows what to do with it (HomeTimelineSection.vue):
//   instead of leaving the whole matching app block undimmed, it dims
//   the block too and draws a brighter overlay only across the exact
//   sub-ranges where that title was really the active window — a plain
//   app/domain/cliente selection has no notion of "sub-range", so this
//   stays null for those.
// - `highlightedFile`: same idea as `highlightedTitle`, but for a Top
//   Editor Files row — `highlightedKey` holds that file's owning project,
//   this holds the raw file path. Only the VSCode lane
//   (HomeTimelineSection.vue) knows what to do with it, same sub-range
//   overlay treatment as titles.
// - `zoom`: the Giorno/4h/1h zoom level, controlled from the Topbar but
//   read by HomeTimelineSection — moved out of the Timeline component's
//   own local state so the Topbar can drive it.
// - `timelineMode`: "Normale"/"Background" toggle in the Topbar (section
//   7 of BLUEPRINT.md's "Prossimi passi") — same pattern as `zoom`,
//   Topbar drives it, HomeTimelineSection reads it to decide which data
//   fills the Timeline. Not persisted: always starts back on 'normal'.
import { defineStore } from 'pinia';

export type TimelineZoom = 'day' | '4h' | '1h';
export type TimelineMode = 'normal' | 'background';

export const useTimelineHighlightStore = defineStore('timelineHighlight', {
  state: () => ({
    highlightedKey: null as string | null,
    highlightedTitle: null as string | null,
    highlightedFile: null as string | null,
    zoom: 'day' as TimelineZoom,
    timelineMode: 'normal' as TimelineMode,
    // --- INIZIO: hover quadratino "Flusso di lavoro" → due linee in Timeline (funzione sperimentale, facile da rimuovere) ---
    // Intervallo (ISO string, non moment — lo stato di uno store deve
    // restare serializzabile) su cui disegnare due linee verticali nella
    // Timeline, impostato da WorkflowGrid.vue al passaggio del mouse su
    // un quadratino e letto da HomeTimelineSection.vue. Se l'intera
    // funzione va rimossa: basta cancellare questo campo e i blocchi
    // delimitati allo stesso modo in WorkflowGrid.vue/
    // HomeTimelineSection.vue, nessun'altra parte dello store la usa.
    hoveredRange: null as { start: string; end: string } | null,
    // --- FINE ---
  }),
  actions: {
    // Clicking the already-selected row again clears it, so there's an
    // obvious way to get back to "show everything" without a separate
    // "clear" control. A plain toggle (app/domain/cliente/sorgente/
    // project) is never title- or file-scoped, so it always drops
    // whichever sub-filter might have been active — switching selection
    // kind, not layering.
    toggle(key: string) {
      this.highlightedKey = this.highlightedKey === key ? null : key;
      this.highlightedTitle = null;
      this.highlightedFile = null;
    },
    // Same toggle-off-on-repeat behavior as toggle(), keyed on the
    // title instead of the app (a title is unique enough on its own —
    // no need to also compare the app to detect "same row clicked
    // again").
    toggleTitle(app: string, title: string) {
      if (this.highlightedTitle === title) {
        this.highlightedKey = null;
        this.highlightedTitle = null;
      } else {
        this.highlightedKey = app;
        this.highlightedTitle = title;
        this.highlightedFile = null;
      }
    },
    // Mirrors toggleTitle() for a Top Editor Files row: `project` is the
    // file's owning project (becomes highlightedKey, matching the
    // VSCode lane's block key), `file` the raw file path.
    toggleFile(project: string, file: string) {
      if (this.highlightedFile === file) {
        this.highlightedKey = null;
        this.highlightedFile = null;
      } else {
        this.highlightedKey = project;
        this.highlightedFile = file;
        this.highlightedTitle = null;
      }
    },
    clear() {
      this.highlightedKey = null;
      this.highlightedTitle = null;
      this.highlightedFile = null;
    },
    setZoom(zoom: TimelineZoom) {
      this.zoom = zoom;
    },
    setTimelineMode(mode: TimelineMode) {
      this.timelineMode = mode;
    },
    // --- INIZIO: hover quadratino "Flusso di lavoro" → due linee in Timeline ---
    setHoveredRange(start: string, end: string) {
      this.hoveredRange = { start, end };
    },
    clearHoveredRange() {
      this.hoveredRange = null;
    },
    // --- FINE ---
  },
});

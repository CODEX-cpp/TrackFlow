<template lang="pug">
div.top-summary
  transition(name="fade" mode="out-in")
    div.top-summary-empty(v-if="!fields || fields.length === 0" key="empty") {{ $t('visualizations.topSummary.noData') }}
    div(v-else key="rows")
      component(
        v-for="(e, i) in visibleFields"
        :key="namefunc(e)"
        :is="linkfunc(e) ? 'a' : 'div'"
        :href="linkfunc(e) || undefined"
        class="top-summary-row"
        :class="{ 'top-summary-row-selected': highlightable && highlightedKey === namefunc(e), 'top-summary-row-clickable': highlightable }"
        :title="hoverfunc ? hoverfunc(e) : namefunc(e)"
        @click="onRowClick(e)"
      )
        div.top-summary-row-main
          div.top-summary-icon(v-if="iconUrlFunc || iconfunc")
            img.top-summary-icon-img(
              v-if="iconUrlFunc && !failedIcons[namefunc(e)]"
              :src="iconUrlFunc(e)"
              @error="markIconFailed(namefunc(e))"
              alt=""
            )
            span(v-else) {{ iconfunc ? iconfunc(e) : '🗔' }}
          div.top-summary-dot(v-else :style="{ backgroundColor: colorFor(e) }")
          div.top-summary-name {{ displayfunc ? displayfunc(e) : namefunc(e) }}
          div.top-summary-value {{ formatHoursMinutes(e.duration) }}
        div.top-summary-track
          div.top-summary-bar(:style="{ width: pct(e) + '%', backgroundColor: colorFor(e) }")

      div.top-summary-actions(v-if="with_limit && fields.length > limit")
        div.top-summary-show-more(@click="onShowMoreClick") {{ expanded ? $t('visualizations.topSummary.fullList') : $t('visualizations.topSummary.showDetails') }}
        div.top-summary-show-more(v-if="expanded" @click="onShowLessClick") {{ $t('visualizations.topSummary.showLess') }}

  div.modal-backdrop(v-if="showAllModal" @click="showAllModal = false")
  div.edit-modal.top-summary-modal(v-if="showAllModal")
    div.edit-modal-title {{ title || $t('visualizations.topSummary.fullListTitle') }}
    div.top-summary-modal-list.themed-scroll
      component(
        v-for="(e, i) in fields"
        :key="namefunc(e)"
        :is="linkfunc(e) ? 'a' : 'div'"
        :href="linkfunc(e) || undefined"
        class="top-summary-row"
        :title="hoverfunc ? hoverfunc(e) : namefunc(e)"
      )
        div.top-summary-row-main
          div.top-summary-icon(v-if="iconUrlFunc || iconfunc")
            img.top-summary-icon-img(
              v-if="iconUrlFunc && !failedIcons[namefunc(e)]"
              :src="iconUrlFunc(e)"
              @error="markIconFailed(namefunc(e))"
              alt=""
            )
            span(v-else) {{ iconfunc ? iconfunc(e) : '🗔' }}
          div.top-summary-dot(v-else :style="{ backgroundColor: colorFor(e) }")
          div.top-summary-name {{ displayfunc ? displayfunc(e) : namefunc(e) }}
          div.top-summary-value {{ formatHoursMinutes(e.duration) }}
        div.top-summary-track
          div.top-summary-bar(:style="{ width: pct(e) + '%', backgroundColor: colorFor(e) }")
    div.edit-modal-actions
      div.pill-btn-ghost(@click="showAllModal = false") {{ $t('visualizations.topSummary.close') }}
</template>

<style lang="scss" scoped>
@import '../style/theme.css';
@import '../style/modals.css';

.top-summary-empty {
  font-size: var(--font-size-sm);
  color: var(--color-text-faint);
}

.top-summary-row {
  display: block;
  padding: 8px 2px;
  text-decoration: none;
  cursor: default;
  border-radius: var(--radius-sm);
}

a.top-summary-row {
  cursor: pointer;
}

a.top-summary-row:hover {
  background-color: var(--color-surface2);
}

.top-summary-row-clickable {
  cursor: pointer;
}

.top-summary-row-clickable:hover {
  background-color: var(--color-surface2);
}

// Marks the row whose key is currently highlighted in the Timeline —
// only ever applied when `highlightable` is true (see the prop). Same
// background as :hover on purpose (explicit choice over a border/
// outline), so a selected row reads as "the one you'd land on if you
// hovered it", not as a separately-styled state.
.top-summary-row-selected {
  background-color: var(--color-surface2);
}

.top-summary-row-main {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 6px;
}

.top-summary-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  flex-shrink: 0;
  transition: background-color 0.3s ease;
}

.top-summary-icon {
  width: 12px;
  height: 12px;
  flex-shrink: 0;
  font-size: 10px;
  line-height: 12px;
  text-align: center;
}

.top-summary-icon-img {
  width: 12px;
  height: 12px;
  object-fit: contain;
  display: block;
}

.top-summary-name {
  flex: 1;
  min-width: 0;
  font-size: var(--font-size-sm);
  color: var(--color-text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.top-summary-value {
  flex-shrink: 0;
  font-size: var(--font-size-xs);
  color: var(--color-text-faint);
  font-variant-numeric: tabular-nums;
}

.top-summary-track {
  height: 3px;
  border-radius: 2px;
  background-color: var(--color-surface2);
  overflow: hidden;
}

.top-summary-bar {
  height: 100%;
  border-radius: 2px;
  // The bar's own DOM node is reused across re-renders (same v-for
  // index), so animating width/color here is enough to turn a hard
  // value snap into a smooth glide whenever the underlying data
  // changes (e.g. switching day in the topbar).
  transition: width 0.35s ease, background-color 0.3s ease;
}

.top-summary-actions {
  display: flex;
  gap: 8px;
  margin-top: 8px;
}

.top-summary-show-more {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  text-align: center;
  padding: 9px;
  border-radius: var(--radius-md);
  background-color: var(--color-surface2);
  color: var(--color-text-dim);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-semibold);
  cursor: pointer;
}

.top-summary-show-more:hover {
  filter: brightness(1.15);
}

.top-summary-modal {
  width: 420px;
  max-height: 75vh;
  display: flex;
  flex-direction: column;
}

.top-summary-modal-list {
  overflow-y: auto;
}

// Cross-fade between the empty state and the row list, instead of a
// hard cut, when the underlying period changes (e.g. a day with no
// data followed by one with data, or vice versa).
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease;
}

.fade-enter,
.fade-leave-to {
  opacity: 0;
}
</style>

<script lang="ts">
// Themed, Vue-native replacement for the old D3-based aw-summary
// (visualizations/Summary.vue) — same prop contract (fields/namefunc/
// hoverfunc/colorfunc/linkfunc/limit/with_limit), so it's a drop-in
// swap inside SelectableVisualization.vue's dispatch for specific
// visualization types, without touching Summary.vue/summary.ts itself
// (still used as-is by TopBucketData.vue, SelectableEventView.vue, and
// any visualization type this file doesn't take over).
//
// "Mostra dettagli" behaviour: first click expands the list once
// (double the default limit); a second click — once already
// expanded — opens a popup listing every field instead of expanding
// further. "Mostra meno" collapses back to the default limit.
import { formatHoursMinutes } from '~/util/projectTime';
import { colorVarForName } from '~/util/hashColor';

export default {
  name: 'TopSummary',
  props: {
    fields: Array,
    namefunc: Function,
    hoverfunc: {
      type: Function,
      default: null,
    },
    colorfunc: Function,
    // Optional — when set and it returns a truthy value for a given
    // row, that literal color (e.g. a hex string) is used directly for
    // the dot/bar instead of the hash-per-name lookup below. Used by
    // Top Applications to show each app's real icon-derived color
    // (util/appNames.ts's iconColorForApp) — a null/undefined return
    // (unmapped app, or no dominant color extracted) falls back to
    // colorfunc/colorVarForName same as if this prop weren't set at
    // all, so every other visualization type keeps its old behavior
    // untouched.
    rawColorFunc: {
      type: Function,
      default: null,
    },
    // Optional — when set, used for the visible row label instead of
    // namefunc(e) (e.g. "VS Code" instead of the raw "Code.exe").
    // namefunc keeps being used for the highlight key/hover title/
    // click-to-highlight value, since that has to match the Timeline's
    // block keys, which are built from the raw identity, not a
    // friendly display name.
    displayfunc: {
      type: Function,
      default: null,
    },
    // Optional — when set, renders a real extracted app icon (an
    // <img>, see util/appNames.ts's iconUrlForApp) instead of the plain
    // color dot. Falls back to iconfunc's emoji if the image 404s
    // (unmapped app, or not extracted yet).
    iconUrlFunc: {
      type: Function,
      default: null,
    },
    // Optional — the emoji fallback for when iconUrlFunc isn't set, or
    // its image fails to load. Not a real per-app icon (the browser
    // can't read an .exe's actual icon on its own) — a curated lookup,
    // see util/appNames.ts.
    iconfunc: {
      type: Function,
      default: null,
    },
    linkfunc: {
      type: Function,
      default: () => null,
    },
    limit: {
      type: Number,
      default: 5,
    },
    with_limit: {
      type: Boolean,
      default: false,
    },
    // Optional — only used as the heading of the "show everything"
    // popup. Falls back to a generic label if the caller doesn't pass
    // one (SelectableVisualization.vue does).
    title: {
      type: String,
      default: '',
    },
    // Only true for the types that have a matching Timeline lane (Top
    // Applications/Titles, Top Browser Domains, Top Clienti VPN, Uso
    // Claude) — clicking a row then emits `select` so the caller can
    // highlight the matching blocks there. Types without a lane (Top
    // Categories, editor stats, ...) leave this false, so a click on
    // those rows behaves exactly as before (does nothing extra, or
    // just follows `linkfunc`'s href).
    highlightable: {
      type: Boolean,
      default: false,
    },
    // Currently-selected key from the shared highlight store — only
    // used to draw the "this row is the one lit up in the Timeline"
    // state, not to decide behavior.
    highlightedKey: {
      type: String,
      default: null,
    },
    // Optional — when set, a row click emits `{ key, secondaryKey }`
    // instead of the bare key. Used by Top Window Titles: the Timeline
    // needs both the title (namefunc) AND its owning app to highlight
    // only the sub-ranges of that app's block matching this exact
    // title (see stores/timelineHighlight.ts's toggleTitle). Every
    // other highlightable type leaves this unset and keeps emitting a
    // plain string, unchanged.
    secondaryKeyFunc: {
      type: Function,
      default: null,
    },
  },
  data() {
    return {
      limit_: this.limit,
      expanded: false,
      showAllModal: false,
      // Keys (namefunc(e)) whose real icon <img> 404'd — switches that
      // row to the emoji fallback instead of a broken-image icon.
      failedIcons: {} as Record<string, boolean>,
    };
  },
  computed: {
    visibleFields(): any[] {
      return (this.fields || []).slice(0, this.limit_);
    },
    maxDuration(): number {
      return this.visibleFields.length ? this.visibleFields[0].duration : 1;
    },
  },
  methods: {
    formatHoursMinutes,
    pct(e: any): number {
      if (!this.maxDuration) return 0;
      return Math.round((e.duration / this.maxDuration) * 100);
    },
    colorFor(e: any): string {
      const raw = this.rawColorFunc && this.rawColorFunc(e);
      if (raw) return raw;
      return colorVarForName(String(this.colorfunc(e)));
    },
    onShowMoreClick() {
      if (!this.expanded) {
        this.limit_ = Math.min((this.fields || []).length, this.limit * 2);
        this.expanded = true;
      } else {
        this.showAllModal = true;
      }
    },
    onShowLessClick() {
      this.limit_ = this.limit;
      this.expanded = false;
    },
    onRowClick(e: any) {
      if (!this.highlightable) return;
      if (this.secondaryKeyFunc) {
        this.$emit('select', { key: this.namefunc(e), secondaryKey: this.secondaryKeyFunc(e) });
      } else {
        this.$emit('select', this.namefunc(e));
      }
    },
    markIconFailed(key: string) {
      this.$set(this.failedIcons, key, true);
    },
  },
};
</script>

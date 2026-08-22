<template lang="pug">
div.timeline-section
  div.timeline-head
    div.timeline-title {{ $t('home.timeline.title') }}
    div.timeline-active-time(v-if="activeTimeToday") {{ $t('home.timeline.timeActive') }} {{ activeTimeToday }}

  transition(name="fade" mode="out-in" appear @after-enter="onTransitionAfterEnter")
    div.timeline-empty(v-if="!loading && lanes.every(l => l.blocks.length === 0)" key="empty")
      | {{ $t('home.timeline.empty') }}

    div.timeline-scroll(
      v-else
      key="scroll"
      ref="scrollArea"
      :class="{ 'timeline-scroll-dragging': isDragging }"
      @wheel="onTimelineWheel"
      @mousedown="onTimelineMouseDown"
    )
      div.timeline-inner(:style="{ width: innerWidthPx + 'px' }")
        // Thin green/red "shape of the day" bar, spanning only the
        // first-to-last-activity range (not the padded ruler) — see
        // afkStatusSegments(). Sits above the ruler on request.
        div.afk-status-bar(v-if="afkStatusSegments.length")
          div.afk-status-segment(
            v-for="(seg, i) in afkStatusSegments"
            :key="'afk-' + i"
            :style="{ left: seg.left + 'px', width: seg.width + 'px', backgroundColor: seg.color }"
          )
        div.time-ruler
          div.time-tick(v-for="t in hourTicks" :key="t.hour" :style="{ left: t.left + 'px' }")
            span {{ t.label }}

        // Purely decorative — a diagonal-striped band over the lunch
        // hour (13:00–14:00), clipped to whatever part of it falls
        // inside the current view. Sits behind the lanes/blocks (first
        // in DOM, no z-index) so nothing about interacting with blocks
        // changes — it's just background texture.
        div.lunch-break(v-if="lunchBreakStyle" :style="lunchBreakStyle")

        // Full-height hour gridlines, one per tick in the ruler above —
        // same reasoning as the lunch band: background-only, behind
        // everything else.
        div.hour-gridline(v-for="t in hourTicks" :key="'grid-' + t.hour" :style="{ left: t.left + 'px' }")

        div.now-line(v-if="showNowLine" :style="{ left: nowLeft + 'px' }")

        div.lane(v-for="lane in lanes" :key="lane.key")
          div.lane-label {{ lane.name }}
          div.lane-track(:style="{ height: trackHeight(lane) + 'px' }")
            div.lane-block(
              v-for="(block, i) in lane.blocks"
              :key="lane.key + '-' + block.key + '-' + i"
              :class="{ 'lane-block-dimmed': shouldDimBlock(lane, block), 'lane-block-selected': isBlockSelected(lane, block), 'lane-block-instant': isWheelZooming }"
              :style="{ left: block.left + 'px', width: block.width + 'px', top: rowTop(block.row) + 'px', backgroundColor: block.color }"
              @click="openBlock(lane, block)"
              @mouseenter="hoverBlock(lane, block, $event)"
              @mousemove="moveTooltip($event)"
              @mouseleave="hoveredBlock = null"
            )
              // Nome + icona dentro il blocco stesso, ma solo quando ci
              // stanno per davvero — richiesta esplicita dell'utente:
              // niente troncamento a metà parola, un blocco troppo
              // stretto resta un semplice rettangolo colorato (il
              // tooltip al passaggio del mouse mostra comunque nome e
              // orario). blockLabelFits() confronta la larghezza VERA
              // del testo (misurata via canvas, non stimata) con
              // block.width, quindi si ricalcola da sé ad ogni tick di
              // zoom — block.width è già reattivo (vedi pxPerMinute),
              // niente da agganciare a parte.
              template(v-if="blockLabelFits(block)")
                img.lane-block-icon(
                  v-if="!failedIcons[block.key]"
                  :src="iconUrlForApp(block.key)"
                  @error="markIconFailed(block.key)"
                  alt=""
                )
                span.lane-block-icon-fallback(v-else) {{ fallbackIconForApp(block.key) }}
                span.lane-block-label(:class="{ 'lane-block-label-dark': isLightColor(block.color) }") {{ displayForKey(block.key) }}
            // Only for the Generale lane, only when the current
            // highlight came from a Top Window Titles row (not a plain
            // app selection) — see titleHighlightRanges(). Drawn above
            // the (dimmed) blocks, with its own click/hover now (used to
            // fall through to the whole app block underneath — explicit
            // bug report: hovering/clicking a highlighted title showed
            // the app's full-day time range instead of just that title's
            // own occurrences).
            template(v-if="lane.key === 'general' && highlightedTitle")
              div.lane-title-highlight(
                v-for="(seg, i) in titleHighlightRanges"
                :key="'title-hl-' + i"
                :class="{ 'lane-block-instant': isWheelZooming }"
                :style="{ left: seg.left + 'px', width: seg.width + 'px', top: seg.top + 'px', backgroundColor: seg.color }"
                @click="openSubBlock(lane, seg)"
                @mouseenter="hoverSubBlock(seg, $event)"
                @mousemove="moveTooltip($event)"
                @mouseleave="hoveredBlock = null"
              )
            // Same idea, for the VSCode lane when the highlight came
            // from a Top Editor Files row instead — see
            // fileHighlightRanges().
            template(v-if="lane.key === 'vscode' && highlightedFile")
              div.lane-title-highlight(
                v-for="(seg, i) in fileHighlightRanges"
                :key="'file-hl-' + i"
                :class="{ 'lane-block-instant': isWheelZooming }"
                :style="{ left: seg.left + 'px', width: seg.width + 'px', top: seg.top + 'px', backgroundColor: seg.color }"
                @click="openSubBlock(lane, seg)"
                @mouseenter="hoverSubBlock(seg, $event)"
                @mousemove="moveTooltip($event)"
                @mouseleave="hoveredBlock = null"
              )
            // Same idea, for the Browser lane when the highlight came
            // from a Top Window Titles row on a browser app — see
            // browserTitleHighlightRanges().
            template(v-if="lane.key === 'browser' && highlightedTitle")
              div.lane-title-highlight(
                v-for="(seg, i) in browserTitleHighlightRanges"
                :key="'browser-title-hl-' + i"
                :class="{ 'lane-block-instant': isWheelZooming }"
                :style="{ left: seg.left + 'px', width: seg.width + 'px', top: seg.top + 'px', backgroundColor: seg.color }"
                @click="openSubBlock(lane, seg)"
                @mouseenter="hoverSubBlock(seg, $event)"
                @mousemove="moveTooltip($event)"
                @mouseleave="hoveredBlock = null"
              )

  div.timeline-tooltip(v-if="hoveredBlock" :style="{ left: tooltipX + 'px', top: tooltipY + 'px' }")
    div.timeline-tooltip-name {{ displayForKey(hoveredBlock.key) }}
    div.timeline-tooltip-time {{ formatRange(hoveredBlock.start, hoveredBlock.end) }}

  timeline-block-detail-modal(
    v-if="selectedBlock"
    :block="selectedBlock"
    :lane-name="selectedLane.name"
    :occurrences="selectedOccurrences"
    :occurrences-by-title="selectedOccurrencesByTitle"
    @close="selectedBlock = null; selectedSubOccurrences = null"
  )
</template>

<style lang="scss" scoped>
@import '../style/theme.css';
@import '../style/modals.css';

.timeline-section {
  padding: 20px 28px 24px;
}

.timeline-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}

.timeline-title {
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-bold);
  color: var(--color-text-dim);
}

.timeline-active-time {
  font-size: var(--font-size-sm);
  color: var(--color-text-faint);
}

.timeline-empty {
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-xl);
  padding: 32px;
  text-align: center;
  color: var(--color-text-faint);
  font-size: var(--font-size-sm);
}

.timeline-scroll {
  overflow-x: auto;
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-xl);
  padding: 16px 0 12px;
  cursor: grab;

  // Themed scrollbar instead of the browser default — Firefox via the
  // scrollbar-* properties, Chrome/Edge/Zen via the ::-webkit-* pseudo
  // elements (both needed, neither covers the other).
  scrollbar-width: thin;
  scrollbar-color: var(--color-border) transparent;

  &::-webkit-scrollbar {
    height: 9px;
  }

  &::-webkit-scrollbar-track {
    background: transparent;
  }

  &::-webkit-scrollbar-thumb {
    background-color: var(--color-border);
    border-radius: var(--radius-pill);
  }

  &::-webkit-scrollbar-thumb:hover {
    background-color: var(--color-text-faint);
  }
}

// Active only while a real drag is in progress (see isDragging) — the
// resting "grab" hand above signals the track is draggable at all;
// swapping to "grabbing" while it's actually held is the standard
// affordance for telling the two states apart. user-select: none stops
// the drag from also highlighting the ruler's time labels as text.
.timeline-scroll-dragging {
  cursor: grabbing;
  user-select: none;
}

.timeline-inner {
  position: relative;
  min-width: 100%;
  transition: width 0.3s ease;
}

// "Shape of the day" bar — thin, green (not-afk) / red (afk) segments
// spanning first-to-last activity, sitting right above the time ruler.
.afk-status-bar {
  position: relative;
  height: 3px;
  margin-top: 2px;
  margin-bottom: 12px;
}

.afk-status-segment {
  position: absolute;
  top: 0;
  height: 100%;
  border-radius: var(--radius-pill);
}

.time-ruler {
  position: relative;
  height: 18px;
  margin-bottom: 8px;
}

.time-tick {
  position: absolute;
  top: 0;
  font-size: 10.5px;
  color: var(--color-text-faint);
  border-left: 1px solid var(--color-border);
  padding-left: 4px;
  height: 100%;
}

// Full-height companion to .time-tick above — same left offsets, but
// spanning the whole lanes area instead of just the ruler, so the hour
// boundaries stay readable while scanning down through the lanes.
.hour-gridline {
  position: absolute;
  top: 0;
  bottom: 0;
  border-left: 1px solid var(--color-border);
  opacity: 0.5;
}

// Decorative lunch-break band (13:00–14:00) — diagonal stripes, same
// spirit as the reference mockup. Never affects layout/interaction,
// just a background texture behind the lanes.
.lunch-break {
  position: absolute;
  top: 0;
  bottom: 0;
  background-image: repeating-linear-gradient(
    135deg,
    var(--color-surface2) 0px,
    var(--color-surface2) 6px,
    transparent 6px,
    transparent 14px
  );
  opacity: 0.5;
}

.now-line {
  position: absolute;
  top: 18px;
  bottom: 12px;
  width: 2px;
  background-color: var(--color-accent1);
  z-index: 2;
}

.lane {
  display: flex;
  align-items: center;
  padding: 6px 0;
  border-bottom: 1px solid var(--color-border);
}

.lane:last-child {
  border-bottom: none;
}

.lane-label {
  position: sticky;
  left: 0;
  width: 90px;
  flex-shrink: 0;
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-semibold);
  color: var(--color-text-dim);
  background-color: var(--color-surface);
  padding-left: 24px;
  z-index: 1;
}

.lane-track {
  position: relative;
  flex: 1;
  // Height set inline per lane (see trackHeight()) — one or two rows
  // depending on whether that lane actually has overlapping blocks.
  transition: height 0.2s ease;
}

.lane-block {
  position: absolute;
  height: 22px;
  border-radius: var(--radius-sm);
  min-width: 3px;
  cursor: pointer;
  opacity: 1;
  // Same-key blocks keep a stable :key across re-renders (see the
  // template), so a day change with overlapping keys glides its
  // blocks to their new position/size instead of hard-snapping. `top`
  // included here too: when blocks that overlap change (see rowTop()),
  // a block moving between row 0/1 glides instead of jumping.
  transition: left 0.3s ease, width 0.3s ease, top 0.2s ease, background-color 0.3s ease,
    opacity 0.2s ease;
  // Icona+nome (vedi blockLabelFits() nello script) — flex invece di
  // absolute per il contenuto interno, il blocco stesso resta
  // posizionato in absolute com'era. overflow:hidden come rete di
  // sicurezza: blockLabelFits() dovrebbe già escludere ogni caso in cui
  // il contenuto non entra, ma un font leggermente diverso da quello
  // usato in blockLabelFits() (fallback prima che il font della pagina
  // sia caricato) non deve mai far sforare il testo fuori dal blocco.
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 0 4px;
  overflow: hidden;
}

.lane-block-icon,
.lane-block-icon-fallback {
  width: 14px;
  height: 14px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
}

.lane-block-icon {
  object-fit: contain;
  border-radius: 3px;
}

.lane-block-icon-fallback {
  font-size: 11px;
  line-height: 1;
}

// Colore fisso bianco con leggera ombra invece di un contrasto
// calcolato per-colore — la palette --client-color-1..8 (vedi
// util/hashColor.ts) è già tutta scura/terrosa apposta per i blocchi
// colorati della Timeline, il bianco ci si legge sempre sopra.
.lane-block-label {
  font-size: 11px;
  font-weight: var(--font-weight-semibold);
  color: white;
  text-shadow: 0 1px 2px rgba(0, 0, 0, 0.35);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

// Applicata quando il colore di sfondo del blocco è troppo chiaro per
// il bianco di default (vedi isLightColor() in util/hashColor.ts) —
// richiesta esplicita dell'utente: alcuni colori icona estratti
// automaticamente sono chiari abbastanza da rendere il nome illeggibile
// in bianco. Ombra chiara invece di quella scura sopra, stesso motivo
// (leggibilità) ma speculare: un alone scuro renderebbe il testo nero
// ancora più scuro/pesante invece di aiutarlo a staccarsi dallo sfondo.
.lane-block-label-dark {
  color: rgba(0, 0, 0, 0.82);
  text-shadow: 0 1px 1px rgba(255, 255, 255, 0.35);
}

// Applied to every block that doesn't match the key selected from a
// summary panel (Top Applications, Top Clienti VPN, Uso Claude, ...) —
// see stores/timelineHighlight.ts. Dimmed instead of hidden so the
// rest of the day stays visible for context.
.lane-block-dimmed {
  opacity: 0.18;
}

// Applied to every block/highlight-segment for the duration of a wheel-
// zoom gesture (see isWheelZooming) — overrides .lane-block's/
// .lane-title-highlight's own left/width/top transitions so rapid wheel
// ticks reposition blocks immediately instead of visibly lagging behind
// the cursor while each 0.3s glide keeps getting restarted mid-flight.
.lane-block-instant {
  transition: none;
}

// Same bright-outline treatment as .lane-title-highlight below, reused
// here on request: a block that's the selected one (matches the shared
// highlight, not dimmed) gets the same "spotlight" look instead of just
// standing out by contrast with the dimmed blocks around it. z-index
// explicit on request: in the rare 3+-overlap case (extras double up on
// the last row instead of getting a third row) a dimmed sibling can
// land later in the block list than the selected one and paint over it
// in the same spot, its low opacity washing out the selected block's
// color underneath instead of actually hiding it. Selected always wins
// the stacking now, regardless of which one happens to come later in
// the list.
.lane-block-selected {
  outline: 2px solid var(--color-text);
  outline-offset: 1px;
  z-index: 2;
}

// Drawn on top of a dimmed Generale block when the highlight is
// title-scoped (see titleHighlightRanges()/shouldDimBlock()) — same
// shape as .lane-block, with a bright outline of its own so it stays
// legible even against an already-dark/dim color underneath. Has its
// own click/hover (see openSubBlock()/hoverSubBlock()) — used to fall
// through to the block underneath, which showed the whole app's time
// range instead of just this title's own occurrences.
.lane-title-highlight {
  position: absolute;
  height: 22px;
  border-radius: var(--radius-sm);
  min-width: 3px;
  cursor: pointer;
  outline: 2px solid var(--color-text);
  outline-offset: 1px;
  // Same stacking guarantee as .lane-block-selected above, same reason
  // (3+-overlap edge case) — this is already last in DOM order for its
  // lane so it wins by default, but explicit is safer than relying on
  // source order surviving future template changes.
  z-index: 2;
  transition: left 0.3s ease, width 0.3s ease, top 0.2s ease;
}

.lane-title-highlight:hover {
  filter: brightness(1.2);
  outline: 2px solid var(--color-accent1);
}

.lane-block:hover {
  filter: brightness(1.2);
  outline: 2px solid var(--color-accent1);
  outline-offset: 1px;
}

.timeline-tooltip {
  position: fixed;
  z-index: 70;
  background-color: var(--color-bg-elev);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-elevated);
  padding: 8px 12px;
  pointer-events: none;
  transform: translate(-50%, -110%);
}

.timeline-tooltip-name {
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-bold);
  color: var(--color-text);
  white-space: nowrap;
}

.timeline-tooltip-time {
  font-size: var(--font-size-xs);
  color: var(--color-text-faint);
  white-space: nowrap;
}

// Cross-fade between the empty state and the scroll area, instead of
// a hard cut, when the day changes (e.g. today has data, yesterday
// doesn't — see TopSummary.vue for the same pattern).
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
// Multi-lane timeline for Home — VPN (by client), Claude (by session/
// window), Browser (by domain), VSCode (by project), Excel (by file),
// VoiSpeed (by client), Generale (by app), built from real ActivityWatch
// buckets.
//
// A lane whose bucket doesn't exist on this server (e.g. no browser
// watcher installed) simply renders no blocks — not a fake/placeholder
// state, just genuinely no data.
//
// The block-building/layout math (merge, overlap cleanup, row packing,
// pixel positioning) lives in util/timelineBlocks.ts — split out once
// this file crossed 1000 lines (same reasoning as the Progetti.vue
// decomposition, see BLUEPRINT.md section 7.3), and the click-to-detail
// popup lives in TimelineBlockDetailModal.vue.
import moment from 'moment';
import { invoke } from '@tauri-apps/api/core';
import { formatDuration } from '~/util/projectTime';
import { colorVarForName, isLightColor } from '~/util/hashColor';
import { domainForEvent } from '~/util/browserDomain';
import {
  displayNameForApp,
  isHiddenSystemApp,
  iconColorForApp,
  isVSCodeApp,
  isExcelApp,
  isBrowserApp,
  vscodeTitleDisplayName,
  iconUrlForApp,
  fallbackIconForApp,
} from '~/util/appNames';
import { measureTextWidth } from '~/util/textMeasure';
import { projectDisplayName, fileDisplayName, isKnownEditorValue } from '~/util/editorNames';
import homeActivityRangeMixin from '~/mixins/homeActivityRangeMixin';
import { useTimelineHighlightStore } from '~/stores/timelineHighlight';
import { get_today_with_offset } from '~/util/time';
import { useSettingsStore } from '~/stores/settings';
import { isWatcherEnabled } from '~/util/modulesConfig';
import { useViewsStore } from '~/stores/views';
import {
  Block,
  mergeEventsByKey,
  dropShortOverlappingRanges,
  assignRows,
  blockExtent as computeBlockExtent,
  layoutBlock as computeLayoutBlock,
  rowTop as computeRowTop,
  trackHeight as computeTrackHeight,
  eventListSignature,
  clipEventsToIntervals,
  subtractIntervals,
} from '~/util/timelineBlocks';

// Stessa euristica di isClaudeApp() in SelectableVisualization.vue (non
// condivisa da lì: è locale a quel file) — serve qui per
// shouldDimBlock(), vedi il suo commento.
const isClaudeAppName = (app: string) => /claude/i.test(app || '');

const PX_PER_MINUTE = { day: 2, '4h': 8, '1h': 32 };
// Width of the sticky lane-name column (.lane-label) — the ruler/
// gridlines/lunch-band/now-line live in .timeline-inner's own
// coordinate space (column 0 = the very left edge, under the label),
// while blocks live inside .lane-track, which only starts after this
// column. Everything that isn't inside a .lane-track needs this added
// to its `left` to actually line up with the blocks — found as a real
// (pre-existing) 90px misalignment while wiring up the "fit to width"
// day view below.
const LANE_LABEL_WIDTH = 90;
// Consecutive same-key raw events with a gap under this get merged
// into a single visual block — otherwise rapid app/tab switching would
// render as an unreadable wall of slivers. Raised from 60s to 300s on
// explicit request (under a minute felt like it split up things that
// were clearly "the same instance" too eagerly) — applies to both the
// Timeline blocks and the block-detail popup's "Altre occorrenze" list,
// since both are built from the same merge.
const MERGE_GAP_SECONDS = 300;

// Now that same-key blocks can span over a brief different-key
// interruption (see the merge above), two blocks in the same lane can
// genuinely overlap in time (e.g. a long Zen Browser stretch with a
// short Windows Terminal glance in the middle) — stacking them all on
// one row would just draw one on top of the other. Explicit request:
// lay overlapping blocks out on up to 2 stacked sub-rows per lane
// (matching the reference mockup), earlier-starting block on top.
const MAX_ROWS_PER_LANE = 2;
const BLOCK_HEIGHT = 22;
const ROW_GAP = 3;
const TRACK_PADDING = 2;
// Cleanup threshold, explicit request: a block under 2 minutes that
// time-overlaps another block in the same lane is dropped instead of
// drawn — see util/timelineBlocks.ts's dropShortOverlappingRanges().
const MIN_OVERLAPPING_BLOCK_SECONDS = 120;

// Deve combaciare con .lane-block-label nel CSS (font-size 11px,
// font-weight semibold) — serve a blockLabelFits() per misurare la
// larghezza VERA del nome via canvas, non una stima. weight/size scritti
// a mano invece di leggerli da --font-weight-semibold/getComputedStyle:
// sarebbe un giro (creare un nodo, leggerne lo stile calcolato) solo per
// un valore che nel tema non cambia mai.
const BLOCK_LABEL_FONT = '600 11px -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif';
const BLOCK_ICON_WIDTH = 14;
const BLOCK_ICON_GAP = 4;
// Padding orizzontale del blocco (0 4px nel CSS, quindi 4px per lato) —
// deve combaciare con quello.
const BLOCK_PADDING_H = 8;

interface Lane {
  key: string;
  name: string;
  blocks: Block[];
}

export default {
  name: 'HomeTimelineSection',
  components: {
    'timeline-block-detail-modal': () => import('./TimelineBlockDetailModal.vue'),
  },
  mixins: [homeActivityRangeMixin],
  data() {
    return {
      loading: true,
      lanes: [] as Lane[],
      viewStart: moment(),
      viewEnd: moment(),
      // Tight bounds of the day's actual activity (no 30-minute padding,
      // unlike viewStart/viewEnd) — where the AFK status bar starts/ends.
      // Null when the day has no activity at all (bar isn't rendered).
      activityStart: null as moment.Moment | null,
      activityEnd: null as moment.Moment | null,
      // Raw AFK bucket events for the day, kept around (not just the
      // not-afk intervals used for clipping in load()) so the status bar
      // can render both afk and not-afk segments.
      rawAfkEvents: [] as any[],
      // Eventi grezzi (non clippati) della finestra, tenuti qui solo per
      // calcolare alwaysActiveIntervals sotto (app "sempre attive").
      rawWindowEventsUnclipped: [] as any[],
      // Eventi già filtrati/clippati pronti per costruire le corsie —
      // salvati qui (non solo dentro `lanes`) così il toggle
      // Normale/Background nella Topbar può ricostruire le corsie
      // all'istante, senza un nuovo fetch di rete, quando cambia solo
      // la modalità e non i dati sottostanti.
      laneEventInputs: null as null | {
        vpnEvents: any[];
        claudeCombined: any[];
        browserCombined: any[];
        vscodeCombined: any[];
        excelEvents: any[];
        voispeedEvents: any[];
        windowEvents: any[];
        trayEvents: any[];
        // Una corsia per ogni watcher personalizzato che ha richiesto
        // "mostra su una riga separata nella Timeline" (wizard,
        // CustomModuleWizard.vue) — vedi loadCustomLanes()/rebuildLanes().
        customLanes: { id: string; name: string; events: any[] }[];
      },
      // Icone app fallite (404 — l'app non ha un'icona estratta) per
      // nome normalizzato, stesso pattern di TopSummary.vue — evita di
      // ritentare la stessa immagine rotta ad ogni re-render mostrando
      // invece l'emoji di fallback (vedi fallbackIconForApp).
      failedIcons: {} as Record<string, boolean>,
      hoveredBlock: null as (Block & { laneName?: string }) | null,
      tooltipX: 0,
      tooltipY: 0,
      selectedLaneKey: null as string | null,
      selectedBlock: null as Block | null,
      // Set by openSubBlock() when the selection is a title/file
      // sub-range rather than a whole lane block — see
      // selectedOccurrences(). Null for a normal block selection.
      selectedSubOccurrences: null as Block[] | null,
      // Continuous zoom override driven by the mouse wheel over the
      // Timeline (explicit request — previously the only way to zoom
      // was the Giorno/4h/1h buttons in the Topbar) — the number of
      // minutes visible across the current container width. Null means
      // "use the Topbar's day/4h/1h preset" (unchanged default
      // behavior); set on first wheel use, cleared again by the zoom
      // watcher below whenever a preset button is clicked, so presets
      // always give a clean jump instead of fighting a lingering wheel
      // adjustment.
      wheelZoomMinutes: null as number | null,
      // Sidesteps the pxPerMinute watcher's own "keep the viewport
      // center in place" logic (see its comment) for a wheel-triggered
      // change — onTimelineWheel() already does its own cursor-anchored
      // scroll adjustment, and running both would double-adjust.
      suppressCenterPreserve: false,
      // True for the duration of an active wheel-zoom gesture (cleared
      // by a short debounce once wheel ticks stop arriving) — applies
      // .lane-block-instant (see template/style) so blocks jump straight
      // to their new left/width instead of riding .lane-block's normal
      // 0.3s glide. That glide exists for day-change/highlight-driven
      // repositioning, where it reads as smooth; under rapid wheel
      // ticks it instead reads as lag trailing the cursor (explicit
      // complaint) since every tick restarts the transition before the
      // previous one finishes.
      isWheelZooming: false,
      wheelZoomEndTimer: null as ReturnType<typeof setTimeout> | null,
      // Click-and-drag panning (explicit request) — active only for a
      // gesture that starts on empty track space (see
      // onTimelineMouseDown), so starting directly on a block still
      // behaves like a plain click (openBlock), not a hijacked drag.
      isDragging: false,
      dragStartX: 0,
      dragStartScrollLeft: 0,
      // True once a drag has moved past the click-vs-drag threshold —
      // used on mouseup to swallow the resulting native click (which
      // fires on whatever the cursor released over, not where the drag
      // started) so dragging across a block doesn't also open it.
      dragMoved: false,
      highlightStore: useTimelineHighlightStore(),
      settingsStore: useSettingsStore(),
      // Measured width of .timeline-scroll's viewport — only used to
      // fit the "Giorno" zoom level to the available space (see
      // pxPerMinute below). Re-measured on mount, after each load, and
      // on window resize.
      containerWidth: 0,
      // Raw (filtered, pre-merge) window events behind the Generale
      // lane's blocks — kept around after load() only so
      // titleHighlightRanges() can recompute exact title sub-ranges on
      // demand (see stores/timelineHighlight.ts's highlightedTitle)
      // without re-fetching from the server on every click.
      rawGeneralEvents: [] as any[],
      // Same idea, for the VSCode lane's fileHighlightRanges() — raw
      // (already "unknown"-filtered) editor events, kept around so a
      // Top Editor Files click can recompute exact file sub-ranges
      // without a re-fetch.
      rawEditorEvents: [] as any[],
      // Same idea, for the Browser lane's browserTitleHighlightRanges()
      // — solo gli eventi arrivati dal watcher finestra (nessun URL),
      // vedi isBrowserWindow più sotto.
      rawBrowserWindowEvents: [] as any[],
      // Auto-refresh (explicit request): re-fetch every 30s while this
      // component is mounted, so new activity shows up without the user
      // having to reload the page or bounce the day picker. Cleared in
      // beforeDestroy — never runs once you've navigated away from Home.
      refreshInterval: null as ReturnType<typeof setInterval> | null,
      // Cheap "did anything actually change" fingerprint of the last
      // successful load — see util/timelineBlocks.ts's
      // eventListSignature(). Skips the whole rebuild (merge/layout/
      // color) on a poll that found nothing new, instead of silently
      // re-doing identical work every 30s.
      lastLoadSignature: null as string | null,
    };
  },
  computed: {
    highlightedKey(): string | null {
      return this.highlightStore.highlightedKey;
    },
    highlightedTitle(): string | null {
      return this.highlightStore.highlightedTitle;
    },
    highlightedFile(): string | null {
      return this.highlightStore.highlightedFile;
    },
    // Giorno/4h/1h now lives in the Topbar (see Topbar.vue), shared via
    // the store so both can read/drive it.
    zoom(): 'day' | '4h' | '1h' {
      return this.highlightStore.zoom;
    },
    timelineMode(): 'normal' | 'background' {
      return this.highlightStore.timelineMode;
    },
    totalMinutes(): number {
      return Math.max(1, this.viewEnd.diff(this.viewStart, 'minutes'));
    },
    // "Giorno" is the one zoom level meant to show the *whole* day
    // without scrolling — explicit request, so its density is derived
    // from the actual available width instead of a fixed constant.
    // 4h/1h stay fixed: those exist specifically to zoom *in*, which
    // means requiring horizontal scroll by design.
    pxPerMinute(): number {
      const available = Math.max(0, this.containerWidth - LANE_LABEL_WIDTH);
      // A wheel-zoom override always wins over the Topbar preset while
      // active — see wheelZoomMinutes' own comment.
      if (this.wheelZoomMinutes !== null && available > 0) {
        return available / this.wheelZoomMinutes;
      }
      if (this.zoom === 'day' && this.containerWidth > 0) {
        if (available > 0) return available / this.totalMinutes;
      }
      return PX_PER_MINUTE[this.zoom];
    },
    innerWidthPx(): number {
      return LANE_LABEL_WIDTH + this.totalMinutes * this.pxPerMinute;
    },
    hourTicks(): { hour: number; left: number; label: string }[] {
      const ticks = [];
      const cursor = this.viewStart.clone().startOf('hour');
      if (cursor.isBefore(this.viewStart)) cursor.add(1, 'hour');
      while (cursor.isBefore(this.viewEnd)) {
        ticks.push({
          hour: cursor.hour(),
          left: LANE_LABEL_WIDTH + cursor.diff(this.viewStart, 'minutes') * this.pxPerMinute,
          label: cursor.format('HH:mm'),
        });
        cursor.add(1, 'hour');
      }
      return ticks;
    },
    // Pixel box for the lunch-break band, clipped to the overlap
    // between settingsStore.lunchBreakStart/End (default 13:00–14:00,
    // no settings UI for it yet — see BLUEPRINT.md section 8) and
    // whatever's currently in view — null (no band rendered) if that
    // day's view doesn't reach into the range at all (e.g. a narrow
    // zoomed-in window elsewhere in the day).
    lunchBreakStyle(): Record<string, string> | null {
      const dayStart = moment(this.date, 'YYYY-MM-DD').startOf('day');
      const lunchStart = moment(
        dayStart.format('YYYY-MM-DD') + ' ' + this.settingsStore.lunchBreakStart,
        'YYYY-MM-DD HH:mm'
      );
      const lunchEnd = moment(
        dayStart.format('YYYY-MM-DD') + ' ' + this.settingsStore.lunchBreakEnd,
        'YYYY-MM-DD HH:mm'
      );
      const clampedStart = moment.max(lunchStart, this.viewStart);
      const clampedEnd = moment.min(lunchEnd, this.viewEnd);
      if (!clampedEnd.isAfter(clampedStart)) return null;
      const left =
        LANE_LABEL_WIDTH + clampedStart.diff(this.viewStart, 'minutes') * this.pxPerMinute;
      const width = clampedEnd.diff(clampedStart, 'minutes') * this.pxPerMinute;
      return { left: left + 'px', width: width + 'px' };
    },
    showNowLine(): boolean {
      return this.date === get_today_with_offset(this.settingsStore.startOfDay);
    },
    nowLeft(): number {
      return LANE_LABEL_WIDTH + moment().diff(this.viewStart, 'minutes') * this.pxPerMinute;
    },
    selectedLane(): Lane | null {
      return this.lanes.find(l => l.key === this.selectedLaneKey) || null;
    },
    selectedOccurrences(): Block[] {
      if (!this.selectedBlock) return [];
      // A title/file sub-block (see openSubBlock()) has its own full set
      // of occurrences already computed — those are the OTHER times that
      // exact title/file appeared today, not every occurrence of the
      // whole app/project it happens to sit inside. Explicit bug report:
      // clicking a Top Window Titles selection in the Timeline used to
      // fall straight through to the app block underneath, showing the
      // app's total time instead of just this title's.
      if (this.selectedSubOccurrences) return this.selectedSubOccurrences;
      if (!this.selectedLane) return [];
      return this.selectedLane.blocks.filter(b => b.key === this.selectedBlock.key);
    },
    // Raw (pre-merge) events per corsia, stessa fonte usata da
    // rebuildLanes() per costruire i blocchi (laneEventInputs) — servono
    // qui per ricalcolare al volo, per QUALUNQUE corsia, quali eventi
    // grezzi appartengono al blocco cliccato (vedi
    // selectedOccurrencesByTitle() sotto).
    rawEventsByLane(): Record<string, any[]> {
      if (!this.laneEventInputs) return {};
      const {
        vpnEvents,
        claudeCombined,
        browserCombined,
        vscodeCombined,
        excelEvents,
        voispeedEvents,
        windowEvents,
        customLanes,
      } = this.laneEventInputs;
      const map: Record<string, any[]> = {
        vpn: vpnEvents,
        claude: claudeCombined,
        browser: browserCombined,
        vscode: vscodeCombined,
        excel: excelEvents,
        voispeed: voispeedEvents,
        general: windowEvents,
      };
      customLanes.forEach((l: any) => {
        map[`custom-${l.id}`] = l.events;
      });
      return map;
    },
    // Sostituisce del tutto "Altre occorrenze" (sopra) con la stessa
    // lista di orari ma raggruppata per titolo, per QUALUNQUE corsia —
    // richiesta esplicita, estesa da un primo tentativo solo per la
    // corsia Browser (a sua volta sostituto di un precedente tentativo
    // ancora diverso, un'unica tabella "titolo → durata totale", trovato
    // poco utile e rimosso). Un blocco raggruppa gli eventi per il suo
    // stesso criterio (vedi laneKeyFn — VPN per cliente, Excel per file,
    // Generale/Browser per app...), quindi "Altre occorrenze" mostra
    // tutte le sue sessioni della giornata mescolate — utile sapere
    // QUANDO, ma non a cosa corrispondesse ciascuna. Qui, quando gli
    // eventi grezzi hanno anche un campo `title` distinto (non tutte le
    // corsie ce l'hanno — vedi il fallback in laneKeyFn), ogni titolo
    // diventa una sua sezione con l'elenco vero degli orari in cui è
    // comparso. Copre l'INTERA giornata (come "Altre occorrenze"), non
    // un singolo intervallo clippato.
    selectedOccurrencesByTitle(): { title: string; occurrences: { start: moment.Moment; end: moment.Moment }[] }[] {
      if (!this.selectedBlock || !this.selectedLaneKey || this.selectedSubOccurrences) return [];
      const rawEvents = this.rawEventsByLane[this.selectedLaneKey];
      if (!rawEvents) return [];
      const keyFn = this.laneKeyFn(this.selectedLaneKey);
      const key = this.selectedBlock.key;
      const events = rawEvents.filter((e: any) => keyFn(e) === key);
      // Nessun campo title (la maggior parte delle corsie non-Generale/
      // Browser): tutti gli eventi ricadono sulla stessa chiave (il
      // blocco stesso) — un solo gruppo, scartato subito sotto.
      const segments = mergeEventsByKey(
        events,
        (e: any) => (e.data && e.data.title) || key,
        MERGE_GAP_SECONDS
      ).sort((a, b) => a.start.diff(b.start));

      const groups = new Map<string, { start: moment.Moment; end: moment.Moment }[]>();
      for (const seg of segments) {
        if (!groups.has(seg.key)) groups.set(seg.key, []);
        (groups.get(seg.key) as { start: moment.Moment; end: moment.Moment }[]).push({
          start: seg.start,
          end: seg.end,
        });
      }
      // Un solo gruppo (o nessuno): "Altre occorrenze" al modo consueto
      // è già chiaro così com'è, non serve la sotto-suddivisione.
      if (groups.size < 2) return [];
      return [...groups.entries()]
        .map(([title, occurrences]) => ({ title, occurrences }))
        .sort((a, b) => a.occurrences[0].start.diff(b.occurrences[0].start));
    },
    // Only meaningful together with highlightedKey (see
    // stores/timelineHighlight.ts's toggleTitle — always set as a
    // pair). Re-merges the raw Generale-lane events (same
    // MERGE_GAP_SECONDS as buildBlocks(), so the resulting segments
    // line up with what a user would expect from the already-familiar
    // block-merging behavior), filtered down to the exact title, then
    // positions each merged segment on top of whichever rendered
    // Generale block actually contains it — that block's `row` decides
    // which sub-row the segment lands on, so it stays aligned even
    // when the app itself has blocks split across both sub-rows over
    // the day.
    titleHighlightRanges(): {
      left: number;
      width: number;
      top: number;
      color: string;
      key: string;
      start: moment.Moment;
      end: moment.Moment;
    }[] {
      return this.subRangeHighlight(
        'general',
        this.highlightedTitle,
        this.rawGeneralEvents,
        (e: any) => (e.data.app || e.data.title || 'Sconosciuto') === this.highlightedKey,
        (e: any) => e.data.title
      );
    },
    // Same idea as titleHighlightRanges(), for the VSCode lane: a Top
    // Editor Files click sets highlightedKey to the file's owning
    // project (matching a VSCode lane block) and highlightedFile to the
    // raw file path — this re-merges the raw editor events down to just
    // that file and positions each resulting segment on top of whichever
    // rendered project block actually contains it.
    fileHighlightRanges(): {
      left: number;
      width: number;
      top: number;
      color: string;
      key: string;
      start: moment.Moment;
      end: moment.Moment;
    }[] {
      return this.subRangeHighlight(
        'vscode',
        this.highlightedFile,
        this.rawEditorEvents,
        (e: any) => projectDisplayName(e.data.project) === this.highlightedKey,
        (e: any) => fileDisplayName(e.data.file)
      );
    },
    // Stessa idea, per la corsia Browser: un click su un titolo di
    // "Titoli finestra principali" imposta highlightedKey al browser
    // (e.data.app, che combacia già col block.key dei blocchi derivati
    // dal solo watcher finestra — nessuna estensione) e highlightedTitle
    // al titolo grezzo. Bug reale corretto: prima non esisteva alcun
    // segmento più stretto, quindi l'intero blocco (spesso l'intera
    // giornata di quel browser, tutte le sessioni comprese) risultava
    // sempre "selezionato" a prescindere da quale titolo fosse stato
    // cliccato — vedi shouldDimBlock/highlightMatchesLaneByProcessName.
    browserTitleHighlightRanges(): {
      left: number;
      width: number;
      top: number;
      color: string;
      key: string;
      start: moment.Moment;
      end: moment.Moment;
    }[] {
      return this.subRangeHighlight(
        'browser',
        this.highlightedTitle,
        this.rawBrowserWindowEvents,
        (e: any) => (e.data.app || 'Sconosciuto') === this.highlightedKey,
        (e: any) => e.data.title
      );
    },
    // "Time active: HH:MM" next to the Timeline title (explicit
    // request) — total not-afk duration, straight from the same
    // rawAfkEvents this.load() already fetches for the status bar
    // below, not a separate query. Empty string (hides the label, see
    // template) rather than "00:00" when there's no AFK data at all for
    // this host, same "unknown, don't claim zero" reasoning used
    // elsewhere for missing AFK data.
    //
    // Explicit request: when viewing today, count only from real
    // calendar midnight (00:00) onward — NOT from the offset-aware
    // "inizio giornata" (dayStart/dayEnd above, still used for
    // everything else on this page). Showing the full offset window
    // here read as "impossible" right after midnight (e.g. "1h50 active"
    // at 00:45), since it silently included hours from the evening
    // before. Only applies to today: a past day's total still spans its
    // own full 00:00-24:00, which is unambiguous once that day is over.
    // Intervalli di tempo (finestra attiva) delle app "sempre attive"
    // (Impostazioni → Pattern sempre attivo): trattati come not-afk anche
    // quando l'AFK watcher (basato solo su input tastiera/mouse, vedi
    // aw-watcher-afk-rust) segnala afk — es. una videochiamata dove
    // l'utente è presente ma non tocca mouse/tastiera. La query
    // server-side in queries.ts (canonicalEvents, usata dai moduli Home)
    // applica lo stesso override; qui va rifatto a mano perché questa
    // timeline calcola l'AFK lato client invece di passare da quella
    // query. Usato sia per il clipping in load() sia da activeTimeToday/
    // afkStatusSegments sotto, così tutte e tre le cose restano coerenti.
    alwaysActiveIntervals(): { start: moment.Moment; end: moment.Moment }[] {
      const apps = this.settingsStore.always_active_apps;
      if (!apps || !apps.length || !this.rawWindowEventsUnclipped.length) return [];
      const set = new Set(apps);
      return this.rawWindowEventsUnclipped
        .filter((e: any) => e.data && set.has(e.data.app))
        .map((e: any) => ({
          start: moment(e.timestamp),
          end: moment(e.timestamp).add(e.duration || 0, 'seconds'),
        }));
    },
    activeTimeToday(): string {
      if (!this.rawAfkEvents.length) return '';
      const isToday = this.date === get_today_with_offset(this.settingsStore.startOfDay);
      const cutoff = isToday ? moment().startOf('day') : null;
      const overrideActive = (t: moment.Moment) =>
        this.alwaysActiveIntervals.some((iv: any) => !t.isBefore(iv.start) && !t.isAfter(iv.end));
      const totalSeconds = this.rawAfkEvents
        .filter(
          (e: any) =>
            e.data &&
            (e.data.status === 'not-afk' ||
              (e.data.status === 'afk' && overrideActive(moment(e.timestamp))))
        )
        .reduce((sum: number, e: any) => {
          const start = moment(e.timestamp);
          const end = start.clone().add(e.duration || 0, 'seconds');
          const clampedStart = cutoff ? moment.max(start, cutoff) : start;
          if (!end.isAfter(clampedStart)) return sum;
          return sum + end.diff(clampedStart, 'seconds');
        }, 0);
      const hours = Math.floor(totalSeconds / 3600);
      const minutes = Math.floor((totalSeconds % 3600) / 60);
      const pad = (n: number) => String(n).padStart(2, '0');
      return `${pad(hours)}:${pad(minutes)}`;
    },
    // Thin status bar above the time ruler (explicit request): green
    // where the AFK watcher says the user was active, red where AFK,
    // spanning only from the first to the last activity of the day (not
    // the padded viewStart/viewEnd everything else uses) — so it reads
    // as "the shape of today's work", not "the shape of the chart".
    // Empty when the day has no activity, or no AFK data exists for
    // this host (nothing to color, same "unknown, don't guess"
    // reasoning as clipEventsToIntervals() in load()).
    afkStatusSegments(): { left: number; width: number; color: string }[] {
      if (!this.activityStart || !this.activityEnd) return [];
      const overrideActive = (t: moment.Moment) =>
        this.alwaysActiveIntervals.some((iv: any) => !t.isBefore(iv.start) && !t.isAfter(iv.end));
      const events = this.rawAfkEvents
        .map((e: any) => {
          const start = moment(e.timestamp);
          let status = e.data && e.data.status;
          // App "sempre attive": una ping afk che cade dentro la finestra
          // attiva di un'app spuntata in Impostazioni diventa verde —
          // stesso override applicato a activeTimeToday/load() sopra.
          if (status === 'afk' && overrideActive(start)) status = 'not-afk';
          return {
            start,
            end: moment(e.timestamp).add(e.duration || 0, 'seconds'),
            status,
          };
        })
        .filter((e: any) => (e.status === 'afk' || e.status === 'not-afk') && e.end.isAfter(e.start))
        .sort((a: any, b: any) => a.start.valueOf() - b.start.valueOf());

      // Bridges small gaps between consecutive raw AFK events (watcher
      // poll jitter, brief restarts on module toggle/app resume — a few
      // seconds up to a few minutes) so the bar reads as one continuous
      // strip instead of visibly dashed — explicit bug report: it should
      // only ever look disconnected for a genuine no-data stretch (PC
      // off, watcher not running for a while), never for normal sampling
      // gaps between two segments that are otherwise back-to-back.
      // Threshold matches the watcher's own pulsetime (timeout+poll_time,
      // ~185s default — see aw-watcher-afk-rust/src/main.rs) with
      // headroom, so a real status-change heartbeat still bridges but
      // hours of PC-off time does not.
      const GAP_BRIDGE_THRESHOLD_SECONDS = 5 * 60;
      for (let i = 1; i < events.length; i++) {
        const prev = events[i - 1];
        const cur = events[i];
        const gapSeconds = cur.start.diff(prev.end, 'seconds');
        if (gapSeconds > 0 && gapSeconds <= GAP_BRIDGE_THRESHOLD_SECONDS) {
          prev.end = cur.start;
        }
      }

      const result: { left: number; width: number; color: string }[] = [];
      for (const e of events) {
        const clampedStart = moment.max(e.start, this.activityStart);
        const clampedEnd = moment.min(e.end, this.activityEnd);
        if (!clampedEnd.isAfter(clampedStart)) continue;
        // blockExtent() is the .lane-track-relative helper (used by the
        // lane blocks, which sit inside .lane-track and are already
        // offset by the flex layout there) — the AFK bar lives directly
        // in .timeline-inner instead, alongside the ruler/gridlines/
        // lunch-band/now-line, all of which manually add
        // LANE_LABEL_WIDTH for exactly this reason (see its own comment
        // above). Missing it here shifted every segment left by 90px —
        // negligible at 1h zoom (~3 real minutes) but well over an hour
        // at Giorno zoom, which is why the bar looked progressively more
        // wrong at lower zoom levels instead of just uniformly off.
        const { left, width } = this.blockExtent({ start: clampedStart, end: clampedEnd });
        result.push({
          left: left + LANE_LABEL_WIDTH,
          width,
          color: e.status === 'not-afk' ? 'var(--color-success)' : 'var(--color-danger)',
        });
      }
      return result;
    },
  },
  watch: {
    // Toggle Normale/Background nella Topbar — ricostruisce solo le
    // corsie dai dati già scaricati (rebuildLanes(), niente fetch),
    // istantaneo.
    timelineMode() {
      this.rebuildLanes();
      this.highlightStore.clear();
    },
    date() {
      // Forces load() past the "nothing changed" short-circuit (see
      // eventListSignature) — a different day's events could, in
      // theory, coincidentally produce the same signature as the old
      // day's, which would wrongly skip the reload.
      this.lastLoadSignature = null;
      this.load();
      // A highlight selected on a different day almost never matches
      // anything on the new one — clear it instead of leaving the
      // whole Timeline dimmed for no visible reason.
      this.highlightStore.clear();
    },
    host() {
      this.lastLoadSignature = null;
      this.load();
    },
    // A Topbar preset click always wins over a lingering wheel-zoom
    // override — see wheelZoomMinutes' own comment.
    zoom() {
      this.wheelZoomMinutes = null;
    },
    // Watching pxPerMinute directly (not just `zoom`) matters because
    // it also changes on its own whenever `containerWidth` is measured
    // late — blocks get laid out with whatever pxPerMinute was current
    // during load(), and if the container's real width only becomes
    // known afterwards (e.g. right after the empty↔scroll cross-fade
    // settles — see onTransitionAfterEnter), the cached block positions
    // would otherwise stay stale relative to the ruler, which always
    // reads the latest pxPerMinute live. Caused a few px of avoidable
    // overflow in "Giorno" mode until this was added.
    //
    // Also re-centers the horizontal scroll on whatever moment was
    // visible before the density change (explicit bug report: switching
    // from Giorno to 4h/1h made the Timeline appear to "jump back" to
    // early morning). Nothing here was actually wrong — the same
    // scrollLeft pixel value just points at a very different (much
    // earlier) time once pxPerMinute increases a lot, since the content
    // got proportionally wider. Converts the currently-centered pixel to
    // a time using the OLD density, then scrolls back to that same time
    // under the NEW one.
    pxPerMinute(newVal: number, oldVal: number) {
      if (this.suppressCenterPreserve) {
        this.suppressCenterPreserve = false;
        this.recomputeLayout();
        return;
      }
      const el = this.$refs.scrollArea as HTMLElement | undefined;
      if (el && oldVal > 0) {
        const centerMinutesFromViewStart = (el.scrollLeft + el.clientWidth / 2 - LANE_LABEL_WIDTH) / oldVal;
        const newCenterPx = LANE_LABEL_WIDTH + centerMinutesFromViewStart * newVal;
        this.recomputeLayout();
        this.$nextTick(() => {
          el.scrollLeft = newCenterPx - el.clientWidth / 2;
        });
      } else {
        this.recomputeLayout();
      }
    },
  },
  async mounted() {
    await this.load();
    document.addEventListener('click', this.onDocumentClick, true);
    window.addEventListener('resize', this.measureContainer);
    this.refreshInterval = setInterval(() => this.load(), 30000);
  },
  beforeDestroy() {
    document.removeEventListener('click', this.onDocumentClick, true);
    window.removeEventListener('resize', this.measureContainer);
    if (this.refreshInterval) clearInterval(this.refreshInterval);
    if (this.wheelZoomEndTimer) clearTimeout(this.wheelZoomEndTimer);
    document.removeEventListener('mousemove', this.onTimelineDragMove);
    document.removeEventListener('mouseup', this.onTimelineDragEnd);
  },
  methods: {
    formatDuration,
    displayForKey: displayNameForApp,
    iconUrlForApp,
    fallbackIconForApp,
    isLightColor,
    markIconFailed(key: string) {
      this.$set(this.failedIcons, key, true);
    },
    // true se nome+icona entrano DAVVERO nella larghezza attuale del
    // blocco — richiesta esplicita dell'utente: icona+nome dentro la
    // barra della Timeline solo quando ci stanno, ricalcolato dal vivo
    // durante lo zoom (block.width cambia in tempo reale, vedi
    // pxPerMinute/recomputeLayout). Confronta la larghezza VERA del
    // testo (measureTextWidth, via canvas) contro lo spazio reale
    // rimasto dopo icona+gap+padding — non una stima a percentuale come
    // nel treemap (lì l'area conta, qui conta solo se il testo entra in
    // una riga).
    blockLabelFits(block: Block): boolean {
      const text = this.displayForKey(block.key);
      const textWidth = measureTextWidth(text, BLOCK_LABEL_FONT);
      const needed = BLOCK_ICON_WIDTH + BLOCK_ICON_GAP + textWidth + BLOCK_PADDING_H;
      return block.width >= needed;
    },
    // .timeline-scroll only exists in the DOM once there's data to show
    // (the empty state renders a different element entirely — see the
    // v-if/v-else in the template), so this has to run after the DOM
    // has actually updated post-load, not just after load()'s promise
    // resolves.
    measureContainer() {
      this.$nextTick(() => {
        const el = this.$refs.scrollArea as HTMLElement | undefined;
        if (el) this.containerWidth = el.clientWidth;
      });
    },
    // Mouse-wheel zoom over the Timeline (explicit request — previously
    // the Giorno/4h/1h Topbar buttons were the only way to zoom).
    // `preventDefault()` because the container's own vertical scroll
    // isn't used for anything (the Timeline is a single row of lanes,
    // no vertical overflow) — without it the page itself would also
    // scroll on every tick. Each tick changes how many minutes are
    // visible across the container by 30 (confirmed with the user:
    // 30 minutes, not 30 seconds), clamped between 15 minutes (finer
    // than the 1h preset — a deliberate bonus, not just matching it) and
    // the whole day (same floor as the Giorno preset's fit-to-width).
    // Zooms toward the cursor, not the viewport center: the exact
    // moment under the mouse is computed with the OLD density, then the
    // scroll position is corrected so that same moment stays under the
    // cursor with the NEW one — standard "zoom toward pointer" feel,
    // and the reason suppressCenterPreserve exists (the generic
    // viewport-center logic in the pxPerMinute watcher would otherwise
    // also fire and fight this).
    onTimelineWheel(evt: WheelEvent) {
      evt.preventDefault();
      const el = this.$refs.scrollArea as HTMLElement | undefined;
      if (!el) return;

      // Suppress the glide transition for the whole gesture, not just
      // this one tick — restarting a 0.3s transition on every wheel
      // event is what produced the lag, so this needs to stay off until
      // ticks actually stop, not toggle on and off within one gesture.
      this.isWheelZooming = true;
      if (this.wheelZoomEndTimer) clearTimeout(this.wheelZoomEndTimer);
      this.wheelZoomEndTimer = setTimeout(() => {
        this.isWheelZooming = false;
        this.wheelZoomEndTimer = null;
      }, 200);

      const available = Math.max(0, this.containerWidth - LANE_LABEL_WIDTH);
      if (available <= 0) return;

      const rect = el.getBoundingClientRect();
      const cursorPx = evt.clientX - rect.left + el.scrollLeft;
      const cursorMinutesFromViewStart = (cursorPx - LANE_LABEL_WIDTH) / this.pxPerMinute;

      const WHEEL_STEP_MINUTES = 30;
      const MIN_VISIBLE_MINUTES = 15;
      const maxVisibleMinutes = Math.max(MIN_VISIBLE_MINUTES, this.totalMinutes);
      const currentVisibleMinutes =
        this.wheelZoomMinutes !== null ? this.wheelZoomMinutes : available / this.pxPerMinute;
      // Wheel down/away = zoom out (more minutes visible); up/toward = zoom in.
      const direction = evt.deltaY > 0 ? 1 : -1;
      const newVisibleMinutes = Math.max(
        MIN_VISIBLE_MINUTES,
        Math.min(maxVisibleMinutes, currentVisibleMinutes + direction * WHEEL_STEP_MINUTES)
      );
      if (newVisibleMinutes === currentVisibleMinutes) return;

      const cursorOffsetInViewport = evt.clientX - rect.left;
      this.suppressCenterPreserve = true;
      this.wheelZoomMinutes = newVisibleMinutes;

      this.$nextTick(() => {
        const newPxPerMinute = available / newVisibleMinutes;
        const newCursorPx = LANE_LABEL_WIDTH + cursorMinutesFromViewStart * newPxPerMinute;
        el.scrollLeft = newCursorPx - cursorOffsetInViewport;
      });
    },
    // Click-and-drag panning (explicit request, once zoomed in enough
    // that .timeline-scroll actually overflows). Only starts when the
    // gesture begins on empty track space — see the template's
    // .lane-block/.lane-title-highlight exclusion — so a plain click on
    // a block still opens it, unaffected by this.
    onTimelineMouseDown(evt: MouseEvent) {
      const target = evt.target as HTMLElement;
      if (target.closest && target.closest('.lane-block, .lane-title-highlight')) return;
      const el = this.$refs.scrollArea as HTMLElement | undefined;
      if (!el) return;

      this.isDragging = true;
      this.dragMoved = false;
      this.dragStartX = evt.clientX;
      this.dragStartScrollLeft = el.scrollLeft;
      // Listened on document, not the element itself — the cursor
      // regularly ends up outside .timeline-scroll mid-drag (dragging
      // fast, or toward the edge of the viewport), and panning shouldn't
      // stop just because the mouse briefly left the container.
      document.addEventListener('mousemove', this.onTimelineDragMove);
      document.addEventListener('mouseup', this.onTimelineDragEnd);
    },
    onTimelineDragMove(evt: MouseEvent) {
      if (!this.isDragging) return;
      const el = this.$refs.scrollArea as HTMLElement | undefined;
      if (!el) return;
      const dx = evt.clientX - this.dragStartX;
      // A few pixels of jitter on what was meant as a click shouldn't
      // count as "dragged" (see onTimelineDragEnd's click-swallow).
      if (Math.abs(dx) > 3) this.dragMoved = true;
      el.scrollLeft = this.dragStartScrollLeft - dx;
    },
    onTimelineDragEnd() {
      this.isDragging = false;
      document.removeEventListener('mousemove', this.onTimelineDragMove);
      document.removeEventListener('mouseup', this.onTimelineDragEnd);

      if (this.dragMoved) {
        // The native click that's about to fire lands on whatever the
        // cursor released over (e.g. a block the drag happened to pass
        // over), not where the drag started — without this, panning
        // across a block would also pop its detail open. Swallowed
        // once, in the capture phase, then removed immediately.
        const swallowClick = (e: MouseEvent) => {
          e.stopPropagation();
          e.preventDefault();
          document.removeEventListener('click', swallowClick, true);
        };
        document.addEventListener('click', swallowClick, true);
      }
    },
    // The authoritative measurement: nextTick alone isn't enough when
    // the empty-state and the scroll area cross-fade into each other
    // (mode="out-in") — the new element only actually lands in the DOM
    // once its enter transition finishes, which takes longer than a
    // single Vue render tick. This fires exactly then, straight off the
    // transitioned element, instead of racing it.
    onTransitionAfterEnter(el: HTMLElement) {
      if (el.classList.contains('timeline-scroll')) {
        this.containerWidth = el.clientWidth;
      }
    },
    formatRange(start: moment.Moment, end: moment.Moment): string {
      return `${start.format('HH:mm')} – ${end.format('HH:mm')}`;
    },
    // Clicking anywhere that isn't a highlightable summary-panel row
    // clears the current selection — so leaving the panel's area drops
    // the highlight instead of leaving the Timeline dimmed forever.
    onDocumentClick(evt: MouseEvent) {
      if (!this.highlightStore.highlightedKey) return;
      const target = evt.target as HTMLElement;
      if (target.closest && target.closest('.top-summary-row-clickable')) return;
      // Explicit bug report: this listener runs on the capture phase
      // (registered with `true` below), so it fired and cleared the
      // highlight BEFORE a click on the highlighted title/file overlay
      // itself reached openSubBlock() — which reads
      // titleHighlightRanges()/fileHighlightRanges(), both empty once
      // the highlight is gone. Clicking anywhere inside the Timeline
      // should never clear a highlight that a click on the Timeline
      // itself might still need — only a click genuinely outside both
      // the Timeline and the summary panels should.
      if (target.closest && target.closest('.timeline-scroll')) return;
      // Same reasoning, explicit follow-up request: clicking a Topbar
      // control (1h/4h/Giorno zoom, date navigation, search, ...) while
      // a highlight is active shouldn't drop it either — e.g. zooming to
      // 1h specifically to look more closely at the selected block used
      // to clear the very selection you were trying to get a better look
      // at.
      if (target.closest && target.closest('.topbar')) return;
      this.highlightStore.clear();
    },
    // Fits the view to the day's actual activity — from 30 minutes
    // before the earliest event to 30 minutes after the latest one
    // (capped at "now" for today, so it never extends into an empty
    // future) — instead of a fixed morning-to-now/end-of-day window.
    // Falls back to a reasonable default span when the day has no
    // activity at all, so the empty state still has a sensible ruler
    // instead of a zero-width one.
    computeViewRange(allEvents: any[]) {
      // Stesso confine offset-aware di homeActivityRangeMixin.ts (da cui
      // arrivano gli eventi in `allEvents`) — devono restare identici,
      // altrimenti earliest/latest/now-line vengono clampati contro un
      // giorno diverso da quello dei dati effettivamente caricati.
      const dayStart = this.dayStart;
      const dayEnd = this.dayEnd;
      const isToday = this.date === get_today_with_offset(this.settingsStore.startOfDay);

      const timestamps: moment.Moment[] = [];
      for (const e of allEvents) {
        const start = moment(e.timestamp);
        timestamps.push(start, start.clone().add(e.duration || 0, 'seconds'));
      }

      if (timestamps.length === 0) {
        this.viewStart = dayStart.clone().add(6, 'hours');
        this.viewEnd = isToday ? moment() : dayStart.clone().add(18, 'hours');
        this.activityStart = null;
        this.activityEnd = null;
        return;
      }

      const earliest = moment.min(timestamps);
      const latest = moment.max(timestamps);
      // Clamped to the day's own bounds, same as viewStart/viewEnd below
      // — an event that overlaps midnight (started yesterday, still
      // running into today, or vice versa) is returned in full by the
      // server query even though only part of it falls on this day, so
      // without this an unclamped earliest/latest could sit outside the
      // selected date entirely.
      this.activityStart = moment.max(dayStart, earliest);
      this.activityEnd = moment.min(dayEnd, latest);

      // Always 30 minutes of padding past the last activity — even for
      // today while activity is ongoing (latest is then only seconds/
      // minutes behind "now"), so the now-line sits with breathing room
      // instead of pinned to the right edge. A previous version capped
      // this at moment() for today, which defeated the padding entirely
      // whenever there was recent/ongoing activity (explicit fix).
      this.viewStart = moment.max(dayStart, earliest.clone().subtract(30, 'minutes'));
      const viewEnd = latest.clone().add(30, 'minutes');
      this.viewEnd = moment.min(dayEnd, viewEnd);

      if (this.viewEnd.isBefore(this.viewStart)) {
        this.viewEnd = this.viewStart.clone().add(1, 'hour');
      }
    },
    async load() {
      const [
        vpnEventsRaw,
        claudeEventsRaw,
        rawWindowEventsUnclipped,
        browserEventsRaw,
        rawEditorEventsUnclipped,
        rawExcelEventsUnclipped,
        rawVoispeedEventsUnclipped,
        afkEvents,
        trayEventsRaw,
      ] = await Promise.all([
        this.fetchEvents('vpn-sessions', this.dayStart, this.dayEnd),
        this.fetchEvents('claude-code-sessions', this.dayStart, this.dayEnd),
        this.fetchEvents('aw-watcher-window', this.dayStart, this.dayEnd),
        this.fetchFirstAvailable(
          ['aw-watcher-web-chrome', 'aw-watcher-web-firefox', 'aw-watcher-web-edge'],
          this.dayStart,
          this.dayEnd
        ),
        this.fetchEvents('aw-watcher-vscode', this.dayStart, this.dayEnd),
        this.fetchEvents('aw-watcher-excel', this.dayStart, this.dayEnd),
        this.fetchEvents('voispeed-calls', this.dayStart, this.dayEnd),
        this.fetchEvents('aw-watcher-afk', this.dayStart, this.dayEnd),
        // Modalità Background della Topbar (sezione 7 del blueprint) —
        // fetch sempre eseguito, non solo quando quella modalità è
        // attiva: così passare da Normale a Background è istantaneo
        // (ricostruisce solo le corsie, vedi rebuildLanes()), senza
        // aspettare un nuovo giro di rete.
        this.fetchEvents('tray-apps', this.dayStart, this.dayEnd),
      ]);
      const customLanes = await this.loadCustomLanes();

      // Auto-refresh (every 30s, see mounted()) short-circuits here on a
      // poll that found nothing new — cheap count+last-id fingerprint,
      // not a real diff, but good enough for "did new activity appear"
      // on a single local user's own data. The very first load always
      // proceeds (lastLoadSignature starts null).
      const signature = eventListSignature([
        vpnEventsRaw,
        claudeEventsRaw,
        rawWindowEventsUnclipped,
        browserEventsRaw,
        rawEditorEventsUnclipped,
        rawExcelEventsUnclipped,
        rawVoispeedEventsUnclipped,
        afkEvents,
        trayEventsRaw,
        ...customLanes.map(l => l.events),
      ]);
      if (signature === this.lastLoadSignature) return;
      this.lastLoadSignature = signature;

      this.loading = true;

      // Explicit bug report: a long stretch of Claude Code activity
      // (heartbeats close enough together to merge, see
      // MERGE_GAP_SECONDS) kept showing as one unbroken bar even across
      // a multi-hour real AFK period — nothing here ever intersected
      // watcher events against the AFK bucket. Clips every lane's raw
      // events down to only the not-afk portions before merging, so an
      // AFK gap actually breaks the bar instead of reading as continuous
      // work. No AFK data for this host (e.g. aw-watcher-afk not
      // installed/running) means "unknown", not "everything is AFK" —
      // clipEventsToIntervals() is a no-op when given an empty interval
      // list, deliberately, to avoid hiding all activity in that case.
      this.rawAfkEvents = afkEvents;
      // Usato da alwaysActiveIntervals sotto (app "sempre attive" —
      // Impostazioni → Pattern sempre attivo), e da activeTimeToday/
      // afkStatusSegments per lo stesso override applicato lì.
      this.rawWindowEventsUnclipped = rawWindowEventsUnclipped;

      const notAfkIntervals = afkEvents
        .filter((e: any) => e.data && e.data.status === 'not-afk')
        .map((e: any) => ({
          start: moment(e.timestamp),
          end: moment(e.timestamp).add(e.duration || 0, 'seconds'),
        }))
        .concat(this.alwaysActiveIntervals);
      const clip = (events: any[]) => clipEventsToIntervals(events, notAfkIntervals);

      const vpnEvents = clip(vpnEventsRaw);
      const claudeEvents = clip(claudeEventsRaw);
      const rawWindowEvents = clip(rawWindowEventsUnclipped);
      const browserEvents = clip(browserEventsRaw);
      const rawEditorEvents = clip(rawEditorEventsUnclipped);
      const excelEvents = clip(rawExcelEventsUnclipped);
      // Non clippati sull'AFK, stesso motivo di trayEvents più sotto: una
      // telefonata VoiSpeed non richiede di toccare mouse/tastiera, quindi
      // clippare sugli intervalli not-afk farebbe sparire proprio le
      // chiamate più lunghe e silenziose (l'uso reale più comune).
      const voispeedEvents = rawVoispeedEventsUnclipped;

      // The Claude Desktop app itself shows up in the raw window bucket
      // like any other app (e.g. "claude.exe") — pulled out here so it
      // lands in the Claude lane instead of getting lost in Generale
      // among every other app, same reasoning as the "Uso Claude"
      // summary panel (SelectableVisualization.vue). This is a coarser
      // signal than claude-code-sessions (window-open time, not
      // active-chat time) so it's kept under its own generic key rather
      // than merged into a specific session's bar.
      const isClaudeWindow = (e: any) => /claude/i.test((e.data && e.data.app) || '');
      // Bug segnalato dall'utente: "Claude (finestra)" compariva quasi
      // sempre insieme alla barra della sessione vera (stesso orario),
      // due barre praticamente identiche senza che fosse chiaro il
      // perché della seconda. Sottrae qui gli intervalli già coperti da
      // una sessione claude-code-sessions reale — la barra "finestra"
      // resta visibile solo per i tratti in cui Claude Desktop era
      // aperta ma nessuna sessione specifica era in corso (es. app
      // aperta, nessun prompt attivo).
      const sessioniClaudeCoperte = claudeEvents.map((e: any) => {
        const start = moment(e.timestamp);
        return { start, end: start.clone().add(e.duration || 0, 'seconds') };
      });
      const claudeWindowEvents = subtractIntervals(
        rawWindowEvents.filter(isClaudeWindow),
        sessioniClaudeCoperte
      );
      // Same idea, for VS Code: pulled out of Generale so its time
      // lands in the VSCode lane instead — explicit fix, previously VS
      // Code's window-focus time stayed in Generale while its editor
      // heartbeats (file/project) went to a separate "vscode" lane,
      // showing the same activity split across two bars and making a
      // Top Applications click highlight the wrong one.
      const isVSCodeWindow = (e: any) => isVSCodeApp((e.data && e.data.app) || '');
      const vscodeWindowEvents = rawWindowEvents.filter(isVSCodeWindow);
      // Excel: a differenza di VS Code, il watcher dedicato
      // (aw-watcher-excel) legge già la finestra in primo piano con la
      // stessa identica tecnica del watcher finestra generico — il suo
      // bucket COPRE già per intero il tempo in cui Excel aveva il
      // focus, non serve nessun merge con un fallback finestra (a
      // differenza di VSCode, dove gli heartbeat editor e il focus
      // finestra sono due segnali distinti con possibili buchi). Basta
      // escludere Excel da Generale, la corsia dedicata usa
      // direttamente excelEvents.
      const isExcelWindow = (e: any) => isExcelApp((e.data && e.data.app) || '');
      // Pulizia richiesta esplicitamente dall'utente: un browser genera
      // un evento nuovo ad ogni cambio di titolo (spesso ogni singola
      // scheda/pagina), che in Generale si vedeva come un muro di
      // blocchi quasi identici (es. episodio dopo episodio dello stesso
      // sito). Il dominio VERO richiederebbe l'estensione browser
      // ufficiale di ActivityWatch (bucket web.tab.current) — non
      // praticabile qui perché quell'estensione parla HTTP con una
      // porta reale, mentre questo server gira solo in-process (vedi
      // build_app_server in lib.rs); browserEvents (sotto, dati veri
      // dall'estensione se mai disponibili) resta comunque nella
      // pipeline per quando/se un giorno tornasse utilizzabile. Con
      // solo il watcher finestra a disposizione (app/title, nessun
      // URL), la corsia dedicata raggruppa per nome del browser
      // (vedi rebuildLanes) invece che per singolo titolo — non è il
      // dominio reale, ma toglie comunque il muro di blocchi da
      // Generale.
      const isBrowserWindow = (e: any) => isBrowserApp((e.data && e.data.app) || '');
      const browserWindowEvents = rawWindowEvents.filter(isBrowserWindow);
      // Same system-app exclusion as Top Applications/Titles
      // (SelectableVisualization.vue) — shell/host processes nobody
      // asked to track, kept out of Generale entirely rather than
      // shown as noise.
      const windowEvents = rawWindowEvents.filter(
        e =>
          !isClaudeWindow(e) &&
          !isVSCodeWindow(e) &&
          !isExcelWindow(e) &&
          !isBrowserWindow(e) &&
          !isHiddenSystemApp((e.data && e.data.app) || '')
      );
      this.rawGeneralEvents = windowEvents;
      this.rawBrowserWindowEvents = browserWindowEvents;

      // Heartbeats with no real file focused report every field as the
      // literal string "unknown" (see util/editorNames.ts) — dropped
      // before both building lane blocks and keeping the raw list around
      // for fileHighlightRanges(), so neither shows a "Sconosciuto"
      // project that's really just "nothing was focused right then".
      const editorEvents = rawEditorEvents.filter(e => isKnownEditorValue(e.data.project));
      this.rawEditorEvents = editorEvents;

      // Deliberately computed from the *unclipped* events, not the
      // AFK-clipped ones above: the visible range (and activityStart/
      // activityEnd, which the AFK status bar spans) must reflect when
      // activity genuinely happened, not be bottlenecked by however
      // recent the AFK watcher's own last update happens to be. AFK
      // detection is keyboard/mouse input only — a long stretch of
      // reading/waiting during an active Claude Code conversation can
      // itself look "afk" for minutes at a time even though real work
      // (visible in the Claude/window buckets) keeps happening. Using
      // the clipped events here caused the whole view — not just the
      // AFK bar — to silently freeze at wherever AFK last said
      // "not-afk" ended, instead of showing genuinely fresh activity as
      // an accurately-colored (if red) trailing stretch.
      this.computeViewRange([
        ...vpnEventsRaw,
        ...claudeEventsRaw,
        ...rawWindowEventsUnclipped,
        ...browserEventsRaw,
        ...rawEditorEventsUnclipped.filter(e => isKnownEditorValue(e.data.project)),
        ...rawVoispeedEventsUnclipped,
      ]);

      // Non ancora clippati sugli intervalli non-afk (a differenza degli
      // altri): la presenza di un'icona in tray non dipende dal fatto
      // che tu stia toccando mouse/tastiera in quel momento, è
      // letteralmente l'opposto caso d'uso (app che gira sola in
      // sottofondo) — clipparla sull'AFK la farebbe sparire proprio
      // quando è più interessante saperla presente.
      const trayEvents = trayEventsRaw;

      this.laneEventInputs = {
        vpnEvents,
        claudeCombined: [...claudeEvents, ...claudeWindowEvents],
        browserCombined: [...browserEvents, ...browserWindowEvents],
        vscodeCombined: [...editorEvents, ...vscodeWindowEvents],
        excelEvents,
        voispeedEvents,
        windowEvents,
        trayEvents,
        customLanes,
      };
      this.rebuildLanes();
      this.loading = false;
      this.measureContainer();
    },
    // Una corsia per ogni watcher personalizzato con "mostra su una riga
    // separata nella Timeline" attivo (wizard, modalità semplificata —
    // vedi CustomModuleWizard.vue/custom_watchers.rs). Solo i watcher in
    // modalità "interval" possono chiederla: il bucket è sempre
    // custom-watcher-<id> (nessun suffisso host — vedi il commento in
    // aw-watcher-afk-rust/src/main.rs), un watcher "raw"/esperto sceglie
    // il proprio bucket_id liberamente, quindi non è collegabile qui in
    // automatico (timeline_lane resta false per quelli, impostato lato
    // backend).
    async loadCustomLanes(): Promise<{ id: string; name: string; events: any[] }[]> {
      let watchers: { id: string; name: string; timeline_lane: boolean }[] = [];
      try {
        watchers = await invoke('elenca_watcher_personalizzati');
      } catch {
        return [];
      }
      const conLinea = watchers.filter(w => w.timeline_lane);
      return Promise.all(
        conLinea.map(async w => ({
          id: w.id,
          name: w.name,
          events: await this.fetchEvents(`custom-watcher-${w.id}`, this.dayStart, this.dayEnd),
        }))
      );
    },
    // Separata da load() apposta: il toggle Normale/Background nella
    // Topbar deve poter ricostruire le corsie all'istante (nessun nuovo
    // fetch di rete) leggendo gli eventi già scaricati/clippati in
    // laneEventInputs — vedi il watcher su timelineMode più sotto.
    rebuildLanes() {
      if (!this.laneEventInputs) return;
      const {
        vpnEvents,
        claudeCombined,
        browserCombined,
        vscodeCombined,
        excelEvents,
        voispeedEvents,
        windowEvents,
        trayEvents,
        customLanes,
      } = this.laneEventInputs;

      if (this.timelineMode === 'background') {
        this.lanes = [
          {
            key: 'background',
            name: this.$t('home.timeline.laneBackground'),
            // Stesso trattamento colore di Generale: colore reale
            // dell'app quando c'è un'icona nota, altrimenti hash sul
            // nome.
            blocks: this.buildBlocks(
              trayEvents,
              e => e.data.app || 'Sconosciuto',
              key => iconColorForApp(key) || colorVarForName(key)
            ),
          },
        ];
        return;
      }

      this.lanes = [
        {
          key: 'vpn',
          name: this.$t('home.timeline.laneVpn'),
          blocks: this.buildBlocks(vpnEvents, e => e.data.cliente || 'Sconosciuto'),
        },
        {
          key: 'claude',
          name: this.$t('home.timeline.laneClaude'),
          blocks: this.buildBlocks(
            claudeCombined,
            e => (e.data && e.data.cliente) || 'Claude Desktop (finestra)'
          ),
        },
        {
          key: 'browser',
          name: this.$t('home.timeline.laneBrowser'),
          // Dominio vero quando l'evento arriva dall'estensione browser
          // (data.url presente, vedi domainForEvent) — nome del browser
          // stesso quando arriva invece dal watcher finestra (nessun
          // URL disponibile, vedi browserWindowEvents più sopra).
          blocks: this.buildBlocks(browserCombined, e =>
            e.data && e.data.url
              ? domainForEvent(e)
              : (e.data && e.data.app) || (e.data && e.data.title) || 'Sconosciuto'
          ),
        },
        {
          key: 'vscode',
          name: this.$t('home.timeline.laneVscode'),
          // Merges real editor heartbeats (data.project, precise
          // per-file signal) with plain window-focus time (data.title,
          // coarser — VS Code was the focused window but no file
          // heartbeat fired, e.g. just browsing without typing) into
          // one lane, keyed consistently: editor events key off their
          // real project path, window events parse the same project
          // name out of the title when there is one (same title format
          // Top Window Titles' displayfunc used to rely on), falling
          // back to a generic label only when neither is available.
          blocks: this.buildBlocks(
            vscodeCombined,
            e =>
              (e.data.project
                ? projectDisplayName(e.data.project)
                : vscodeTitleDisplayName(e.data.title || '')) || 'VS Code (finestra)'
          ),
        },
        {
          key: 'excel',
          name: this.$t('home.timeline.laneExcel'),
          blocks: this.buildBlocks(excelEvents, e => e.data.file || 'Sconosciuto'),
        },
        {
          key: 'voispeed',
          name: this.$t('home.timeline.laneVoispeed'),
          blocks: this.buildBlocks(voispeedEvents, e => e.data.cliente || 'Sconosciuto'),
        },
        {
          key: 'general',
          name: this.$t('home.timeline.laneGeneral'),
          // Real per-app color (from the app's own icon, see
          // util/appNames.ts's iconColorForApp) when there is one,
          // hash-per-name otherwise — e.g. a raw window title (no
          // matching app icon) still falls back cleanly.
          blocks: this.buildBlocks(
            windowEvents,
            e => e.data.app || e.data.title || 'Sconosciuto',
            key => iconColorForApp(key) || colorVarForName(key)
          ),
        },
        // Etichetta di raggruppamento facoltativa: se lo script scrive
        // un campo "etichetta" (o "label") nei suoi dati, i blocchi si
        // raggruppano per quel valore — altrimenti tutta l'attività del
        // watcher forma un'unica barra col suo nome.
        ...customLanes.map(l => ({
          key: `custom-${l.id}`,
          name: l.name,
          blocks: this.buildBlocks(
            l.events,
            (e: any) => (e.data && (e.data.etichetta || e.data.label)) || l.name
          ),
        })),
      ];

      // Nasconde le corsie di feature non usate invece di lasciarle
      // sempre visibili-ma-vuote — richiesta esplicita dell'utente, due
      // condizioni in OR:
      //  1. watcher spento (menu Moduli della tray) E il modulo
      //     corrispondente non è nella lista "Moduli" della Home — la
      //     coppia di segnali che indica "non uso affatto questa
      //     feature", non solo "oggi non ci sono dati". Tenuta separata
      //     dal controllo 2 apposta: un giorno passato con dati veri
      //     resta visibile anche a watcher spento oggi (non si vuole
      //     nascondere la cronologia solo perché la feature è stata
      //     disattivata nel frattempo).
      //  2. nessun blocco per il giorno visualizzato in questo momento
      //     — anche a watcher acceso e modulo visibile, una corsia
      //     vuota oggi non aggiunge niente.
      // "Generale" non ha un watcher dedicato da controllare (è la
      // corsia di raccolta per tutto il resto), resta sempre visibile
      // come prima.
      const elements = useViewsStore().views[0]?.elements || [];
      const hasModule = (...types: string[]) => elements.some((el: any) => types.includes(el.type));
      const laneChecks: Record<string, { watcher: string | null; moduleTypes: string[] }> = {
        vpn: { watcher: 'aw-watcher-vpn', moduleTypes: ['top_vpn_clients'] },
        claude: { watcher: 'aw-watcher-claude-code', moduleTypes: ['top_claude_usage'] },
        // Nessun watcher dedicato per VSCode (i dati arrivano da
        // un'estensione esterna, non da un nostro sidecar con toggle) —
        // il controllo 1 si basa solo sulla visibilità del modulo.
        vscode: { watcher: null, moduleTypes: ['top_editor_projects', 'top_editor_files'] },
        excel: { watcher: 'aw-watcher-excel', moduleTypes: ['top_excel_files'] },
        // VoiSpeed non è un sidecar con toggle nel menu Moduli della tray
        // (vive nel processo Tauri principale, collegato/scollegato dalle
        // Impostazioni, non dal menu Moduli) — solo la visibilità del
        // modulo conta per il controllo 1, stesso caso di VSCode/Browser.
        voispeed: { watcher: null, moduleTypes: ['top_voispeed_contacts'] },
        // A differenza di VSCode/VoiSpeed, questa corsia NON dipende
        // più solo dall'estensione browser (aw-watcher-web-*, oggi non
        // raggiungibile — vedi il commento su browserWindowEvents più
        // sopra): riceve dati anche dal solo watcher finestra, quindi
        // conta come "usata" già solo con quello acceso, come
        // VPN/Claude/Excel.
        browser: {
          watcher: 'aw-watcher-window',
          moduleTypes: ['top_domains', 'top_urls', 'top_browser_titles'],
        },
      };
      this.lanes = this.lanes.filter(lane => {
        const check = laneChecks[lane.key];
        if (!check) return true;
        // Nessun watcher dedicato (VSCode): quel contributo non conta,
        // solo la visibilità del modulo decide questa parte — vedi
        // commento sopra sulla mappa laneChecks.
        const watcherAcceso = check.watcher ? isWatcherEnabled(check.watcher) : false;
        const usataAffatto = watcherAcceso || hasModule(...check.moduleTypes);
        // Impostazione dedicata (Impostazioni > Home): l'utente può
        // disattivare del tutto il nascondere le corsie vuote — vedi
        // stores/settings.ts's hideEmptyTimelineLanes.
        return usataAffatto && (!this.settingsStore.hideEmptyTimelineLanes || lane.blocks.length > 0);
      });
    },
    // Thin wrapper tying util/timelineBlocks.ts's pure pipeline
    // (merge → drop short overlaps → assign rows → position in pixels)
    // to this component's current view range/zoom/constants.
    buildBlocks(
      events: any[],
      keyFn: (e: any) => string,
      colorFn: (key: string) => string = colorVarForName
    ): Block[] {
      const merged = mergeEventsByKey(events, keyFn, MERGE_GAP_SECONDS);
      const visible = merged.filter(
        b => b.end.isAfter(this.viewStart) && b.start.isBefore(this.viewEnd)
      );
      const cleaned = dropShortOverlappingRanges(visible, MIN_OVERLAPPING_BLOCK_SECONDS);
      const withRows = assignRows(cleaned, MAX_ROWS_PER_LANE);
      return withRows.map(b =>
        computeLayoutBlock(b, this.viewStart, this.viewEnd, this.pxPerMinute, colorFn)
      );
    },
    blockExtent(range: { start: moment.Moment; end: moment.Moment }): {
      left: number;
      width: number;
    } {
      return computeBlockExtent(range, this.viewStart, this.viewEnd, this.pxPerMinute);
    },
    rowTop(row: number): number {
      return computeRowTop(row, BLOCK_HEIGHT, ROW_GAP, TRACK_PADDING);
    },
    trackHeight(lane: Lane): number {
      const maxRow = lane.blocks.reduce((m, b) => Math.max(m, b.row), 0);
      return computeTrackHeight(maxRow, BLOCK_HEIGHT, ROW_GAP, TRACK_PADDING);
    },
    // Motore condiviso dietro titleHighlightRanges()/fileHighlightRanges()/
    // browserTitleHighlightRanges() — stessa identica forma tre volte
    // (trova la corsia, trova i blocchi già chiavati su highlightedKey,
    // ri-unisce gli eventi grezzi filtrati per quel sotto-valore, e
    // posiziona ogni segmento sopra al blocco che lo contiene davvero),
    // estratta qui invece di ripeterla una terza volta. `containerMatch`
    // decide quali eventi grezzi appartengono al blocco selezionato
    // (stesso criterio usato per chiavare i blocchi di quella corsia in
    // rebuildLanes), `valueFn` legge il sotto-valore da evidenziare
    // (titolo o file).
    subRangeHighlight(
      laneKey: string,
      highlightValue: string | null,
      rawEvents: any[],
      containerMatch: (e: any) => boolean,
      valueFn: (e: any) => string
    ): {
      left: number;
      width: number;
      top: number;
      color: string;
      key: string;
      start: moment.Moment;
      end: moment.Moment;
    }[] {
      if (!highlightValue || !this.highlightedKey) return [];
      const lane = this.lanes.find(l => l.key === laneKey);
      if (!lane) return [];
      const containerBlocks = lane.blocks.filter(b => b.key === this.highlightedKey);
      if (!containerBlocks.length) return [];

      const matchingEvents = rawEvents.filter(e => containerMatch(e) && valueFn(e) === highlightValue);
      const segments = mergeEventsByKey(matchingEvents, () => highlightValue, MERGE_GAP_SECONDS);

      const result: {
        left: number;
        width: number;
        top: number;
        color: string;
        key: string;
        start: moment.Moment;
        end: moment.Moment;
      }[] = [];
      for (const seg of segments) {
        const container = containerBlocks.find(
          b => !seg.start.isBefore(b.start) && !seg.end.isAfter(b.end)
        );
        // A containing block can be missing if it was dropped by the
        // anti-clutter filter (short overlapping blocks — see
        // util/timelineBlocks.ts's dropShortOverlappingRanges()): in
        // that case the segment has nothing left to visually anchor
        // to, so it's simply omitted instead of inventing a position.
        if (!container) continue;
        const { left, width } = this.blockExtent(seg);
        result.push({
          left,
          width,
          top: this.rowTop(container.row),
          color: container.color,
          key: seg.key,
          start: seg.start,
          end: seg.end,
        });
      }
      return result;
    },
    // Stessa identica funzione di raggruppamento usata da rebuildLanes()
    // per costruire i blocchi di ciascuna corsia (vedi keyFn passata a
    // buildBlocks lì) — duplicata qui apposta invece di farla richiamare
    // da entrambe: rebuildLanes() resta la definizione "viva" di ogni
    // corsia, questa serve solo a ritrovare gli eventi grezzi di un
    // blocco già disegnato (selectedOccurrencesByTitle) senza dipendere
    // dall'ordine di costruzione delle corsie stesse.
    laneKeyFn(laneKey: string): (e: any) => string {
      switch (laneKey) {
        case 'vpn':
        case 'voispeed':
          return (e: any) => (e.data && e.data.cliente) || 'Sconosciuto';
        case 'claude':
          return (e: any) => (e.data && e.data.cliente) || 'Claude Desktop (finestra)';
        case 'browser':
          return (e: any) =>
            e.data && e.data.url
              ? domainForEvent(e)
              : (e.data && e.data.app) || (e.data && e.data.title) || 'Sconosciuto';
        case 'vscode':
          return (e: any) =>
            (e.data.project
              ? projectDisplayName(e.data.project)
              : vscodeTitleDisplayName(e.data.title || '')) || 'VS Code (finestra)';
        case 'excel':
          return (e: any) => e.data.file || 'Sconosciuto';
        case 'general':
          return (e: any) => e.data.app || e.data.title || 'Sconosciuto';
        default: {
          if (laneKey.startsWith('custom-')) {
            const lane = this.lanes.find((l: Lane) => l.key === laneKey);
            return (e: any) =>
              (e.data && (e.data.etichetta || e.data.label)) || (lane ? lane.name : 'Sconosciuto');
          }
          return () => 'Sconosciuto';
        }
      }
    },
    // Top Applications selects by the raw process name (e.g.
    // "claude.exe", "Code.exe") — but the Claude/VSCode lanes key their
    // own blocks by session/project name instead (richer, but a
    // different string), so a plain key === highlightedKey check never
    // matches there. Explicit bug report: selecting Claude/VS Code from
    // Top Applications didn't highlight anything in their own Timeline
    // lane, even with real data on screen, while selecting the exact
    // same app from its OWN dedicated module (Uso Claude/Top Editor
    // Projects — those already key by session/project, matching their
    // lane) worked fine. When the shared highlight looks like a raw
    // process name for one of these two lanes, treat the whole lane as
    // matching instead of comparing individual block keys.
    highlightMatchesLaneByProcessName(lane: Lane): boolean {
      if (!this.highlightedKey) return false;
      if (lane.key === 'claude') return isClaudeAppName(this.highlightedKey);
      if (lane.key === 'vscode') return isVSCodeApp(this.highlightedKey);
      if (lane.key === 'excel') return isExcelApp(this.highlightedKey);
      // La corsia Browser NON serve qui, a differenza delle altre tre:
      // i suoi blocchi (senza estensione) sono chiavati per nome del
      // browser (e.data.app), lo stesso valore grezzo che arriva già
      // da un click su "Applicazioni principali" — combacia da solo col
      // confronto normale block.key === highlightedKey più sotto, come
      // per la corsia Generale. Bug reale corretto: un `return true`
      // qui selezionava l'INTERA corsia (ogni sessione separata nella
      // giornata) anche cliccando un singolo titolo da "Titoli finestra
      // principali", perché il controllo non guardava affatto
      // highlightedTitle.
      return false;
    },
    // Normally a block is dimmed only when its key doesn't match the
    // shared highlight — but when the selection is title- or
    // file-scoped (see titleHighlightRanges()/fileHighlightRanges()),
    // the Generale/VSCode lane's own matching block dims too: the
    // "selected" look now belongs to the narrower overlay segments drawn
    // on top of it, not to the whole block.
    shouldDimBlock(lane: Lane, block: Block): boolean {
      if (!this.highlightedKey) return false;
      if (this.highlightMatchesLaneByProcessName(lane)) return false;
      if (block.key !== this.highlightedKey) return true;
      if (lane.key === 'general') return !!this.highlightedTitle;
      if (lane.key === 'vscode') return !!this.highlightedFile;
      // Stesso trattamento di Generale: un titolo specifico selezionato
      // sposta l'evidenziazione dal blocco intero (tutta la sessione di
      // quel browser) al segmento più stretto disegnato sopra, vedi
      // browserTitleHighlightRanges().
      if (lane.key === 'browser') return !!this.highlightedTitle;
      return false;
    },
    // The bright outline treatment (same look as .lane-title-highlight,
    // explicit request to reuse it here too) — true exactly for a block
    // that's the "whole thing is selected" case: matches the shared
    // highlight and isn't itself dimmed. Excludes the Generale/VSCode
    // lane while a title/file sub-range is active, since there the
    // outline belongs to titleHighlightRanges'/fileHighlightRanges'
    // narrower segments instead (the block underneath is dimmed in that
    // case, see shouldDimBlock).
    isBlockSelected(lane: Lane, block: Block): boolean {
      if (!this.highlightedKey) return false;
      if (this.highlightMatchesLaneByProcessName(lane)) return true;
      return block.key === this.highlightedKey && !this.shouldDimBlock(lane, block);
    },
    // Re-lays out already-fetched blocks for the new zoom level
    // without refetching from the server. Row assignment isn't
    // recomputed here — it only depends on start/end times, which
    // don't change with zoom, just their pixel position.
    recomputeLayout() {
      this.lanes = this.lanes.map(lane => ({
        ...lane,
        blocks: lane.blocks.map(b => ({ ...b, ...this.blockExtent(b) })),
      }));
    },
    hoverBlock(lane: Lane, block: Block, evt: MouseEvent) {
      this.hoveredBlock = block;
      this.tooltipX = evt.clientX;
      this.tooltipY = evt.clientY;
    },
    moveTooltip(evt: MouseEvent) {
      if (!this.hoveredBlock) return;
      this.tooltipX = evt.clientX;
      this.tooltipY = evt.clientY;
    },
    openBlock(lane: Lane, block: Block) {
      this.selectedLaneKey = lane.key;
      this.selectedBlock = block;
      this.selectedSubOccurrences = null;
    },
    // Click on a title/file highlight overlay (titleHighlightRanges()/
    // fileHighlightRanges()) — explicit bug report: this used to have no
    // handler of its own, so the click fell through to the whole app/
    // project block underneath, showing its full-day time range instead
    // of just this title/file's own occurrences.
    openSubBlock(
      lane: Lane,
      seg: { left: number; width: number; color: string; key: string; start: moment.Moment; end: moment.Moment }
    ) {
      this.selectedLaneKey = lane.key;
      this.selectedBlock = {
        key: seg.key,
        start: seg.start,
        end: seg.end,
        row: 0,
        left: seg.left,
        width: seg.width,
        color: seg.color,
      };
      const source =
        lane.key === 'general'
          ? this.titleHighlightRanges
          : lane.key === 'browser'
            ? this.browserTitleHighlightRanges
            : this.fileHighlightRanges;
      this.selectedSubOccurrences = source.map(s => ({
        key: s.key,
        start: s.start,
        end: s.end,
        row: 0,
        left: s.left,
        width: s.width,
        color: s.color,
      }));
    },
    // Same idea as hoverBlock(), for a title/file highlight overlay.
    hoverSubBlock(
      seg: { left: number; width: number; color: string; key: string; start: moment.Moment; end: moment.Moment },
      evt: MouseEvent
    ) {
      this.hoveredBlock = {
        key: seg.key,
        start: seg.start,
        end: seg.end,
        row: 0,
        left: seg.left,
        width: seg.width,
        color: seg.color,
      };
      this.tooltipX = evt.clientX;
      this.tooltipY = evt.clientY;
    },
  },
};
</script>

<template lang="pug">
div.bucket-timeline
  div.bucket-timeline-card
    // .bucket-timeline-measure resta SEMPRE nel DOM (anche a 0 blocchi)
    // — è lui il bersaglio del ResizeObserver che misura containerWidth,
    // da cui dipende pxPerMinute, da cui dipende blocks: se spariva
    // insieme ai blocchi (v-if/v-else sui due rami) la larghezza non
    // veniva mai misurata la prima volta, quindi blocks restava sempre
    // vuoto anche con eventi reali (bug reale, mai visto un solo blocco).
    // Deve anche restare SENZA padding proprio (il padding vive
    // sull'esterna .bucket-timeline-card): tacche e blocchi sono
    // posizionati in "left" px calcolati da questa stessa larghezza,
    // quindi clientWidth deve combaciare esattamente con lo spazio in
    // cui vengono disegnati — un padding qui dentro li avrebbe fatti
    // uscire dal bordo destro della card (bug reale segnalato da un
    // utente, tanto più visibile quanto più lungo l'intervallo scelto).
    div.bucket-timeline-measure(ref="container")
      div.bucket-timeline-empty(v-if="!blocks.length") {{ $t('visualizations.bucketPage.timelineEmpty') }}
      template(v-else)
        div.time-ruler
          div.time-tick(v-for="(t, i) in ticks" :key="i" :style="{ left: t.left + 'px' }")
            span {{ t.label }}
        div.now-line(v-if="showNowLine" :style="{ left: nowLeft + 'px' }")
        div.lane-track(:style="{ height: trackHeightPx + 'px' }")
          div.lane-block(
            v-for="(block, i) in blocks"
            :key="block.key + '-' + block.start.valueOf() + '-' + i"
            :style="{ left: block.left + 'px', width: block.width + 'px', top: rowTop(block.row) + 'px', backgroundColor: block.color }"
            @mouseenter="hoverBlock(block, $event)"
            @mousemove="moveTooltip"
            @mouseleave="hoveredBlock = null"
          )
  div.bucket-timeline-tooltip(v-if="hoveredBlock" :style="{ left: tooltipX + 'px', top: tooltipY + 'px' }")
    div.bucket-timeline-tooltip-name {{ hoveredBlock.key || $t('visualizations.bucketPage.timelineUnnamedEvent') }}
    div.bucket-timeline-tooltip-time {{ formatRange(hoveredBlock.start, hoveredBlock.end) }}
</template>

<style lang="scss" scoped>
@import '../style/theme.css';

// Stessa identica lingua visiva della timeline della Home
// (HomeTimelineSection.vue) — richiesta esplicita: un utente non
// capiva a cosa servisse la vecchia timeline (libreria vis-timeline,
// stile Bootstrap non più coerente col resto dell'app). A differenza
// di quella però qui c'è UNA sola corsia (un solo bucket) e nessuno
// zoom/pan proprio: l'intervallo da mostrare lo sceglie già
// input-timeinterval sopra, quindi i blocchi si adattano sempre alla
// larghezza disponibile invece di poter scorrere oltre.
.bucket-timeline {
  margin: 16px 0;
}

// A differenza di .bucket-timeline-inner (sempre nel DOM, vedi il
// commento nel template) questo è solo testo — il contenitore attorno
// fornisce già sfondo/bordo, non serve ripeterli qui.
.bucket-timeline-empty {
  padding: 8px 0;
  text-align: center;
  color: var(--color-text-faint);
  font-size: var(--font-size-sm);
}

.bucket-timeline-card {
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-xl);
  padding: 16px 16px 12px;
  // Rete di sicurezza contro eventuali arrotondamenti di 1-2px sul
  // blocco più a destra (Math.max(3, ...) in blockExtent) — non deve
  // mai più essere il padding a causare l'uscita, ma un margine di
  // sicurezza in più non fa male.
  overflow: hidden;
}

.bucket-timeline-measure {
  position: relative;
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

.now-line {
  position: absolute;
  top: 18px;
  bottom: 12px;
  width: 2px;
  background-color: var(--color-accent1);
  z-index: 2;
}

.lane-track {
  position: relative;
}

.lane-block {
  position: absolute;
  height: 22px;
  border-radius: var(--radius-sm);
  min-width: 3px;
  cursor: default;
}

.lane-block:hover {
  filter: brightness(1.2);
  outline: 2px solid var(--color-accent1);
  outline-offset: 1px;
}

.bucket-timeline-tooltip {
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

.bucket-timeline-tooltip-name {
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-bold);
  color: var(--color-text);
  white-space: nowrap;
}

.bucket-timeline-tooltip-time {
  font-size: var(--font-size-xs);
  color: var(--color-text-faint);
  white-space: nowrap;
}
</style>

<script lang="ts">
import moment from 'moment';
import { getColorFromString, getTitleAttr } from '~/util/color';
import {
  mergeEventsByKey,
  dropShortOverlappingRanges,
  assignRows,
  layoutBlock,
  rowTop as computeRowTop,
  trackHeight as computeTrackHeight,
  Block,
} from '~/util/timelineBlocks';

// Stesse costanti/euristiche usate dalla timeline della Home (vedi
// HomeTimelineSection.vue) — mantiene lo stesso comportamento visivo
// (eventi ravvicinati uniti in un unico blocco, overlap impilati su al
// massimo 2 righe) senza duplicarne la logica di layout, già estratta
// in util/timelineBlocks.ts.
const MERGE_GAP_SECONDS = 300;
const MAX_ROWS = 2;
const MIN_OVERLAPPING_BLOCK_SECONDS = 120;
const BLOCK_HEIGHT = 22;
const ROW_GAP = 3;
const TRACK_PADDING = 2;

export default {
  name: 'BucketTimeline',
  props: {
    events: { type: Array, required: true },
    daterange: { type: Array, required: true },
    bucket: { type: Object, required: true },
  },
  data() {
    return {
      containerWidth: 0,
      resizeObserver: null as ResizeObserver | null,
      hoveredBlock: null as Block | null,
      tooltipX: 0,
      tooltipY: 0,
    };
  },
  computed: {
    viewStart(): moment.Moment {
      return this.daterange[0];
    },
    viewEnd(): moment.Moment {
      return this.daterange[1];
    },
    totalMinutes(): number {
      return Math.max(1, this.viewEnd.diff(this.viewStart, 'minutes'));
    },
    pxPerMinute(): number {
      return this.containerWidth > 0 ? this.containerWidth / this.totalMinutes : 0;
    },
    blocks(): Block[] {
      if (!this.pxPerMinute) return [];
      const merged = mergeEventsByKey(
        this.events,
        (e: any) => getTitleAttr(this.bucket, e),
        MERGE_GAP_SECONDS
      );
      const cleaned = dropShortOverlappingRanges(merged, MIN_OVERLAPPING_BLOCK_SECONDS);
      const withRows = assignRows(cleaned, MAX_ROWS);
      return withRows.map(r =>
        layoutBlock(r, this.viewStart, this.viewEnd, this.pxPerMinute, getColorFromString)
      );
    },
    trackHeightPx(): number {
      const maxRow = this.blocks.reduce((m, b) => Math.max(m, b.row), 0);
      return computeTrackHeight(maxRow, BLOCK_HEIGHT, ROW_GAP, TRACK_PADDING);
    },
    ticks(): { left: number; label: string }[] {
      const n = 6;
      const sameDay = this.viewStart.isSame(this.viewEnd, 'day');
      const fmt = sameDay ? 'HH:mm' : 'DD/MM HH:mm';
      const out = [];
      for (let i = 0; i <= n; i++) {
        const minutes = (this.totalMinutes * i) / n;
        out.push({
          left: minutes * this.pxPerMinute,
          label: moment(this.viewStart).add(minutes, 'minutes').format(fmt),
        });
      }
      return out;
    },
    showNowLine(): boolean {
      return moment().isBetween(this.viewStart, this.viewEnd);
    },
    nowLeft(): number {
      return moment().diff(this.viewStart, 'minutes') * this.pxPerMinute;
    },
  },
  mounted() {
    this.measure();
    this.resizeObserver = new ResizeObserver(() => this.measure());
    this.resizeObserver.observe(this.$refs.container as HTMLElement);
  },
  beforeDestroy() {
    if (this.resizeObserver) this.resizeObserver.disconnect();
  },
  methods: {
    measure() {
      if (this.$refs.container) {
        this.containerWidth = (this.$refs.container as HTMLElement).clientWidth;
      }
    },
    rowTop(row: number): number {
      return computeRowTop(row, BLOCK_HEIGHT, ROW_GAP, TRACK_PADDING);
    },
    formatRange(start: moment.Moment, end: moment.Moment): string {
      const sameDay = start.isSame(end, 'day');
      const fmt = sameDay ? 'HH:mm:ss' : 'DD/MM HH:mm:ss';
      return `${start.format(fmt)} – ${end.format(fmt)}`;
    },
    hoverBlock(block: Block, evt: MouseEvent) {
      this.hoveredBlock = block;
      this.moveTooltip(evt);
    },
    moveTooltip(evt: MouseEvent) {
      this.tooltipX = evt.clientX;
      this.tooltipY = evt.clientY;
    },
  },
};
</script>

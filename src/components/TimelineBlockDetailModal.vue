<template lang="pug">
div
  div.modal-backdrop(@click="$emit('close')")
  div.edit-modal.block-detail-modal.themed-scroll
    div.edit-modal-title {{ displayName }}
    div.block-detail-meta
      div.block-detail-row
        span.block-detail-label {{ $t('home.timelineBlockDetail.lane') }}
        span.block-detail-value {{ laneName }}
      div.block-detail-row
        span.block-detail-label {{ $t('home.timelineBlockDetail.time') }}
        span.block-detail-value {{ formatRange(block.start, block.end) }}
      div.block-detail-row
        span.block-detail-label {{ $t('home.timelineBlockDetail.duration') }}
        span.block-detail-value {{ formatDuration(block.end.diff(block.start, 'seconds')) }}

    div.block-detail-subhead(v-if="occurrences.length > 1") {{ $t('home.timelineBlockDetail.otherOccurrences') }}
    table.block-detail-table(v-if="occurrences.length > 1")
      tbody
        tr(v-for="(occ, i) in occurrences" :key="i")
          td {{ occ.start.format('HH:mm') }} – {{ occ.end.format('HH:mm') }}
          td {{ formatDuration(occ.end.diff(occ.start, 'seconds')) }}

    div.block-detail-subhead(v-if="titleBreakdown.length") {{ $t('home.timelineBlockDetail.topWindowTitles') }}
    table.block-detail-table(v-if="titleBreakdown.length")
      tbody
        tr(v-for="(t, i) in titleBreakdown" :key="i")
          td {{ t.title }}
          td {{ formatDuration(t.duration) }}

    div.edit-modal-actions
      // Explicit request: no more thumbnails dumped inline here (they
      // read as disconnected from *when* in the block they happened) —
      // a single button instead, opening a dedicated gallery that groups
      // them by which window title was active, so "show me the
      // screenshots of activity X" doesn't require mentally matching
      // timestamps by hand.
      div.pill-btn-ghost(v-if="screenshots.length" @click="showGallery = true") {{ $t('home.timelineBlockDetail.viewScreenshots', { count: screenshots.length }) }}
      div.pill-btn-ghost(@click="$emit('close')") {{ $t('home.timelineBlockDetail.close') }}

  screenshot-gallery-modal(
    v-if="showGallery"
    :screenshots="screenshots"
    :title-segments="titleSegments"
    :display-name="displayName"
    @close="showGallery = false"
  )
</template>

<style lang="scss" scoped>
@import '../style/theme.css';
@import '../style/modals.css';

.block-detail-modal {
  width: 400px;
  max-height: 75vh;
  overflow-y: auto;
}

.block-detail-meta {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-bottom: 6px;
}

.block-detail-row {
  display: flex;
  justify-content: space-between;
  font-size: var(--font-size-sm);
}

.block-detail-label {
  color: var(--color-text-faint);
}

.block-detail-value {
  color: var(--color-text);
  font-weight: var(--font-weight-semibold);
}

.block-detail-subhead {
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-bold);
  color: var(--color-text-dim);
  margin: 16px 0 8px;
}

.block-detail-table {
  width: 100%;
  border-collapse: collapse;
  font-size: var(--font-size-sm);
}

.block-detail-table td {
  padding: 6px 0;
  border-bottom: 1px solid var(--color-border);
  color: var(--color-text-dim);
}

.block-detail-table td:last-child {
  text-align: right;
  font-variant-numeric: tabular-nums;
}

.block-detail-table tr:last-child td {
  border-bottom: none;
}
</style>

<script lang="ts">
// Presentational detail popup for a clicked Timeline block — split out
// of HomeTimelineSection.vue once that file crossed 1000 lines (same
// reasoning as the Progetti.vue decomposition, see BLUEPRINT.md section
// 7.3). Owns no state of its own: the parent decides what's selected
// and just hands this component the finished data to display, same
// pattern as ProjectDetailModal.vue.
import moment from 'moment';
import { formatDuration } from '~/util/projectTime';
import { displayNameForApp } from '~/util/appNames';
import { getHomeClient } from '~/util/awclient';

export default {
  name: 'TimelineBlockDetailModal',
  components: {
    'screenshot-gallery-modal': () => import('./ScreenshotGalleryModal.vue'),
  },
  props: {
    block: { type: Object, required: true },
    laneName: { type: String, required: true },
    occurrences: { type: Array, default: () => [] },
    // Per-title time breakdown within this block's own range — computed
    // by the parent (HomeTimelineSection.vue's selectedBlockTitleBreakdown(),
    // which has the raw window events this needs), empty for anything
    // that isn't a whole Generale-lane app block.
    titleBreakdown: { type: Array, default: () => [] },
    // Same source data as titleBreakdown but with real start/end moments
    // instead of just totals — handed straight through to the gallery
    // (see selectedBlockTitleSegments()), which needs actual timestamps
    // to match each screenshot to the title active when it was taken.
    titleSegments: { type: Array, default: () => [] },
  },
  data() {
    return {
      // Desktop screenshots (aw-watcher-screenshot) captured during this
      // block's own time range — not scoped to `laneName`, since a
      // screenshot is evidence of what was on screen regardless of
      // which lane the clicked block belongs to.
      screenshots: [] as { filename: string; time: string; timestamp: moment.Moment }[],
      // Explicit request: no more inline thumbnails/lightbox here — a
      // single button opens the dedicated gallery instead (see
      // ScreenshotGalleryModal.vue).
      showGallery: false,
    };
  },
  computed: {
    displayName(): string {
      return displayNameForApp(this.block.key);
    },
  },
  watch: {
    block: {
      immediate: true,
      handler() {
        this.showGallery = false;
        this.loadScreenshots();
      },
    },
  },
  methods: {
    formatDuration,
    formatRange(start: moment.Moment, end: moment.Moment): string {
      return `${start.format('HH:mm')} – ${end.format('HH:mm')}`;
    },
    async loadScreenshots() {
      let events = [];
      try {
        // Detached client — see getHomeClient() for why.
        events = await getHomeClient().getEvents('aw-watcher-screenshot', {
          start: this.block.start.toDate(),
          end: this.block.end.toDate(),
          limit: -1,
        });
      } catch {
        // Bucket doesn't exist on this server — empty, not an error.
        events = [];
      }
      this.screenshots = events
        .filter(e => e.data && e.data.filename)
        .map(e => ({
          filename: e.data.filename,
          time: moment(e.timestamp).format('HH:mm:ss'),
          timestamp: moment(e.timestamp),
        }))
        // The API returns newest-first; the gallery reads left-to-right/
        // top-to-bottom as "earliest in this block" first.
        .reverse();
    },
  },
};
</script>

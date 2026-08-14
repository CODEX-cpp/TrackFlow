<template lang="pug">
div
  div.modal-backdrop(@click="$emit('close')")
  div.edit-modal.gallery-modal.themed-scroll
    div.edit-modal-title Screenshot — {{ displayName }}
    div.gallery-empty(v-if="!flatScreenshots.length") {{ $t('home.screenshotGallery.empty') }}
    div.gallery-group(v-for="group in groups" :key="group.title")
      div.gallery-group-title {{ group.title }}
      div.gallery-group-thumbs
        img.gallery-thumb(
          v-for="s in group.shots"
          v-show="!brokenFilenames[s.filename]"
          :key="s.filename"
          :src="'/pages/app-data/screenshots/' + s.filename"
          :title="s.time"
          @click="openLightbox(s)"
          @error="$set(brokenFilenames, s.filename, true)"
        )
    div.edit-modal-actions
      div.pill-btn-ghost(@click="$emit('close')") {{ $t('home.screenshotGallery.close') }}

  div.modal-backdrop.gallery-lightbox-backdrop(v-if="lightboxIndex !== null" @click="closeLightbox")
  div.gallery-lightbox(v-if="lightboxIndex !== null")
    div.gallery-lightbox-nav.gallery-lightbox-prev(
      v-if="lightboxIndex > 0"
      @click.stop="prev"
    )
      icon(name="angle-left")
    img.gallery-lightbox-img(
      :class="{ 'gallery-lightbox-img-zoomed': imageZoom > 1, 'gallery-lightbox-img-dragging': isDragging }"
      :src="'/pages/app-data/screenshots/' + currentShot.filename"
      :style="{ transform: 'translate(' + panX + 'px, ' + panY + 'px) scale(' + imageZoom + ')' }"
      draggable="false"
      @click.stop
      @wheel="onImageWheel"
      @mousedown.stop="onImageMouseDown"
    )
    div.gallery-lightbox-nav.gallery-lightbox-next(
      v-if="lightboxIndex < flatScreenshots.length - 1"
      @click.stop="next"
    )
      icon(name="angle-right")
    div.gallery-lightbox-caption(@click.stop) {{ currentShot.title }} — {{ currentShot.time }}
</template>

<style lang="scss" scoped>
@import '../style/theme.css';
@import '../style/modals.css';

// Deliberately much wider than the standard .edit-modal (340px) — this
// holds grouped thumbnail grids, not a form.
.gallery-modal {
  width: 720px;
  max-width: calc(100vw - 48px);
  max-height: 80vh;
  overflow-y: auto;
}

.gallery-empty {
  color: var(--color-text-faint);
  font-size: var(--font-size-sm);
}

.gallery-group {
  margin-bottom: 18px;
}

.gallery-group:last-child {
  margin-bottom: 0;
}

.gallery-group-title {
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-bold);
  color: var(--color-text-dim);
  margin-bottom: 8px;
}

.gallery-group-thumbs {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.gallery-thumb {
  height: 90px;
  border-radius: var(--radius-sm);
  border: 1px solid var(--color-border);
  cursor: pointer;
  transition: filter 0.15s ease;
}

.gallery-thumb:hover {
  filter: brightness(1.15);
}

// Stacked above the gallery modal itself (z-index 50, see modals.css) —
// a click on the backdrop closes it, same "click outside to dismiss"
// convention as every other modal here.
.gallery-lightbox-backdrop {
  z-index: 60;
}

.gallery-lightbox {
  position: fixed;
  inset: 0;
  z-index: 61;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 32px;
  pointer-events: none;
}

.gallery-lightbox-img {
  max-width: 100%;
  max-height: 100%;
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-elevated);
  pointer-events: auto;
  cursor: zoom-in;
  // scale()/translate() are relative to the image's own (already
  // max-width/height clamped) box, so a zoomed-in image can extend past
  // the lightbox's padding — expected, panning (see below) is exactly
  // what lets you reach the parts that go offscreen.
  transition: transform 0.08s ease-out;
  user-select: none;
}

// Explicit follow-up request: once zoomed in, the image needs to be
// draggable to reach the parts that scaled outside the visible area —
// grab/grabbing cursors signal that, and the transition above is
// dropped while actively dragging so the image tracks the cursor
// directly instead of easing behind it.
.gallery-lightbox-img-zoomed {
  cursor: grab;
}

.gallery-lightbox-img-dragging {
  cursor: grabbing;
  transition: none;
}

.gallery-lightbox-nav {
  position: absolute;
  top: 50%;
  transform: translateY(-50%);
  width: 48px;
  height: 48px;
  border-radius: 50%;
  background-color: var(--color-bg-elev);
  border: 1px solid var(--color-border);
  color: var(--color-text);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  pointer-events: auto;
  user-select: none;
}

// Explicit fix: the ‹ › text glyphs used here before weren't visually
// centered inside the circle (font glyph metrics are asymmetric, no
// amount of flexbox centering fixes that) — real SVG icons instead,
// which render centered in their own viewBox by construction. ::v-deep
// (not >>>, which silently no-ops under lang="scss" here) reaches past
// the Icon child component's own scoping boundary to size its <svg>.
.gallery-lightbox-nav ::v-deep svg {
  width: 18px;
  height: 18px;
  fill: currentColor;
}

.gallery-lightbox-nav:hover {
  filter: brightness(1.2);
}

.gallery-lightbox-prev {
  left: 24px;
}

.gallery-lightbox-next {
  right: 24px;
}

.gallery-lightbox-caption {
  position: absolute;
  bottom: 24px;
  left: 50%;
  transform: translateX(-50%);
  background-color: var(--color-bg-elev);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  padding: 6px 14px;
  font-size: var(--font-size-sm);
  color: var(--color-text);
  pointer-events: auto;
  white-space: nowrap;
  max-width: calc(100% - 48px);
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>

<script lang="ts">
// Screenshot gallery for a Timeline block's detail popup — split out of
// TimelineBlockDetailModal.vue (explicit request): thumbnails used to
// sit inline there with no connection to *when* in the block they
// happened, so seeing "the screenshots of activity X" meant manually
// matching timestamps between the Top Window Titles breakdown and the
// screenshot strip by hand. This groups screenshots by whichever window
// title was actually active when each was taken (via titleSegments'
// real start/end times, see HomeTimelineSection.vue's
// selectedBlockTitleSegments()), and opens a larger lightbox with
// Prev/Next navigation across every screenshot in the block, not just
// the ones in the group you clicked from.
import moment from 'moment';
import 'vue-awesome/icons/angle-left';
import 'vue-awesome/icons/angle-right';

interface Screenshot {
  filename: string;
  time: string;
  timestamp: moment.Moment;
}

interface TitleSegment {
  title: string;
  start: moment.Moment;
  end: moment.Moment;
}

export default {
  name: 'ScreenshotGalleryModal',
  props: {
    screenshots: { type: Array as () => Screenshot[], default: () => [] },
    titleSegments: { type: Array as () => TitleSegment[], default: () => [] },
    displayName: { type: String, required: true },
  },
  data() {
    return {
      // Index into flatScreenshots, not into any one group — Prev/Next
      // moves across the whole block chronologically regardless of
      // which title's thumbnail row you opened.
      lightboxIndex: null as number | null,
      // Wheel-zoom on the open image (explicit request) — a plain CSS
      // scale, reset back to 1 on every navigation so zooming into one
      // screenshot doesn't carry over to the next.
      imageZoom: 1,
      // Pan offset while zoomed in (explicit follow-up request — once
      // zoomed, the interesting detail is often outside the visible
      // area, so it needs to be draggable). Raw pixels, reset alongside
      // imageZoom on every navigation.
      panX: 0,
      panY: 0,
      isDragging: false,
      dragStartMouse: { x: 0, y: 0 },
      dragStartPan: { x: 0, y: 0 },
      // Filename -> true per ogni miniatura il cui file non esiste più
      // (es. eliminato dalla pulizia automatica per anzianità) — nascosta
      // invece di mostrare l'icona di immagine rotta del browser.
      brokenFilenames: {} as Record<string, boolean>,
    };
  },
  computed: {
    // Every screenshot resolved to the title active at its own moment —
    // 'Sconosciuto' when there's no matching segment at all (e.g. the
    // block isn't from the Generale lane, so titleSegments is empty).
    flatScreenshots(): (Screenshot & { title: string })[] {
      return this.screenshots.map(s => ({ ...s, title: this.titleForTimestamp(s.timestamp) }));
    },
    // Grouped in order of each title's first screenshot — not
    // alphabetical or by total count — so scanning top to bottom roughly
    // follows the order things happened in.
    groups(): { title: string; shots: (Screenshot & { title: string })[] }[] {
      const order: string[] = [];
      const byTitle = new Map<string, (Screenshot & { title: string })[]>();
      for (const s of this.flatScreenshots) {
        if (!byTitle.has(s.title)) {
          byTitle.set(s.title, []);
          order.push(s.title);
        }
        byTitle.get(s.title)!.push(s);
      }
      return order.map(title => ({ title, shots: byTitle.get(title)! }));
    },
    currentShot(): (Screenshot & { title: string }) | null {
      return this.lightboxIndex !== null ? this.flatScreenshots[this.lightboxIndex] : null;
    },
  },
  mounted() {
    window.addEventListener('keydown', this.onKeydown);
  },
  beforeDestroy() {
    window.removeEventListener('keydown', this.onKeydown);
    // Defensive — in case the component is torn down mid-drag.
    window.removeEventListener('mousemove', this.onDragMove);
    window.removeEventListener('mouseup', this.onDragEnd);
  },
  methods: {
    titleForTimestamp(ts: moment.Moment): string {
      const seg = this.titleSegments.find(
        (s: TitleSegment) => !ts.isBefore(s.start) && ts.isBefore(s.end)
      );
      return seg ? seg.title : (this.$t('home.screenshotGallery.unknown') as string);
    },
    resetZoom() {
      this.imageZoom = 1;
      this.panX = 0;
      this.panY = 0;
    },
    openLightbox(shot: Screenshot & { title: string }) {
      this.lightboxIndex = this.flatScreenshots.findIndex(s => s.filename === shot.filename);
      this.resetZoom();
    },
    closeLightbox() {
      this.lightboxIndex = null;
      this.resetZoom();
    },
    prev() {
      if (this.lightboxIndex !== null && this.lightboxIndex > 0) {
        this.lightboxIndex--;
        this.resetZoom();
      }
    },
    next() {
      if (this.lightboxIndex !== null && this.lightboxIndex < this.flatScreenshots.length - 1) {
        this.lightboxIndex++;
        this.resetZoom();
      }
    },
    // Explicit request: scrolling the wheel while hovering the
    // full-size image zooms it — scoped to the <img> itself (not the
    // whole lightbox) via @wheel there, and preventDefault()+stop()
    // stops it from bubbling up to anything that might otherwise treat
    // the scroll as page/Timeline navigation.
    onImageWheel(evt: WheelEvent) {
      evt.preventDefault();
      evt.stopPropagation();
      const ZOOM_STEP = 0.15;
      const MIN_ZOOM = 1;
      const MAX_ZOOM = 5;
      const direction = evt.deltaY > 0 ? -1 : 1;
      this.imageZoom = Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, this.imageZoom + direction * ZOOM_STEP));
      // Back at 1x there's nothing to pan into — drop any leftover
      // offset instead of leaving the image visibly shifted once it
      // snaps back to its normal (unscaled) size.
      if (this.imageZoom === MIN_ZOOM) {
        this.panX = 0;
        this.panY = 0;
      }
    },
    // Explicit follow-up request: once zoomed in, drag the image around
    // to reach the parts that scaled outside the visible area. Only
    // starts a drag when actually zoomed in — at 1x the image can't
    // overflow its box, so there'd be nowhere to pan to. mousemove/up
    // are bound on `window` rather than the image itself so the drag
    // keeps tracking even if the cursor slips past the image's edges
    // mid-drag (very likely once zoomed in, since the image can extend
    // beyond the lightbox's own bounds).
    onImageMouseDown(evt: MouseEvent) {
      if (this.imageZoom <= 1) return;
      this.isDragging = true;
      this.dragStartMouse = { x: evt.clientX, y: evt.clientY };
      this.dragStartPan = { x: this.panX, y: this.panY };
      window.addEventListener('mousemove', this.onDragMove);
      window.addEventListener('mouseup', this.onDragEnd);
    },
    onDragMove(evt: MouseEvent) {
      this.panX = this.dragStartPan.x + (evt.clientX - this.dragStartMouse.x);
      this.panY = this.dragStartPan.y + (evt.clientY - this.dragStartMouse.y);
    },
    onDragEnd() {
      this.isDragging = false;
      window.removeEventListener('mousemove', this.onDragMove);
      window.removeEventListener('mouseup', this.onDragEnd);
    },
    // Explicit request was for on-screen Prev/Next buttons — arrow-key
    // support is the same navigation via a second, expected input path
    // for a lightbox, not a separate feature.
    onKeydown(evt: KeyboardEvent) {
      if (this.lightboxIndex === null) return;
      if (evt.key === 'ArrowLeft') this.prev();
      else if (evt.key === 'ArrowRight') this.next();
      else if (evt.key === 'Escape') this.closeLightbox();
    },
  },
};
</script>

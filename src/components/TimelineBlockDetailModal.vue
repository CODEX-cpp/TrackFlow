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

    div.block-detail-subhead-row(v-if="occurrencesByTitle.length || occurrences.length > 1")
      span.block-detail-subhead {{ $t('home.timelineBlockDetail.otherOccurrences') }}
      // Solo nel caso non raggruppato — quando i titoli sono raggruppati,
      // il pulsante vive accanto a ciascun titolo invece che qui.
      span.block-detail-photos-btn(
        v-if="!occurrencesByTitle.length && screenshotsFor(occurrences).length"
        @click="openGallery(displayName, occurrences)"
      ) {{ $t('home.timelineBlockDetail.viewPhotos', { count: screenshotsFor(occurrences).length }) }}
    template(v-if="occurrencesByTitle.length")
      div.block-detail-title-group(v-for="group in occurrencesByTitle" :key="group.title")
        div.block-detail-title-group-head
          span.block-detail-title-group-name {{ group.title }}
          // Filtrato SOLO agli orari di questo titolo — richiesta
          // esplicita: non tutte le foto del blocco, solo quelle cadute
          // dentro una delle fasce orarie elencate qui sotto.
          span.block-detail-photos-btn(
            v-if="screenshotsFor(group.occurrences).length"
            @click="openGallery(group.title, group.occurrences)"
          ) {{ $t('home.timelineBlockDetail.viewPhotos', { count: screenshotsFor(group.occurrences).length }) }}
        table.block-detail-table.block-detail-title-group-table
          tbody
            tr(v-for="(occ, i) in group.occurrences" :key="i")
              td {{ occ.start.format('HH:mm') }} – {{ occ.end.format('HH:mm') }}
              td {{ formatDuration(occ.end.diff(occ.start, 'seconds')) }}
    table.block-detail-table(v-else-if="occurrences.length > 1")
      tbody
        tr(v-for="(occ, i) in occurrences" :key="i")
          td {{ occ.start.format('HH:mm') }} – {{ occ.end.format('HH:mm') }}
          td {{ formatDuration(occ.end.diff(occ.start, 'seconds')) }}

    div.edit-modal-actions
      div.pill-btn-ghost(@click="$emit('close')") {{ $t('home.timelineBlockDetail.close') }}

  screenshot-gallery-modal(
    v-if="showGallery"
    :screenshots="galleryScreenshots"
    :title-segments="galleryTitleSegments"
    :display-name="galleryTitle"
    @close="showGallery = false"
  )
</template>

<style lang="scss" scoped>
@import '../style/theme.css';
@import '../style/modals.css';

.block-detail-modal {
  width: 480px;
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

.block-detail-subhead-row {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 10px;
  margin: 16px 0 8px;
}

.block-detail-subhead {
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-bold);
  color: var(--color-text-dim);
}

// Vero pulsante a pillola a destra del titolo (non più un link a
// testo semplice, giudicato brutto) — apre la galleria filtrata solo
// agli orari di QUEL titolo (vedi screenshotsFor()/openGallery()
// sotto), non a tutto il blocco.
.block-detail-photos-btn {
  display: inline-flex;
  align-items: center;
  flex-shrink: 0;
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-semibold);
  color: var(--color-text-dim);
  background-color: var(--color-surface2);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-pill);
  padding: 3px 10px;
  cursor: pointer;
  white-space: nowrap;
}

.block-detail-photos-btn:hover {
  color: var(--color-text);
  border-color: var(--color-accent1);
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

.block-detail-title-group + .block-detail-title-group {
  margin-top: 12px;
}

.block-detail-title-group-head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 10px;
  margin-bottom: 4px;
}

.block-detail-title-group-name {
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-semibold);
  color: var(--color-text);
}

// Righe orario leggermente rientrate, con una barra verticale a
// sinistra allineata col titolo sopra — richiesta estetica esplicita.
// Il padding va sulla prima cella, non sulla <table>: il padding di un
// elemento <table> non crea spazio visibile prima del contenuto delle
// celle (a differenza di un div), motivo per cui riga e barra
// risultavano incollate nel primo tentativo.
.block-detail-title-group-table {
  margin-left: 0;
  border-left: 2px solid var(--color-border);
}

.block-detail-title-group-table td:first-child {
  padding-left: 10px;
}

// La riga di separazione tra un orario e l'altro (border-bottom, da
// .block-detail-table td) partiva dal bordo sinistro della cella —
// stesso punto della barra verticale qui sopra, formando una "T" ad
// ogni riga. Sostituita con una linea disegnata via gradiente, che
// parte più a destra (stesso rientro del testo) invece di toccare la
// barra.
.block-detail-title-group-table td {
  border-bottom: none;
}

.block-detail-title-group-table tr {
  background-image: linear-gradient(to right, transparent 10px, var(--color-border) 10px);
  background-position: bottom;
  background-repeat: no-repeat;
  background-size: 100% 1px;
}

.block-detail-title-group-table tr:last-child {
  background-image: none;
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

interface Screenshot {
  filename: string;
  time: string;
  timestamp: moment.Moment;
}

export default {
  name: 'TimelineBlockDetailModal',
  components: {
    'screenshot-gallery-modal': () => import('./ScreenshotGalleryModal.vue'),
  },
  props: {
    block: { type: Object, required: true },
    laneName: { type: String, required: true },
    occurrences: { type: Array, default: () => [] },
    // Stessa lista di occurrences, ma raggruppata per titolo — quando
    // presente sostituisce del tutto la tabella piatta sopra (solo per
    // le corsie i cui eventi grezzi hanno un campo title distinto, vedi
    // selectedOccurrencesByTitle() in HomeTimelineSection.vue).
    occurrencesByTitle: { type: Array, default: () => [] },
  },
  data() {
    return {
      // Screenshot di TUTTA la giornata coperta da occurrences/
      // occurrencesByTitle (non solo l'intervallo del blocco cliccato,
      // vedi screenshotRange) — caricati una sola volta, poi filtrati
      // per titolo al volo in screenshotsFor() invece di rifare una
      // richiesta di rete ad ogni click su un pulsante diverso.
      screenshots: [] as Screenshot[],
      showGallery: false,
      galleryTitle: '',
      galleryScreenshots: [] as Screenshot[],
      galleryTitleSegments: [] as { title: string; start: moment.Moment; end: moment.Moment }[],
    };
  },
  computed: {
    displayName(): string {
      return displayNameForApp(this.block.key);
    },
    // L'intervallo da interrogare per gli screenshot: non solo
    // block.start/end (una singola occorrenza), ma l'estremo min/max fra
    // TUTTE le occorrenze del giorno — necessario perché ogni pulsante
    // "N foto" può aprire la galleria per un titolo comparso ore prima o
    // dopo l'istanza di blocco effettivamente cliccata.
    screenshotRange(): { start: moment.Moment; end: moment.Moment } {
      const starts = [this.block.start, ...this.occurrences.map((o: any) => o.start)];
      const ends = [this.block.end, ...this.occurrences.map((o: any) => o.end)];
      return { start: moment.min(starts), end: moment.max(ends) };
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
          start: this.screenshotRange.start.toDate(),
          end: this.screenshotRange.end.toDate(),
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
    // Solo gli screenshot caduti dentro una qualunque delle fasce
    // orarie passate — usato sia per decidere se mostrare il pulsante
    // (nessuna foto = nessun pulsante) sia per popolare la galleria.
    screenshotsFor(occs: { start: moment.Moment; end: moment.Moment }[]): Screenshot[] {
      return this.screenshots.filter((s: Screenshot) =>
        occs.some(o => !s.timestamp.isBefore(o.start) && s.timestamp.isBefore(o.end))
      );
    },
    openGallery(title: string, occs: { start: moment.Moment; end: moment.Moment }[]) {
      this.galleryTitle = title;
      this.galleryScreenshots = this.screenshotsFor(occs);
      this.galleryTitleSegments = occs.map(o => ({ title, start: o.start, end: o.end }));
      this.showGallery = true;
    },
  },
};
</script>

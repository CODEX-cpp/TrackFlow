<template lang="pug">
div
  div.modal-backdrop(@click="$emit('close')")
  div.edit-modal.block-detail-modal.themed-scroll(:class="{ 'block-detail-modal-wide': occurrencesTimeline.length }")
    div.edit-modal-title-row
      div.edit-modal-title {{ displayName }}
      // Solo per le corsie "generiche" (non VPN/Excel/VoiSpeed/VSCode/
      // Claude, vedi mostraBottoneAi) e solo se una chiave API è
      // configurata — altrimenti il pulsante non compare affatto invece
      // di aprire una chat che fallirebbe subito. Apre la chat AI con
      // questa attività allegata al prossimo messaggio, come rispondere
      // a un messaggio su WhatsApp — vedi apriConversazioneAi().
      button.block-detail-ai-btn(
        v-if="mostraBottoneAi"
        type="button"
        @click="apriConversazioneAi"
        :title="$t('home.timelineBlockDetail.askAi')"
      )
        icon(name="comments")
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

    template(v-if="occurrencesTimeline.length")
      // Elenco cronologico dentro il SOLO blocco cliccato (non l'intera
      // giornata) — attivo quando non c'è nessuna evidenziazione app
      // già in corso, vedi appInteroSelezionato()/
      // selectedOccurrencesTimeline() in HomeTimelineSection.vue per il
      // perché. Niente pulsante foto qui per ora, richiesta esplicita
      // dell'utente ("vediamo quanti dati uscirebbero prima così").
      div.block-detail-subhead-row
        span.block-detail-subhead {{ $t('home.timelineBlockDetail.duringThisBlock') }}
      // Sotto al sottotitolo, allineato a sinistra come lui — non a
      // destra come il pulsante delle altre viste — richiesta estetica
      // esplicita per questa vista in particolare.
      div.block-detail-photos-row(v-if="mostraFoto && timelineScreenshots.length")
        span.block-detail-photos-btn(@click="openGalleryTimeline")
          | {{ $t('home.timelineBlockDetail.viewPhotos', { count: timelineScreenshots.length }) }}
      table.block-detail-table.block-detail-timeline-table
        tbody
          tr(v-for="(occ, i) in occurrencesTimeline" :key="i")
            td.block-detail-timeline-time {{ occ.start.format('HH:mm') }} – {{ occ.end.format('HH:mm') }}
            td.block-detail-timeline-title {{ occ.title }}
    template(v-else)
      div.block-detail-subhead-row(v-if="occurrencesByTitle.length || occurrences.length > 1")
        span.block-detail-subhead {{ $t('home.timelineBlockDetail.otherOccurrences') }}
        // Solo nel caso non raggruppato — quando i titoli sono raggruppati,
        // il pulsante vive accanto a ciascun titolo invece che qui.
        span.block-detail-photos-btn(
          v-if="mostraFoto && !occurrencesByTitle.length && screenshotsFor(occurrences).length"
          @click="openGallery(displayName, occurrences)"
        ) {{ $t('home.timelineBlockDetail.viewPhotos', { count: screenshotsFor(occurrences).length }) }}
      template(v-if="occurrencesByTitle.length")
        div.block-detail-title-group(v-for="group in occurrencesByTitle" :key="group.title")
          div.block-detail-title-group-head
            span.block-detail-title-group-name {{ group.title }}
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

// Solo per l'elenco cronologico del singolo blocco (occurrencesTimeline):
// i titoli di finestra reali sono spesso lunghi e con soli 480px
// venivano troncati quasi subito — più larga per lasciare respirare il
// testo, non serve per le altre viste (raggruppata per titolo, VPN,
// Excel...) che restano compatte.
.block-detail-modal-wide {
  width: 720px;
}

.edit-modal-title-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  // .edit-modal-title (globale, modals.css) porta il suo margin-bottom:
  // spostato qui sulla riga intera, altrimenti solo il titolo lo
  // manterrebbe come flex item, disallineando verticalmente il pulsante
  // AI accanto.
  margin-bottom: 18px;

  .edit-modal-title {
    margin-bottom: 0;
  }
}

// Stesso stile pieno-giallo del FAB della chat AI in basso a destra
// (AiChatWidget.vue's .ai-chat-fab) — richiesta esplicita, per far
// riconoscere subito che porta allo stesso posto. Ingrandito da 30 a
// 38px, sempre su richiesta esplicita ("un po più grande e visibile").
.block-detail-ai-btn {
  flex-shrink: 0;
  width: 38px;
  height: 38px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: var(--radius-pill);
  background-color: var(--color-accent1);
  color: #241a12;
  cursor: pointer;
  font-size: 16px;
  box-shadow: var(--shadow-elevated);

  &:hover {
    filter: brightness(1.08);
  }
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
.block-detail-photos-row {
  margin: -2px 0 8px;
}

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

// Colonna del titolo nell'elenco cronologico (occurrencesTimeline) — a
// differenza delle altre due colonne (orario, durata) può essere
// lunghissima (titoli di finestra reali), quindi tronca con ellissi
// invece di allargare la tabella o andare a capo.
// Colonna oraria dell'elenco cronologico: con table-layout: fixed e
// solo la colonna del titolo con una width esplicita (sotto), questa
// colonna veniva schiacciata a una larghezza quasi nulla e il testo
// "12:38 – 12:39" andava a capo su più righe — con la colonna titolo
// che, per via del vertical-align di default, finiva visivamente
// incastrata TRA le righe spezzate dell'orario invece che accanto
// (bug segnalato dall'utente). Width fissa + nowrap + vertical-align
// esplicito risolvono entrambe le cose.
.block-detail-timeline-time {
  width: 90px;
  white-space: nowrap;
  vertical-align: top;
}

.block-detail-timeline-title {
  color: var(--color-text);
  max-width: 0; // forza table-layout a rispettare la colonna precedente
  width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  padding: 6px 10px !important;
  text-align: left !important;
  vertical-align: top;
}

.block-detail-timeline-table {
  table-layout: fixed;
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
import 'vue-awesome/icons/comments';
import moment from 'moment';
import { invoke } from '@tauri-apps/api/core';
import { formatDuration } from '~/util/projectTime';
import { displayNameForApp } from '~/util/appNames';
import { getHomeClient } from '~/util/awclient';
import { useAiChatContextStore } from '~/stores/aiChatContext';

// Corsie escluse dal pulsante "chiedi all'AI" — richiesta esplicita
// dell'utente: hanno già un legame dati troppo specifico/ambiguo (VPN,
// Excel, VoiSpeed sono sessioni/telefonate non "attività" nel senso che
// l'AI interroga di solito; VSCode e Claude hanno bucket dedicati che
// l'AI può già interrogare direttamente con le sue query). Il pulsante
// resta per le corsie generiche (Generale, Browser, Sfondo).
const CORSIE_ESCLUSE_AI = ['vpn', 'excel', 'voispeed', 'vscode', 'claude'];

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
    // Chiave stabile della corsia (es. "vpn"), non il nome tradotto —
    // serve a mostraFoto() sotto per un confronto che non dipenda dalla
    // lingua attiva.
    laneKey: { type: String, default: '' },
    occurrences: { type: Array, default: () => [] },
    // Stessa lista di occurrences, ma raggruppata per titolo — quando
    // presente sostituisce del tutto la tabella piatta sopra (solo per
    // le corsie i cui eventi grezzi hanno un campo title distinto, vedi
    // selectedOccurrencesByTitle() in HomeTimelineSection.vue).
    occurrencesByTitle: { type: Array, default: () => [] },
    // Elenco cronologico (non raggruppato) dentro il SOLO blocco
    // cliccato — quando presente sostituisce del tutto sia occurrences
    // sia occurrencesByTitle, vedi selectedOccurrencesTimeline() in
    // HomeTimelineSection.vue.
    occurrencesTimeline: { type: Array, default: () => [] },
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
      aiChatContextStore: useAiChatContextStore(),
      // null finché non controllata — vedi lo stesso pattern in
      // AiChatWidget.vue per il perché non parte da false.
      apiConfigurata: null as boolean | null,
    };
  },
  created() {
    this.verificaConfigurazioneAi();
  },
  computed: {
    displayName(): string {
      return displayNameForApp(this.block.key);
    },
    // Bug reale segnalato dall'utente: gli screenshot sono scatti
    // periodici del desktop, del tutto slegati da QUALE client VPN era
    // connesso o QUALE file Excel era aperto in quel momento — mostrare
    // "N foto" su un blocco VPN/Excel suggeriva un legame che non
    // esiste (le foto sono semplicemente quelle scattate per caso
    // durante quella finestra oraria, di qualunque app fosse a fuoco —
    // per Excel in particolare, il file può restare "aperto" per ore
    // in background senza mai essere a fuoco). Per le altre corsie
    // (app, browser, editor...) il legame ha invece senso, quindi il
    // pulsante resta.
    mostraFoto(): boolean {
      return this.laneKey !== 'vpn' && this.laneKey !== 'excel';
    },
    mostraBottoneAi(): boolean {
      return this.apiConfigurata === true && !CORSIE_ESCLUSE_AI.includes(this.laneKey);
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
    // Solo per la vista cronologica del singolo blocco: intervallo
    // continuo dall'inizio del PRIMO evento elencato alla fine
    // dell'ULTIMO — richiesta esplicita dell'utente, a differenza di
    // screenshotsFor() qui sotto (usato dalle altre viste) questo non
    // esclude gli screenshot caduti nei micro-buchi tra un'occorrenza e
    // l'altra.
    timelineScreenshots(): Screenshot[] {
      if (!this.occurrencesTimeline.length) return [];
      const start = (this.occurrencesTimeline[0] as any).start;
      const end = (this.occurrencesTimeline[this.occurrencesTimeline.length - 1] as any).end;
      return this.screenshots.filter(
        (s: Screenshot) => !s.timestamp.isBefore(start) && s.timestamp.isBefore(end)
      );
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
    // Usa le occorrenze cronologiche stesse come titleSegments, così la
    // galleria raggruppa ogni screenshot sotto il titolo davvero attivo
    // in quel momento (non un unico titolo generico) — quelli caduti
    // nei micro-buchi tra un'occorrenza e l'altra finiscono nel gruppo
    // "Sconosciuto" già gestito da ScreenshotGalleryModal.
    openGalleryTimeline() {
      this.galleryTitle = this.displayName;
      this.galleryScreenshots = this.timelineScreenshots;
      this.galleryTitleSegments = this.occurrencesTimeline as { title: string; start: moment.Moment; end: moment.Moment }[];
      this.showGallery = true;
    },
    async verificaConfigurazioneAi() {
      try {
        const config = await invoke<{ api_key: string } | null>('ai_agent_get_config');
        this.apiConfigurata = !!config && !!config.api_key.trim();
      } catch {
        // Fuori da Tauri (npx vite puro) — vedi lo stesso catch in
        // AiChatWidget.vue.
        this.apiConfigurata = true;
      }
    },
    // Il testo con i dati veri (elenco attività/orari) non finisce mai
    // nella bolla mostrata in chat — viene anteposto al messaggio
    // dell'utente solo al momento dell'invio, vedi invia() in
    // AiChatWidget.vue. Qui si prepara solo quel testo più l'etichetta
    // "a cosa sto rispondendo" mostrata sopra il campo di scrittura.
    apriConversazioneAi() {
      const intervallo = this.formatRange(this.block.start, this.block.end);
      const label = `${this.displayName} ${intervallo}`;
      let extra: string;
      if (this.occurrencesTimeline.length) {
        const righe = (this.occurrencesTimeline as { start: moment.Moment; end: moment.Moment; title: string }[])
          .map(o => `- ${o.start.format('HH:mm')}–${o.end.format('HH:mm')}: ${o.title}`)
          .join('\n');
        extra = `Attività raccolte per "${this.displayName}" (corsia: ${this.laneName}) tra le ${intervallo}:\n${righe}`;
      } else {
        extra = `Blocco "${this.displayName}" (corsia: ${this.laneName}) attivo dalle ${intervallo}.`;
      }
      this.aiChatContextStore.imposta({ label, extra });
      this.$emit('close');
    },
  },
};
</script>

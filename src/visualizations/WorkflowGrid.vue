<template lang="pug">
div.workflow-grid-root
  div.workflow-grid-empty(v-if="!loading && rows.length === 0") {{ $t('visualizations.workflowGrid.empty') }}
  template(v-else)
    div.workflow-grid-inner
      div.workflow-grid-row(v-for="row in rows" :key="row.key")
        div.workflow-grid-row-label(:style="{ color: row.color }") {{ row.name }}
        div.workflow-grid-cells
          div.workflow-grid-cell(
            v-for="(coverage, i) in row.cells"
            :key="i"
            :class="{ 'workflow-grid-cell-active': coverage > 0 }"
            :style="coverage > 0 ? { backgroundColor: row.color, opacity: cellOpacity(coverage) } : null"
            @mouseenter="onCellHover(i)"
            @mouseleave="onCellLeave"
          )
      div.workflow-grid-row.workflow-grid-row-distractions
        div.workflow-grid-row-label {{ $t('visualizations.workflowGrid.distractions') }}
        div.workflow-grid-distraction-track
          div.workflow-grid-distraction-segment(
            v-for="(seg, i) in distractionSegments"
            :key="i"
            :style="{ left: seg.left + '%', width: seg.width + '%' }"
          )
      div.workflow-grid-ruler
        span.workflow-grid-tick(v-for="t in hourTicks" :key="t.hour" :style="{ left: t.left + '%' }") {{ t.label }}
</template>

<style lang="scss" scoped>
@import '../style/theme.css';

// Altezza fissa delle celle — la LARGHEZZA invece è libera (flex, vedi
// .workflow-grid-cell sotto): richiesta esplicita dell'utente, niente
// barra di scorrimento orizzontale, i quadratini si rimpiccioliscono
// tutti insieme per stare nella larghezza reale della card invece di
// traboccare.
$altezza-cella: 14px;
$gap-celle: 2px;
// Larghezza della colonna etichette (nome categoria/"Distrazioni") + il
// gap tra etichetta e celle — deve combaciare con .workflow-grid-row
// gap e .workflow-grid-row-label width sotto (il righello vive fuori
// dal flex label+celle, quindi ha bisogno dello stesso offset a mano
// per allinearsi alla colonna delle celle vere e proprie).
$larghezza-etichetta: 90px;
$gap-etichetta-celle: 8px;

.workflow-grid-empty {
  font-size: var(--font-size-sm);
  color: var(--color-text-faint);
}

.workflow-grid-inner {
  position: relative;
}

.workflow-grid-row {
  display: flex;
  align-items: center;
  gap: $gap-etichetta-celle;
  padding: 2px 0;
}

.workflow-grid-row-label {
  width: $larghezza-etichetta;
  flex-shrink: 0;
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-semibold);
  color: var(--color-text-dim);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.workflow-grid-cells {
  display: flex;
  gap: $gap-celle;
  // flex:1 (non width fissa) — riempie tutto lo spazio rimasto dopo la
  // colonna etichetta, qualunque esso sia; min-width:0 è necessario
  // perché un figlio flex non si restringe mai sotto la sua dimensione
  // di contenuto per default, e qui il "contenuto" sono N quadratini a
  // larghezza fissa che altrimenti la farebbero traboccare comunque.
  flex: 1;
  min-width: 0;
}

.workflow-grid-cell {
  // flex:1 in parti uguali su TUTTI i quadratini della riga — è questo
  // (non un valore fisso) a farli rimpicciolire insieme quando sono
  // troppi per la larghezza disponibile, invece di traboccare con uno
  // scroll orizzontale (comportamento precedente, esplicitamente
  // respinto dall'utente). min-width piccolo ma non zero: restano
  // sempre visibili anche in una giornata con moltissimi slot, invece
  // di sparire del tutto.
  flex: 1 1 0;
  min-width: 2px;
  height: $altezza-cella;
  border-radius: 2px;
  background-color: var(--color-surface2);
}

// Riga "Distrazioni" separata visivamente dalle categorie sopra (bordo
// + margine) — è un tipo di dato diverso (tempo AFK, non una categoria
// dell'utente), non deve leggersi come una categoria in più tra le
// altre.
.workflow-grid-row-distractions {
  margin-top: 6px;
  padding-top: 6px;
  border-top: 1px solid var(--color-border);
}

.workflow-grid-row-distractions .workflow-grid-row-label {
  color: var(--color-danger);
}

// Riga "Distrazioni" come una linea sottile continua, non quadratini —
// richiesta esplicita dell'utente, stesso identico stile della barra
// AFK sopra la Timeline (.afk-status-bar/.afk-status-segment in
// HomeTimelineSection.vue): qui in percentuale invece che in px perché
// le celle sopra sono ormai anch'esse a larghezza flessibile (vedi
// .workflow-grid-cell), non c'è più una larghezza fissa nota in
// anticipo da cui derivare un px assoluto.
.workflow-grid-distraction-track {
  position: relative;
  flex: 1;
  min-width: 0;
  height: 3px;
  border-radius: var(--radius-pill);
  background-color: var(--color-surface2);
}

.workflow-grid-distraction-segment {
  position: absolute;
  top: 0;
  height: 100%;
  border-radius: var(--radius-pill);
  background-color: var(--color-danger);
}

.workflow-grid-ruler {
  position: relative;
  height: 16px;
  margin-top: 4px;
  margin-left: $larghezza-etichetta + $gap-etichetta-celle;
}

.workflow-grid-tick {
  position: absolute;
  font-size: 10.5px;
  color: var(--color-text-faint);
}
</style>

<script lang="ts">
// Modulo Home "Flusso di lavoro" — una riga per categoria (quelle
// dell'utente, Impostazioni → Categorizzazione), colonne = slot da 15
// minuti nell'arco della giornata corrente, cella colorata se c'era
// attività di quella categoria in quello slot. Ultima riga
// "Distrazioni" = periodi AFK (tempo lontano dal PC), stessa idea della
// barra verde/rossa sopra la Timeline ma discretizzata sugli stessi
// slot invece che continua — scelta dell'utente via AskUserQuestion:
// categorie dell'utente (non le corsie della Timeline), distrazioni =
// tempo AFK (non un elenco di app da marcare a mano), slot da 15
// minuti.
//
// Componente autonomo (nessuna prop, come ActivityHeatmap.vue) — a
// differenza dei moduli "semplici" (CategoryBar, TopSummary) che
// ricevono dati già aggregati dal genitore, questo ha bisogno degli
// eventi grezzi CON i loro orari per costruire la griglia, quindi si
// scarica i bucket da sé per il giorno corrente (homeActivityRangeMixin
// — stesso identico "quale giorno, quale host, scarica questo bucket in
// sicurezza" già usato da HomeTimelineSection.vue).
import moment from 'moment';
import homeActivityRangeMixin from '~/mixins/homeActivityRangeMixin';
import { useAppCategoriesStore } from '~/stores/appCategories';
import { useSettingsStore } from '~/stores/settings';
import { eventListSignature, clipEventsToIntervals } from '~/util/timelineBlocks';
// --- INIZIO: hover quadratino → due linee in Timeline (funzione sperimentale, facile da rimuovere — vedi stores/timelineHighlight.ts) ---
import { useTimelineHighlightStore } from '~/stores/timelineHighlight';
// --- FINE ---

const SLOT_MINUTES = 15;
// Stessa soglia della barra AFK della Timeline (vedi
// GAP_BRIDGE_THRESHOLD_SECONDS in HomeTimelineSection.vue) — salda i
// piccoli buchi tra eventi afk consecutivi ravvicinati (jitter del
// polling del watcher) così la riga "Distrazioni" legge come blocchi
// continui invece che tratteggiata per un artefatto di campionamento.
const GAP_BRIDGE_SECONDS = 5 * 60;

interface Row {
  key: string;
  name: string;
  color: string;
  // Frazione (0..1) di ciascuno slot davvero coperta da attività — non
  // più un semplice booleano, vedi il commento sul computed rows().
  cells: number[];
  totalSeconds: number;
}

// Opacità minima per una cella "attiva" (coverage > 0) — anche un solo
// minuto reale su 15 resta visibile come una tinta leggera invece di
// sparire quasi del tutto; 1.0 (coverage intero) resta colore pieno.
// Intervallo scelto a occhio, non da una formula: 0.35 era il punto in
// cui una cella al minimo ancora si distingue dallo sfondo neutro delle
// celle vuote su entrambi i temi.
const MIN_CELL_OPACITY = 0.35;

export default {
  name: 'aw-workflow-grid',
  mixins: [homeActivityRangeMixin],
  data() {
    return {
      loading: true,
      rawWindowEvents: [] as any[],
      rawAfkEvents: [] as any[],
      lastSignature: null as string | null,
      categorieStore: useAppCategoriesStore(),
      settingsStore: useSettingsStore(),
      // --- INIZIO: hover quadratino → due linee in Timeline ---
      highlightStore: useTimelineHighlightStore(),
      // Ritarda la vera cancellazione dell'intervallo di qualche decina
      // di ms invece di farla scattare subito su mouseleave — passando
      // da un quadratino al successivo il mouse attraversa per un
      // istante il piccolo spazio vuoto tra le due celle, e senza
      // questo ritardo quello basta a smontare e rimontare le linee
      // nella Timeline (mouseleave→mouseenter) invece di limitarsi a
      // spostarle: uno smontaggio non può animare, da qui lo sfarfallio
      // segnalato dall'utente nonostante la transizione CSS. Se il
      // mouse esce DAVVERO dalla griglia (nessun nuovo hover entro il
      // ritardo), onCellHover() non arriva mai a cancellare questo
      // timer e la cancellazione avviene comunque.
      hoverClearTimer: null as ReturnType<typeof setTimeout> | null,
      // --- FINE ---
    };
  },
  computed: {
    // Confini reali di attività della giornata (non l'intera finestra
    // offset-aware dayStart/dayEnd, quasi sempre molto più ampia e
    // piena di ore vuote) — stessa idea della "shape of the day" già
    // usata per la barra AFK della Timeline. Arrotondati allo slot da
    // 15 minuti più vicino così ogni colonna della griglia è uno slot
    // intero, mai una frazione.
    bounds(): { start: moment.Moment; end: moment.Moment } | null {
      let min: moment.Moment | null = null;
      let max: moment.Moment | null = null;
      const consider = (t: moment.Moment) => {
        if (!min || t.isBefore(min)) min = t;
        if (!max || t.isAfter(max)) max = t;
      };
      for (const e of this.rawWindowEvents as any[]) {
        consider(moment(e.timestamp));
        consider(moment(e.timestamp).add(e.duration || 0, 'seconds'));
      }
      for (const e of this.rawAfkEvents as any[]) {
        consider(moment(e.timestamp));
        consider(moment(e.timestamp).add(e.duration || 0, 'seconds'));
      }
      if (!min || !max) return null;
      const start = (min as moment.Moment).clone();
      start.seconds(0).milliseconds(0);
      start.minutes(Math.floor(start.minutes() / SLOT_MINUTES) * SLOT_MINUTES);
      const end = (max as moment.Moment).clone();
      end.seconds(0).milliseconds(0);
      end.minutes(Math.ceil(end.minutes() / SLOT_MINUTES) * SLOT_MINUTES);
      if (!end.isAfter(start)) end.add(SLOT_MINUTES, 'minutes');
      return { start, end };
    },
    slotCount(): number {
      if (!this.bounds) return 0;
      return Math.round(this.bounds.end.diff(this.bounds.start, 'minutes') / SLOT_MINUTES);
    },
    // `left` è una PERCENTUALE (0-100), non più un px assoluto — i
    // quadratini non hanno più una larghezza fissa nota in anticipo
    // (vedi .workflow-grid-cell nel CSS, ora flex:1 così si
    // rimpiccioliscono per stare nella card invece di traboccare),
    // quindi anche il righello deve posizionarsi in proporzione alla
    // larghezza reale invece che contare i px di ogni slot.
    hourTicks(): { hour: number; left: number; label: string }[] {
      if (!this.bounds || this.slotCount === 0) return [];
      const ticks: { hour: number; left: number; label: string }[] = [];
      const cursor = this.bounds.start.clone().startOf('hour');
      if (cursor.isBefore(this.bounds.start)) cursor.add(1, 'hour');
      while (cursor.isBefore(this.bounds.end)) {
        const slotIndex = cursor.diff(this.bounds.start, 'minutes') / SLOT_MINUTES;
        ticks.push({ hour: cursor.hour(), left: (slotIndex / this.slotCount) * 100, label: cursor.format('HH:mm') });
        cursor.add(1, 'hour');
      }
      return ticks;
    },
    // Una riga per categoria, ma SOLO quelle con almeno un minuto di
    // attività oggi — niente righe vuote per categorie definite ma non
    // toccate nella giornata corrente. App non categorizzate non
    // producono nessuna riga (categoryForApp torna null): a differenza
    // del treemap, qui non ha senso una riga "Non categorizzato" — il
    // punto del modulo è mostrare LE TUE categorie nell'arco della
    // giornata, non censire ogni singolo evento.
    // `cells` è la frazione (0..1) di ciascuno slot da 15 minuti
    // davvero coperta da attività di quella categoria — non solo "sì/no
    // c'era attività", richiesta esplicita dell'utente dopo aver notato
    // che nel mockup i quadratini erano sfumati: uno slot occupato per
    // intero è a colore pieno, uno con solo pochi minuti reali (es. un
    // cambio app a metà slot) è più sbiadito — vedi cellOpacity() nei
    // methods per come la frazione diventa opacità.
    rows(): Row[] {
      if (!this.bounds || this.slotCount === 0) return [];
      const { start } = this.bounds;
      const slotCount = this.slotCount;
      const perCategory = new Map<string, number[]>();
      const totalPerCategory = new Map<string, number>();

      // Somma, per ciascuno slot toccato dall'evento, i secondi VERI di
      // sovrapposizione tra evento e slot — non l'intero slot marcato
      // in blocco (quello darebbe sempre "pieno", anche per un evento
      // di 30 secondi a cavallo di due slot). Accumula su più eventi
      // che cadono nello stesso slot (es. due sessioni Claude separate
      // nello stesso quarto d'ora), convertito in frazione 0..1 solo
      // alla fine.
      const markRange = (cellSeconds: number[], evStart: moment.Moment, evEnd: moment.Moment) => {
        const s = Math.max(0, Math.floor(evStart.diff(start, 'minutes') / SLOT_MINUTES));
        const e = Math.min(slotCount, Math.ceil(evEnd.diff(start, 'minutes') / SLOT_MINUTES));
        for (let i = s; i < e; i++) {
          const slotStart = start.clone().add(i * SLOT_MINUTES, 'minutes');
          const slotEnd = slotStart.clone().add(SLOT_MINUTES, 'minutes');
          const overlapStart = moment.max(evStart, slotStart);
          const overlapEnd = moment.min(evEnd, slotEnd);
          cellSeconds[i] += Math.max(0, overlapEnd.diff(overlapStart, 'seconds'));
        }
      };

      // clippedWindowEvents (non rawWindowEvents) — vedi il suo commento
      // sopra: esclude le porzioni in cui l'utente era davvero afk,
      // così un'app rimasta a fuoco durante una pausa reale non finisce
      // per sembrare "attività" nella griglia.
      for (const ev of this.clippedWindowEvents as any[]) {
        const app = ev.data && ev.data.app;
        if (!app) continue;
        const durata = ev.duration || 0;
        if (durata <= 0) continue;
        const categoria = this.categorieStore.categoryForApp(app);
        if (!categoria) continue;
        if (!perCategory.has(categoria)) {
          perCategory.set(categoria, new Array(slotCount).fill(0));
          totalPerCategory.set(categoria, 0);
        }
        const evStart = moment(ev.timestamp);
        markRange(perCategory.get(categoria) as number[], evStart, evStart.clone().add(durata, 'seconds'));
        totalPerCategory.set(categoria, (totalPerCategory.get(categoria) as number) + durata);
      }

      const slotSeconds = SLOT_MINUTES * 60;
      return [...perCategory.entries()]
        .map(([name, cellSeconds]) => ({
          key: name,
          name,
          color: this.categorieStore.colorForCategoryName(name),
          cells: cellSeconds.map(s => Math.min(1, s / slotSeconds)),
          totalSeconds: totalPerCategory.get(name) || 0,
        }))
        .sort((a, b) => b.totalSeconds - a.totalSeconds);
    },
    // Linea sottile continua, non slot discreti — richiesta esplicita
    // dell'utente, stessa idea della barra AFK della Timeline
    // (afkStatusSegments in HomeTimelineSection.vue): `left`/`width` in
    // percentuale sull'arco bounds.start–bounds.end, calcolati
    // direttamente dagli orari veri degli intervalli afk invece di
    // passare per gli slot da 15 minuti (che restano usati solo dalle
    // righe categoria sopra).
    // Intervalli in cui una app marcata "sempre attiva" (Impostazioni →
    // Orari di lavoro → App sempre attive) era a fuoco — stessa idea già
    // usata da afkStatusSegments in HomeTimelineSection.vue: un ping afk
    // che cade dentro uno di questi intervalli NON conta come
    // distrazione (es. desktop remoto/foglio Excel guardati senza
    // toccare tastiera/mouse, ma comunque "lavoro" per l'utente).
    alwaysActiveIntervals(): { start: moment.Moment; end: moment.Moment }[] {
      const apps = this.settingsStore.always_active_apps;
      if (!apps || !apps.length || !this.rawWindowEvents.length) return [];
      const set = new Set(apps);
      return (this.rawWindowEvents as any[])
        .filter(e => e.data && set.has(e.data.app))
        .map(e => ({
          start: moment(e.timestamp),
          end: moment(e.timestamp).add(e.duration || 0, 'seconds'),
        }));
    },
    // Intervalli not-afk (+ le app "sempre attive" sopra) — usati per
    // "ripulire" gli eventi finestra prima di costruire le righe
    // categoria, stesso identico filtro già applicato dalla Timeline
    // (vedi clipEventsToIntervals in HomeTimelineSection.vue). Bug
    // segnalato dall'utente: una finestra rimasta a fuoco durante una
    // pausa pranzo reale (nessun cambio finestra, quindi il watcher la
    // segna ancora "a fuoco" anche senza nessun input) risultava attiva
    // nei quadratini anche se l'utente era davvero assente — il watcher
    // finestra guarda solo il focus, non l'input, e senza questo filtro
    // la griglia non aveva modo di saperlo.
    notAfkIntervals(): { start: moment.Moment; end: moment.Moment }[] {
      return (this.rawAfkEvents as any[])
        .filter(e => e.data && e.data.status === 'not-afk')
        .map(e => ({
          start: moment(e.timestamp),
          end: moment(e.timestamp).add(e.duration || 0, 'seconds'),
        }))
        .concat(this.alwaysActiveIntervals);
    },
    clippedWindowEvents(): any[] {
      return clipEventsToIntervals(this.rawWindowEvents, this.notAfkIntervals);
    },
    distractionSegments(): { left: number; width: number }[] {
      if (!this.bounds) return [];
      const { start, end } = this.bounds;
      const totalMinutes = end.diff(start, 'minutes');
      if (totalMinutes <= 0) return [];

      const overrideActive = (t: moment.Moment) =>
        this.alwaysActiveIntervals.some((iv: any) => !t.isBefore(iv.start) && !t.isAfter(iv.end));

      const intervals = (this.rawAfkEvents as any[])
        .filter(e => e.data && e.data.status === 'afk' && !overrideActive(moment(e.timestamp)))
        .map(e => ({
          start: moment(e.timestamp),
          end: moment(e.timestamp).add(e.duration || 0, 'seconds'),
        }))
        .sort((a, b) => a.start.valueOf() - b.start.valueOf());

      for (let i = 1; i < intervals.length; i++) {
        const prev = intervals[i - 1];
        const cur = intervals[i];
        const gapSeconds = cur.start.diff(prev.end, 'seconds');
        if (gapSeconds > 0 && gapSeconds <= GAP_BRIDGE_SECONDS) prev.end = cur.start;
      }

      const segments: { left: number; width: number }[] = [];
      for (const iv of intervals) {
        const clampedStart = moment.max(iv.start, start);
        const clampedEnd = moment.min(iv.end, end);
        if (!clampedEnd.isAfter(clampedStart)) continue;
        segments.push({
          left: (clampedStart.diff(start, 'minutes') / totalMinutes) * 100,
          width: (clampedEnd.diff(clampedStart, 'minutes') / totalMinutes) * 100,
        });
      }
      return segments;
    },
  },
  watch: {
    date() {
      this.load();
    },
    host() {
      this.load();
    },
  },
  mounted() {
    this.load();
  },
  // --- INIZIO: hover quadratino → due linee in Timeline ---
  beforeDestroy() {
    // Il modulo può sparire (rimosso da "Modifica moduli") mentre il
    // mouse ci sta ancora sopra — senza questo, le due linee resterebbero
    // disegnate per sempre nella Timeline, orfane di qualunque hover.
    if (this.hoverClearTimer) clearTimeout(this.hoverClearTimer);
    this.highlightStore.clearHoveredRange();
  },
  // --- FINE ---
  methods: {
    cellOpacity(coverage: number): number {
      return MIN_CELL_OPACITY + coverage * (1 - MIN_CELL_OPACITY);
    },
    // --- INIZIO: hover quadratino → due linee in Timeline ---
    onCellHover(slotIndex: number) {
      // Un nuovo hover annulla sempre la cancellazione ritardata in
      // sospeso (vedi hoverClearTimer in data()) — è esattamente questo
      // a far "scivolare" le linee tra celle vicine invece di farle
      // sparire e ricomparire.
      if (this.hoverClearTimer) {
        clearTimeout(this.hoverClearTimer);
        this.hoverClearTimer = null;
      }
      if (!this.bounds) return;
      const start = (this.bounds.start as moment.Moment).clone().add(slotIndex * SLOT_MINUTES, 'minutes');
      const end = start.clone().add(SLOT_MINUTES, 'minutes');
      this.highlightStore.setHoveredRange(start.toISOString(), end.toISOString());
    },
    onCellLeave() {
      if (this.hoverClearTimer) clearTimeout(this.hoverClearTimer);
      this.hoverClearTimer = setTimeout(() => {
        this.highlightStore.clearHoveredRange();
        this.hoverClearTimer = null;
      }, 60);
    },
    // --- FINE ---
    async load() {
      const [windowEvents, afkEvents] = await Promise.all([
        this.fetchEvents('aw-watcher-window', this.dayStart, this.dayEnd),
        this.fetchEvents('aw-watcher-afk', this.dayStart, this.dayEnd),
      ]);
      // Stesso "non rifare il lavoro se niente è davvero cambiato" di
      // ActivityHeatmap.vue — questo modulo ricalcola griglia/righello
      // interi ad ogni load, non ha senso rifarlo ad ogni poll di 30s se
      // i due bucket non hanno prodotto eventi diversi.
      const signature = eventListSignature([windowEvents, afkEvents]);
      if (signature === this.lastSignature) {
        this.loading = false;
        return;
      }
      this.lastSignature = signature;
      this.rawWindowEvents = windowEvents;
      this.rawAfkEvents = afkEvents;
      this.loading = false;
    },
  },
};
</script>

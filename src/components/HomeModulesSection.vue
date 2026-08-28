<template lang="pug">
div.modules-section(v-if="view")
  div.modules-head
    div.grid-head {{ $t('home.modules.title') }}
    div.modules-actions
      template(v-if="editing")
        div.pill-btn(@click="addVisualization") {{ $t('home.modules.addModule') }}
        div.pill-btn-ghost(@click="cancelEdit") {{ $t('home.modules.cancel') }}
        div.pill-btn(@click="saveEdit") {{ $t('home.modules.save') }}
      template(v-else)
        div.pill-btn-ghost(@click="startEdit") {{ $t('home.modules.editModules') }}

  // Vero motore di impaccamento 2D (non più vuedraggable/SortableJS su
  // colonne flex indipendenti — riscrittura completa, 2026-08-13,
  // richiesta esplicita dell'utente dopo aver scoperto che il modulo
  // "doppio" Categorie, vivendo in una banda a sé, non aveva nessun
  // altro elemento con cui scambiarsi trascinandolo). Ogni card è
  // posizionata in assoluto (computePacking, vedi script) dentro questo
  // unico contenitore relative — permette a un modulo di larghezza
  // qualsiasi (1 colonna, 2 colonne, intera) di essere trascinato
  // ovunque nella sequenza, non solo dentro il proprio "gruppo" di
  // taglia. Il trascinamento stesso è implementato a mano con soli
  // eventi mouse (non drag-and-drop nativo HTML5, né SortableJS) —
  // stessa lezione appresa poco fa: il drag nativo è inaffidabile
  // dentro un webview embedded come questo.
  div.modules-masonry(ref="masonryEl" :style="{ height: packed.totalHeight + 'px' }")
    // Riquadro tratteggiato che mostra dove il modulo trascinato
    // atterrerebbe se rilasciato ora — riusa esattamente la posizione già
    // calcolata da packed.layout per l'elemento in trascinamento (con
    // colOverride, vedi script), quindi resta sempre sincronizzato col
    // vero risultato dell'impaccamento invece di essere una stima a parte.
    div.modules-drop-placeholder(v-if="dropPlaceholder" :style="dropPlaceholderStyle")
    // :key su el.id (stabile per card, vedi IElement in stores/views.ts),
    // NON su el.__idx (posizione nell'array, cambia ad ogni riordino) —
    // altrimenti Vue riusa il nodo DOM sbagliato dopo un trascinamento,
    // portandosi dietro la posizione CSS di un'ALTRA card come punto di
    // partenza della transizione. __idx resta usato solo internamente
    // (itemStyle/ResizeObserver/dragging), sempre coerente con se stesso
    // ad ogni singolo render.
    div.modules-grid-item(
      v-for="el in renderOrder"
      :key="el.id || el.__idx"
      :data-idx="el.__idx"
      :data-height-key="el.id || el.__idx"
      :ref="'card-' + el.__idx"
      :class="{ 'modules-grid-item-dragging': dragging && dragging.idx === el.__idx }"
      :style="itemStyle(el.__idx)"
      @mousedown="onWrapperMouseDown($event, el.__idx)"
    )
      aw-selectable-vis(
        :id="el.__idx"
        :type="el.type"
        :props="el.props"
        :view-id="view.id"
        :editable="editing"
        @onTypeChange="onTypeChange"
        @onRemove="onRemove"
        @onVisibilityChange="onVisibilityChange"
      )

  // Bug reale segnalato dall'utente: un modulo nascosto per mancanza di
  // dati (es. VS Code non ancora usato oggi) restava nascosto anche
  // dopo l'arrivo di dati veri più tardi nella giornata, tornando
  // visibile solo entrando in "Modifica moduli" — perché uscendo dal
  // v-for della masonry sopra (renderOrder esclude i nascosti) la sua
  // istanza Vue veniva distrutta insieme al watcher su `visibile` che
  // avrebbe dovuto farlo riapparire da solo. Questo blocco le tiene
  // montate (invisibili, fuori dal flusso) SOLO per continuare a
  // valutare i loro dati — appena uno smette di essere vuoto,
  // onVisibilityChange lo toglie da nascostiPerAssenzaDati e rientra
  // da solo nella masonry sopra. In modifica non serve: editOrder le
  // include già visibilmente lì.
  div.modules-hidden-probes(v-if="!editing")
    aw-selectable-vis(
      v-for="el in hiddenIndexedElements"
      :key="'hidden-' + el.__idx"
      :id="el.__idx"
      :type="el.type"
      :props="el.props"
      :view-id="view.id"
      :editable="false"
      @onVisibilityChange="onVisibilityChange"
    )

  custom-module-picker(
    v-if="wizardEntryType"
    :entry-type="wizardEntryType"
    @close="wizardEntryType = null"
    @created="onWizardCreated"
  )
</template>

<style lang="scss" scoped>
@import '../style/theme.css';
@import '../style/modals.css';
@import '../style/projectCards.css';

.modules-section {
  padding: 0 28px 24px;
}

.modules-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}

.modules-actions {
  display: flex;
  gap: 8px;
}

.modules-masonry {
  position: relative;
  width: 100%;
}

// Vedi il commento nel template sopra — montate solo per tenere vivo il
// loro watcher su `visibile`, mai da mostrare né da far occupare spazio.
.modules-hidden-probes {
  display: none;
}

.modules-grid-item {
  box-sizing: border-box;
}

.modules-grid-item-dragging {
  z-index: 50;
  pointer-events: none;
  box-shadow: var(--shadow-elevated);
}

.modules-drop-placeholder {
  position: absolute;
  border: 1.5px dashed color-mix(in srgb, var(--color-accent1) 45%, transparent);
  border-radius: var(--radius-md);
  background-color: color-mix(in srgb, var(--color-accent1) 6%, transparent);
  pointer-events: none;
  z-index: 1;
  transition: transform 0.15s ease, width 0.15s ease, height 0.15s ease;
}
</style>

<script lang="ts">
// The old app's "views" system (Summary/Window/Browser/Editor tabs,
// "+ Add visualization", drag-to-reorder) rendered here directly in
// our themed Home area — same underlying data (useViewsStore) and
// same aw-selectable-vis dispatch as the legacy Activity page, just a
// new place to render the default ("Summary") view's modules.
//
// This component now ALSO owns loading activityStore itself (the old
// Activity.vue/ActivityView.vue Bootstrap page has been removed
// entirely — explicit request, along with the period-switching
// day/week/month/year toolbar and the Summary/Window/Browser/Editor
// tab system, neither of which is coming back). Top Applications/
// Titles/Domini/Categorie read straight from activityStore (see
// SelectableVisualization.vue), and nothing populates that store on
// its own — Activity.vue used to be the thing that did, purely as a
// side effect of also being mounted on this page. Since it's gone,
// this component is the new owner: same ensure_loaded() call, same
// default query options (day period, AFK-filtered, audible+stopwatch
// included, no category filter — Activity.vue's own defaults), plus
// the same 30s auto-refresh as the rest of Home (see BLUEPRINT.md
// section 6.9).
import { useViewsStore } from '~/stores/views';
import { useActivityStore, QueryOptions } from '~/stores/activity';
import { useSettingsStore } from '~/stores/settings';
import { useBucketsStore } from '~/stores/buckets';
import { get_day_start_with_offset, get_today_with_offset } from '~/util/time';
import { useClockStore } from '~/stores/clock';
import { TimePeriod } from '~/util/timeperiod';
import { getHomeClient } from '~/util/awclient';

// Moduli a piena larghezza (span = tutte le colonne) — nessun tipo la
// usa più oggi (i due che la popolavano, sunburst_clock e vis_timeline,
// sono stati rimossi, vedi BLUEPRINT.md sezione 16), lasciata pronta
// per un futuro modulo "large". workflow_grid l'aveva usata inizialmente
// ma è stato riportato a 3 colonne su richiesta esplicita (vedi
// TRIPLE_TYPES sotto) — troppo largo a tutta pagina.
const LARGE_TYPES: string[] = [];
// Moduli larghi 2 colonne (es. Categorie) — vedi BLUEPRINT.md sezione
// 17/18 per la storia completa di come si è arrivati a un vero motore
// di impaccamento invece di una banda a parte. activity_heatmap
// (calendario stile "contributi GitHub") ci si è spostato qui da
// TRIPLE_TYPES su richiesta esplicita, in coppia con la finestra
// temporale portata a 6 mesi (vedi GIORNI_FINESTRA in
// ActivityHeatmap.vue) — è la combinazione larghezza/durata che riempie
// la card senza lasciarla vuota a destra né farla traboccare. workflow_grid
// (griglia categorie×slot da 15 minuti) è passato di qui in due tappe su
// richiesta esplicita dell'utente — prima tutta larghezza (LARGE_TYPES),
// poi 3 colonne (TRIPLE_TYPES), infine 2 — resta comunque scrollabile in
// orizzontale se il contenuto non entra (vedi .workflow-grid-scroll).
const DOUBLE_TYPES = ['top_categories', 'activity_heatmap', 'workflow_grid'];
// Moduli larghi 3 colonne — category_treemap (treemap categorie→app)
// richiede esplicitamente questa larghezza (richiesta dell'utente: "x3,
// non x2 come Categorie/Calendario") per avere spazio sufficiente a
// mostrare più categorie fianco a fianco senza schiacciarle.
const TRIPLE_TYPES: string[] = ['category_treemap'];
const COLUMN_MIN_WIDTH = 280;
const COLUMN_GAP = 14;
// Altezza di riserva per una card non ancora misurata dal
// ResizeObserver (primo render) — solo una stima ragionevole, corretta
// al volo non appena arriva la prima misura reale.
const FALLBACK_ITEM_HEIGHT = 180;

interface DragState {
  idx: number;
  width: number;
  height: number;
  offsetX: number;
  offsetY: number;
  cursorX: number;
  cursorY: number;
  // Posizione REALE del mouse (relativa al container), separata da
  // cursorX/cursorY (che sono l'angolo in alto a sinistra della card
  // trascinata, usato solo per il rendering) — vedi il commento sopra
  // previewOrder per il perché serve tenerle distinte.
  mouseX: number;
  mouseY: number;
}

interface PackedLayout {
  layout: Record<number, { x: number; y: number; width: number; height: number }>;
  totalHeight: number;
}

export default {
  name: 'HomeModulesSection',
  components: {
    'custom-module-picker': () => import('./CustomModulePicker.vue'),
  },
  data() {
    return {
      editing: false,
      // Tipo di modulo per cui è aperto il wizard di creazione ('watcher'
      // | 'html' | null se chiuso) — vedi onTypeChange/onWizardCreated.
      wizardEntryType: null as 'watcher' | 'html' | null,
      wizardTargetElId: null as number | null,
      columnCount: 4,
      columnWidthPx: 0,
      activityStore: useActivityStore(),
      settingsStore: useSettingsStore(),
      bucketsStore: useBucketsStore(),
      refreshInterval: null as ReturnType<typeof setInterval> | null,
      lastBucketsSignature: null as string | null,
      // Debounce del cambio giorno/host — vedi il watcher `date()` sotto
      // per il perché (stesso problema e stessa soluzione di
      // HomeTimelineSection.vue: scorrimento veloce tra i giorni
      // attraversava ogni giorno di passaggio con una query completa).
      timerDebounceCambioGiorno: null as ReturnType<typeof setTimeout> | null,
      // Indici (__idx) dei moduli che si sono nascosti da soli per
      // mancanza di dati (Excel/VPN/VoiSpeed — vedi il commento su
      // `visibile` in SelectableVisualization.vue) — popolato via
      // l'evento onVisibilityChange emesso da ciascuna card. Oggetto
      // reattivo (non un Set: Vue 2 non traccia le mutazioni di
      // Set/Map) aggiornato con $set/$delete.
      nascostiPerAssenzaDati: {} as Record<number, boolean>,
      // Altezza reale misurata di ciascuna card (via ResizeObserver,
      // vedi observeItems) — chiave el.id (stabile), NON __idx. Bug
      // reale segnalato dall'utente dopo l'introduzione della colonna
      // fissa: __idx è la posizione nell'array, cambia ad ogni
      // riordino (stesso identico problema già risolto per :key sopra,
      // qui non ancora applicato) — dopo un trascinamento, l'altezza
      // registrata per una posizione poteva restare quella del modulo
      // che stava lì PRIMA, finché il ResizeObserver non la
      // ricorreggeva un istante dopo. computePacking usava quel valore
      // sbagliato nel frattempo, producendo sovrapposizioni/spazi
      // vuoti. Finché una card non è stata misurata almeno una volta
      // si usa FALLBACK_ITEM_HEIGHT.
      itemHeights: {} as Record<string, number>,
      ro: null as ResizeObserver | null,
      // Stato del trascinamento in corso, null quando non si sta
      // trascinando nulla — vedi onWrapperMouseDown/onDragMouseMove/
      // onDragMouseUp. Implementato a mano (solo eventi mouse, niente
      // drag-and-drop nativo HTML5) per lo stesso motivo per cui è
      // stato disattivato il drag nativo nel resto della sezione 18:
      // inaffidabile dentro un webview Tauri/WebView2.
      dragging: null as DragState | null,
    };
  },
  computed: {
    // TrackFlow è mono-client: l'host non è mai un parametro di rotta,
    // si risolve sempre dal buckets store.
    host(): string {
      return this.bucketsStore.host;
    },
    // Same fallback Activity.vue's own `_date` computed used. Bug reale
    // segnalato dall'utente: essendo un computed Vue, questo NON si
    // ricalcolava da solo al passare della mezzanotte/dell'ora di inizio
    // giornata — get_today_with_offset() legge l'ora reale, che Vue non
    // traccia come dipendenza, quindi restava congelato al valore della
    // prima valutazione finché route.params.date o startOfDay non
    // cambiavano per altri motivi (es. cambiare giorno a mano nel
    // selettore, che per questo "sistemava" il bug). Il tick di
    // stores/clock.ts, letto qui solo per il suo effetto collaterale,
    // forza un ricalcolo entro un minuto dal vero cambio giorno.
    date(): string {
      useClockStore().tick;
      return (
        (this.$route.params.date as string) || get_today_with_offset(this.settingsStore.startOfDay)
      );
    },
    // Always a single day — the old week/month/year/7d/30d period
    // switcher lived entirely in the now-removed Activity.vue toolbar
    // and was deliberately not carried over.
    timeperiod(): TimePeriod {
      return {
        start: get_day_start_with_offset(this.date, this.settingsStore.startOfDay),
        length: [1, 'day'],
      };
    },
    views(): import('~/stores/views').View[] {
      return useViewsStore().views;
    },
    // The "default" view — same one the legacy Summary tab used to
    // show (first in the list).
    view(): import('~/stores/views').View | undefined {
      return this.views[0];
    },
    elements: {
      get() {
        return this.view ? this.view.elements : [];
      },
      set(elements: any[]) {
        if (!this.view) return;
        useViewsStore().setElements({ view_id: this.view.id, elements });
      },
    },
    indexedElements(): any[] {
      return this.elements.map((el: any, idx: number) => ({ ...el, __idx: idx }));
    },
    // Solo i moduli con dati/visibili — usato per la vista normale.
    visibleIndexedElements(): any[] {
      return this.indexedElements.filter((el: any) => !this.nascostiPerAssenzaDati[el.__idx]);
    },
    // Moduli attualmente nascosti per mancanza di dati — vedi anche il
    // blocco "modules-hidden-probes" nel template: un modulo che si
    // nasconde da solo esce dal v-for della masonry (renderOrder sotto),
    // quindi la sua istanza Vue verrebbe distrutta e con lei il watcher
    // su `visibile` che potrebbe farlo ricomparire da solo quando torna
    // ad avere dati — bug reale segnalato dall'utente (VS Code nascosto
    // restava nascosto tutto il giorno pur avendo dati veri, tornava
    // visibile solo entrando in modifica moduli). Questo elenco serve a
    // tenerle "vive" altrove, invisibili, solo per continuare a
    // guardare i dati.
    hiddenIndexedElements(): any[] {
      return this.indexedElements.filter((el: any) => this.nascostiPerAssenzaDati[el.__idx]);
    },
    // In modifica: gli stessi moduli visibili, NELLO STESSO ORDINE già
    // mostrato (non ricalcolato da indexedElements grezzo) — così
    // entrare in modifica non sposta nulla di già visibile, requisito
    // esplicito dell'utente (sezione 6sexies) — poi i moduli nascosti
    // per mancanza di dati vengono accodati alla fine, visibili SOLO
    // qui per poterli comunque gestire/rimuovere.
    editOrder(): any[] {
      return [...this.visibleIndexedElements, ...this.hiddenIndexedElements];
    },
    baseOrder(): any[] {
      return this.editing ? this.editOrder : this.visibleIndexedElements;
    },
    // Durante un trascinamento, l'ordine "live" con l'elemento
    // trascinato spostato nella posizione sotto il cursore — vedi
    // computeInsertionIndex. Fuori da un trascinamento coincide con
    // baseOrder.
    previewOrder(): any[] {
      if (!this.dragging) return this.baseOrder;
      const order = this.baseOrder.slice();
      const fromIdx = order.findIndex((e: any) => e.__idx === (this.dragging as DragState).idx);
      if (fromIdx === -1) return order;
      const [item] = order.splice(fromIdx, 1);
      const d = this.dragging as DragState;
      // Bug reale segnalato dall'utente: usare il centro della card
      // trascinata (invece della posizione vera del mouse) faceva
      // sembrare lo sgancio "duro" ogni volta che la card in mano aveva
      // dimensioni diverse dai moduli bersaglio — il punto valutato non
      // corrispondeva a dove l'utente vedeva davvero il cursore.
      const insertAt = this.computeInsertionIndex(order, d.mouseX, d.mouseY);
      order.splice(insertAt, 0, item);
      return order;
    },
    renderOrder(): any[] {
      return this.dragging ? this.previewOrder : this.baseOrder;
    },
    // Layout impaccato (posizione/larghezza in px per ogni __idx) per
    // l'ordine attualmente da mostrare — vedi computePacking. Durante un
    // trascinamento, la colonna dell'elemento trascinato viene forzata a
    // quella sotto il cursore (invece della sua vecchia col, o
    // dell'euristica "colonna più corta") — così l'intera griglia si
    // ridispone dal vivo mostrando dove finirebbe davvero se rilasciato
    // ora, non solo dopo il rilascio.
    packed(): PackedLayout {
      const override = this.dragging
        ? { [(this.dragging as DragState).idx]: this.columnFromX((this.dragging as DragState).mouseX) }
        : undefined;
      return this.computePacking(this.renderOrder, override);
    },
    // Riquadro tratteggiato mostrato durante il trascinamento, nella
    // stessa identica posizione/dimensione che packed.layout ha già
    // calcolato per l'elemento trascinato (grazie all'override sopra) —
    // richiesta esplicita: rendere visibile dove il modulo atterrerebbe.
    dropPlaceholder(): { x: number; y: number; width: number; height: number } | null {
      if (!this.dragging) return null;
      return this.packed.layout[(this.dragging as DragState).idx] || null;
    },
    dropPlaceholderStyle(): Record<string, string> {
      const pos = this.dropPlaceholder;
      if (!pos) return {};
      return {
        transform: `translate(${pos.x}px, ${pos.y}px)`,
        width: pos.width + 'px',
        height: pos.height + 'px',
      };
    },
  },
  watch: {
    // Bug reale segnalato dall'utente: cambiando giorno/host, i moduli
    // sparivano del tutto per un istante (start_loading() azzerava i
    // dati a null PRIMA che i nuovi arrivassero) invece di aggiornarsi
    // sul posto — esattamente il problema che il flag `background` di
    // ensure_loaded() risolve già, finora usato solo dall'auto-refresh
    // periodico (vedi checkForUpdatesAndReload sotto). Passandolo anche
    // qui, i vecchi valori (nomi app, icone, barre) restano visibili
    // finché i nuovi non sono pronti, poi vengono sostituiti sul posto
    // — stessa transizione morbida dell'auto-refresh, non più uno
    // sparire-e-riapparire.
    date() {
      this.avviaCaricamentoConDebounce();
    },
    host() {
      this.avviaCaricamentoConDebounce();
    },
  },
  async mounted() {
    this.ro = new ResizeObserver((entries: ResizeObserverEntry[]) => {
      for (const entry of entries) {
        const key = (entry.target as HTMLElement).dataset.heightKey;
        if (key === undefined) continue;
        const h = entry.contentRect.height;
        if (Math.abs((this.itemHeights[key] || 0) - h) > 0.5) {
          this.$set(this.itemHeights, key, h);
        }
      }
    });

    await useViewsStore().load();
    try {
      await this.loadActivityData();
    } catch (e) {
      if (e.message !== 'canceled') console.error(e);
    }
    this.measureColumns();
    window.addEventListener('resize', this.measureColumns);
    this.$nextTick(() => this.observeItems());
    this.lastBucketsSignature = await this.currentBucketsSignature();
    this.refreshInterval = setInterval(() => this.checkForUpdatesAndReload(), 30000);
  },
  updated() {
    this.$nextTick(() => this.observeItems());
  },
  beforeDestroy() {
    window.removeEventListener('resize', this.measureColumns);
    window.removeEventListener('mousemove', this.onDragMouseMove);
    window.removeEventListener('mouseup', this.onDragMouseUp);
    if (this.ro) this.ro.disconnect();
    if (this.refreshInterval) clearInterval(this.refreshInterval);
    if (this.timerDebounceCambioGiorno) clearTimeout(this.timerDebounceCambioGiorno);
    // Cancels pending requests and resets the store — same cleanup
    // Activity.vue's own beforeDestroy used to do when leaving the page.
    this.activityStore.reset();
  },
  methods: {
    // Richiesta esplicita dell'utente dopo l'indagine di performance:
    // scorrere velocemente tra i giorni lanciava una query completa per
    // ogni giorno di passaggio, anche se l'utente si fermava solo
    // sull'ultimo — lavoro vero sprecato (rete + calcolo lato server),
    // non solo un render di troppo. Aspetta 200ms di "silenzio" (nessun
    // altro cambio giorno/host) prima di partire sul serio — sempre con
    // `background: true` (vedi il commento sul watcher `date()`), così
    // anche il primo aggiornamento dopo l'attesa resta morbido invece
    // di far sparire tutto.
    avviaCaricamentoConDebounce() {
      if (this.timerDebounceCambioGiorno) {
        clearTimeout(this.timerDebounceCambioGiorno);
      }
      this.timerDebounceCambioGiorno = setTimeout(() => {
        this.timerDebounceCambioGiorno = null;
        this.loadActivityData(false, true);
      }, 200);
    },
    async loadActivityData(force = false, background = false) {
      const queryOptions: QueryOptions = {
        host: this.host,
        timeperiod: this.timeperiod,
        force,
        background,
        filter_afk: true,
        include_audible: true,
        include_stopwatch: true,
        always_active_apps: this.settingsStore.always_active_apps,
      };
      await this.activityStore.ensure_loaded(queryOptions);
    },
    async currentBucketsSignature(): Promise<string> {
      try {
        const buckets = await getHomeClient().getBuckets();
        this.bucketsStore.update_buckets(Object.values(buckets));
        return Object.values(buckets)
          .map((b: any) => `${b.id}:${b.last_updated}`)
          .sort()
          .join('|');
      } catch {
        return '';
      }
    },
    async checkForUpdatesAndReload() {
      const signature = await this.currentBucketsSignature();
      if (signature && signature === this.lastBucketsSignature) return;
      this.lastBucketsSignature = signature;
      try {
        await this.loadActivityData(true, true);
      } catch (e) {
        if (e.message !== 'canceled') console.error(e);
      }
    },
    isLarge(type: string): boolean {
      return LARGE_TYPES.includes(type);
    },
    isTriple(type: string): boolean {
      return TRIPLE_TYPES.includes(type);
    },
    isDouble(type: string): boolean {
      return DOUBLE_TYPES.includes(type);
    },
    measureColumns() {
      this.$nextTick(() => {
        const el = this.$refs.masonryEl as HTMLElement | undefined;
        const width = el ? el.clientWidth : 0;
        if (width <= 0) return;
        const columnsThatFit = Math.floor((width + COLUMN_GAP) / (COLUMN_MIN_WIDTH + COLUMN_GAP));
        this.columnCount = Math.max(1, columnsThatFit);
        this.columnWidthPx = (width - (this.columnCount - 1) * COLUMN_GAP) / this.columnCount;
      });
    },
    // Attacca il ResizeObserver ad ogni card attualmente renderizzata —
    // observe() è sicuro da richiamare più volte sullo stesso elemento
    // (nessuna callback duplicata), quindi rifarlo ad ogni update non
    // ha effetti collaterali, solo un po' di lavoro ridondante quando
    // niente di nuovo è comparso.
    observeItems() {
      if (!this.ro) return;
      for (const el of this.baseOrder) {
        const ref = this.$refs['card-' + el.__idx] as HTMLElement | HTMLElement[] | undefined;
        const node = Array.isArray(ref) ? ref[0] : ref;
        if (node) (this.ro as ResizeObserver).observe(node);
      }
    },
    // Colonna (indice 0-based, entro i limiti correnti) sotto una
    // coordinata X del contenitore — condivisa da packed (override live
    // durante il trascinamento), computeInsertionIndex e onDragMouseUp
    // (assegnazione finale al rilascio), così i tre punti concordano
    // sempre sulla stessa identica colonna per lo stesso cursore.
    columnFromX(x: number): number {
      const colW = this.columnWidthPx > 0 ? this.columnWidthPx : COLUMN_MIN_WIDTH;
      const colUnit = colW + COLUMN_GAP;
      return Math.min(this.columnCount - 1, Math.max(0, Math.floor(x / colUnit)));
    },
    // Motore di impaccamento: scorre `order` e per ciascun elemento
    // determina la colonna di partenza. Se l'elemento ha una `col`
    // esplicita (impostata trascinandolo — vedi onDragMouseUp — oppure
    // forzata da `colOverride` durante un trascinamento live, vedi
    // `packed`), quella colonna è FISSA: l'elemento si impila lì anche
    // se ne risulta una colonna più alta delle altre — richiesta
    // esplicita dell'utente, per poter mettere un modulo sotto un altro
    // più alto invece di essere sempre ribilanciato altrove. Solo gli
    // elementi MAI spostati manualmente (nessuna `col`) continuano a
    // usare la vecchia euristica "colonna più corta", riorganizzandosi
    // da soli intorno a quelli ormai fissi.
    computePacking(order: any[], colOverride?: Record<number, number>): PackedLayout {
      const colHeights = new Array(this.columnCount).fill(0);
      const layout: Record<number, { x: number; y: number; width: number; height: number }> = {};
      const colW = this.columnWidthPx > 0 ? this.columnWidthPx : COLUMN_MIN_WIDTH;
      for (const el of order) {
        // Larghezza per-istanza (es. i moduli creati dal wizard watcher
        // personalizzato, che lasciano scegliere la dimensione alla
        // creazione) ha priorità sulla larghezza fissa per TIPO usata da
        // tutti gli altri moduli — additivo, non tocca isLarge/isDouble.
        const gridWidth = el.props && typeof el.props.gridWidth === 'number' ? el.props.gridWidth : null;
        const span = gridWidth
          ? Math.min(this.columnCount, Math.max(1, Math.round(gridWidth)))
          : this.isLarge(el.type)
          ? this.columnCount
          : this.isTriple(el.type)
          ? Math.min(3, this.columnCount)
          : this.isDouble(el.type)
          ? Math.min(2, this.columnCount)
          : 1;
        const h = this.itemHeights[el.id || el.__idx] || FALLBACK_ITEM_HEIGHT;
        const maxStart = this.columnCount - span;

        const pinned =
          colOverride && colOverride[el.__idx] !== undefined ? colOverride[el.__idx] : el.col;
        let start: number;
        if (typeof pinned === 'number' && Number.isFinite(pinned)) {
          start = Math.min(maxStart, Math.max(0, Math.round(pinned)));
        } else {
          let bestStart = 0;
          let bestTop = Infinity;
          for (let s = 0; s <= maxStart; s++) {
            let top = 0;
            for (let c = s; c < s + span; c++) top = Math.max(top, colHeights[c]);
            if (top < bestTop) {
              bestTop = top;
              bestStart = s;
            }
          }
          start = bestStart;
        }

        let top = 0;
        for (let c = start; c < start + span; c++) top = Math.max(top, colHeights[c]);
        const x = start * (colW + COLUMN_GAP);
        const width = span * colW + (span - 1) * COLUMN_GAP;
        layout[el.__idx] = { x, y: top, width, height: h };
        for (let c = start; c < start + span; c++) colHeights[c] = top + h + COLUMN_GAP;
      }
      const totalHeight = Math.max(0, ...colHeights, COLUMN_GAP) - COLUMN_GAP;
      return { layout, totalHeight };
    },
    // Trova la posizione di inserimento ragionando per COLONNA, non per
    // distanza dal centro di ogni modulo della griglia intera — prima
    // versione (bug reale segnalato dall'utente): confrontare la
    // distanza euclidea da ogni card, indipendentemente da quale
    // colonna occupasse, faceva scattare il modulo "più vicino" in modo
    // imprevedibile vicino ai bordi tra colonne o tra righe di altezza
    // diversa — serviva azzeccare quasi il pixel esatto per far aprire
    // lo spazio. Ora: 1) la colonna bersaglio è quella sotto il cursore
    // per l'INTERA sua larghezza (non un punto), 2) dentro quella
    // colonna si cerca il modulo la cui metà (sopra/sotto il suo centro
    // verticale) contiene il cursore — zona di sgancio molto più ampia
    // e prevedibile, corrisponde alla struttura visiva reale invece che
    // a una metrica "invisibile" dipendente dal contenuto.
    computeInsertionIndex(order: any[], cursorX: number, cursorY: number): number {
      if (order.length === 0) return 0;
      const packing = this.computePacking(order);
      const colW = this.columnWidthPx > 0 ? this.columnWidthPx : COLUMN_MIN_WIDTH;
      const colUnit = colW + COLUMN_GAP;
      const targetCol = this.columnFromX(cursorX);

      // Moduli il cui span orizzontale copre la colonna bersaglio,
      // ordinati dall'alto in basso — sono i soli candidati per
      // decidere "prima/dopo" in base alla posizione verticale.
      const candidates = order.filter((el: any) => {
        const pos = packing.layout[el.__idx];
        if (!pos) return false;
        const startCol = Math.round(pos.x / colUnit);
        const span = Math.round((pos.width + COLUMN_GAP) / colUnit);
        return targetCol >= startCol && targetCol < startCol + span;
      });
      candidates.sort((a: any, b: any) => packing.layout[a.__idx].y - packing.layout[b.__idx].y);

      if (candidates.length === 0) {
        // Colonna vuota (bordo della griglia, non dovrebbe capitare in
        // pratica) — ripiego sul modulo più vicino in assoluto.
        let best = -1;
        let bestDist = Infinity;
        order.forEach((el: any, i: number) => {
          const pos = packing.layout[el.__idx];
          if (!pos) return;
          const d = Math.hypot(
            pos.x + pos.width / 2 - cursorX,
            pos.y + pos.height / 2 - cursorY
          );
          if (d < bestDist) {
            bestDist = d;
            best = i;
          }
        });
        return best === -1 ? order.length : best;
      }

      for (const el of candidates) {
        const pos = packing.layout[el.__idx];
        if (cursorY < pos.y + pos.height / 2) {
          return order.indexOf(el);
        }
      }
      const last = candidates[candidates.length - 1];
      return order.indexOf(last) + 1;
    },
    itemStyle(idx: number) {
      // Sempre position+transform, mai left/top — bug reale segnalato
      // dall'utente: usare left/top qui durante il trascinamento e
      // transform nello stato fermo faceva "saltare" la card al
      // rilascio (left/top azzerati di scatto passando da un ramo
      // all'altro, poi la transizione su transform partiva da quella
      // base sbagliata invece che dal vero punto di rilascio). Stessa
      // proprietà in entrambi i rami: il rilascio è solo un cambio di
      // valore di transform, quindi la transizione parte sempre dal
      // punto corretto.
      if (this.dragging && this.dragging.idx === idx) {
        const d = this.dragging;
        return {
          position: 'absolute',
          transform: `translate(${d.cursorX}px, ${d.cursorY}px)`,
          width: d.width + 'px',
          transition: 'none',
        };
      }
      const pos = this.packed.layout[idx];
      if (!pos) return { display: 'none' };
      return {
        position: 'absolute',
        transform: `translate(${pos.x}px, ${pos.y}px)`,
        width: pos.width + 'px',
        transition: 'transform 0.22s ease',
      };
    },
    // Solo l'handle (.vis-card-title, l'intestazione della card dentro
    // SelectableVisualization.vue) avvia un trascinamento — stessa
    // restrizione che prima dava l'opzione `handle` di SortableJS,
    // qui verificata a mano risalendo dal target del click. Così i
    // pulsanti tipo/rimuovi dentro la card restano cliccabili normalmente.
    onWrapperMouseDown(evt: MouseEvent, idx: number) {
      if (!this.editing) return;
      const target = evt.target as HTMLElement;
      if (!target.closest || !target.closest('.vis-card-title')) return;
      const masonryEl = this.$refs.masonryEl as HTMLElement;
      const rect = masonryEl.getBoundingClientRect();
      const pos = this.packed.layout[idx];
      if (!pos) return;
      evt.preventDefault();
      this.dragging = {
        idx,
        width: pos.width,
        height: pos.height,
        offsetX: evt.clientX - rect.left - pos.x,
        offsetY: evt.clientY - rect.top - pos.y,
        cursorX: pos.x,
        cursorY: pos.y,
        mouseX: evt.clientX - rect.left,
        mouseY: evt.clientY - rect.top,
      };
      window.addEventListener('mousemove', this.onDragMouseMove);
      window.addEventListener('mouseup', this.onDragMouseUp);
    },
    onDragMouseMove(evt: MouseEvent) {
      if (!this.dragging) return;
      const masonryEl = this.$refs.masonryEl as HTMLElement;
      const rect = masonryEl.getBoundingClientRect();
      this.dragging.cursorX = evt.clientX - rect.left - this.dragging.offsetX;
      this.dragging.cursorY = evt.clientY - rect.top - this.dragging.offsetY;
      this.dragging.mouseX = evt.clientX - rect.left;
      this.dragging.mouseY = evt.clientY - rect.top;
    },
    onDragMouseUp() {
      if (!this.dragging) return;
      const draggedIdx = this.dragging.idx;
      // Colonna sotto il cursore al momento del rilascio — diventa la
      // `col` fissa del modulo, stessa colonna già mostrata dal
      // placeholder tratteggiato durante il trascinamento (packed usa lo
      // stesso columnFromX come override live, vedi computed `packed`).
      const targetCol = this.columnFromX(this.dragging.mouseX);
      const finalOrder = this.previewOrder.map((el: any) =>
        el.__idx === draggedIdx ? { ...el, col: targetCol } : el
      );
      window.removeEventListener('mousemove', this.onDragMouseMove);
      window.removeEventListener('mouseup', this.onDragMouseUp);
      this.dragging = null;
      this.commitOrder(finalOrder);
    },
    commitOrder(order: any[]) {
      if (!this.view) return;
      // id preservato — vedi il commento su :key nel template: se qui
      // andasse perso, ogni riordino ripartirebbe da zero identità per
      // TUTTE le card, non solo quella appena spostata.
      this.elements = order.map((el: any) => ({
        type: el.type,
        props: el.props,
        col: el.col,
        id: el.id,
      }));
    },
    saveEdit() {
      useViewsStore().save();
      this.editing = false;
    },
    cancelEdit() {
      useViewsStore().load();
      this.editing = false;
    },
    startEdit() {
      this.editing = true;
    },
    addVisualization() {
      if (!this.view) return;
      useViewsStore().addVisualization({ view_id: this.view.id, type: 'top_apps' });
    },
    async onTypeChange(id: number, type: string) {
      if (!this.view) return;
      if (type === 'custom_watcher_view' || type === 'custom_html_module') {
        this.wizardTargetElId = id;
        this.wizardEntryType = type === 'custom_watcher_view' ? 'watcher' : 'html';
        return;
      }
      await useViewsStore().editView({ view_id: this.view.id, el_id: id, type, props: {} });
    },
    async onWizardCreated(payload: { type: string; props: Record<string, unknown> }) {
      if (!this.view || this.wizardTargetElId === null) {
        this.wizardEntryType = null;
        return;
      }
      await useViewsStore().editView({
        view_id: this.view.id,
        el_id: this.wizardTargetElId,
        type: payload.type,
        props: payload.props,
      });
      this.wizardEntryType = null;
      this.wizardTargetElId = null;
    },
    async onRemove(id: number) {
      if (!this.view) return;
      await useViewsStore().removeVisualization({ view_id: this.view.id, el_id: id });
    },
    onVisibilityChange(idx: number, visibile: boolean) {
      if (visibile) {
        if (idx in this.nascostiPerAssenzaDati) this.$delete(this.nascostiPerAssenzaDati, idx);
      } else {
        this.$set(this.nascostiPerAssenzaDati, idx, true);
      }
    },
  },
};
</script>

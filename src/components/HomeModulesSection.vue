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
    div.modules-grid-item(
      v-for="el in renderOrder"
      :key="el.__idx"
      :data-idx="el.__idx"
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
import { TimePeriod } from '~/util/timeperiod';
import { getHomeClient } from '~/util/awclient';

// Moduli a piena larghezza (span = tutte le colonne) — nessun tipo la
// usa più oggi (i due che la popolavano, sunburst_clock e vis_timeline,
// sono stati rimossi — vedi BLUEPRINT.md sezione 16), lasciata pronta
// per un futuro modulo "large".
const LARGE_TYPES: string[] = [];
// Moduli larghi 2 colonne (es. Categorie) — vedi BLUEPRINT.md sezione
// 17/18 per la storia completa di come si è arrivati a un vero motore
// di impaccamento invece di una banda a parte.
const DOUBLE_TYPES = ['top_categories'];
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
  data() {
    return {
      editing: false,
      columnCount: 4,
      columnWidthPx: 0,
      activityStore: useActivityStore(),
      settingsStore: useSettingsStore(),
      bucketsStore: useBucketsStore(),
      refreshInterval: null as ReturnType<typeof setInterval> | null,
      lastBucketsSignature: null as string | null,
      // Indici (__idx) dei moduli che si sono nascosti da soli per
      // mancanza di dati (Excel/VPN/VoiSpeed — vedi il commento su
      // `visibile` in SelectableVisualization.vue) — popolato via
      // l'evento onVisibilityChange emesso da ciascuna card. Oggetto
      // reattivo (non un Set: Vue 2 non traccia le mutazioni di
      // Set/Map) aggiornato con $set/$delete.
      nascostiPerAssenzaDati: {} as Record<number, boolean>,
      // Altezza reale misurata di ciascuna card (via ResizeObserver,
      // vedi observeItems) — chiave __idx. Finché una card non è stata
      // misurata almeno una volta si usa FALLBACK_ITEM_HEIGHT.
      itemHeights: {} as Record<number, number>,
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
    // Same fallback Activity.vue's own `_date` computed used — the
    // prop default can't depend on "today" at mount time (would freeze
    // to whatever day the app happened to load on), so it's resolved
    // fresh here instead.
    date(): string {
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
    // l'ordine attualmente da mostrare — vedi computePacking.
    packed(): PackedLayout {
      return this.computePacking(this.renderOrder);
    },
  },
  watch: {
    date() {
      this.loadActivityData();
    },
    host() {
      this.loadActivityData();
    },
  },
  async mounted() {
    this.ro = new ResizeObserver((entries: ResizeObserverEntry[]) => {
      for (const entry of entries) {
        const idxAttr = (entry.target as HTMLElement).dataset.idx;
        if (idxAttr === undefined) continue;
        const idx = parseInt(idxAttr, 10);
        const h = entry.contentRect.height;
        if (Math.abs((this.itemHeights[idx] || 0) - h) > 0.5) {
          this.$set(this.itemHeights, idx, h);
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
    // Cancels pending requests and resets the store — same cleanup
    // Activity.vue's own beforeDestroy used to do when leaving the page.
    this.activityStore.reset();
  },
  methods: {
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
    // Motore di impaccamento: scorre `order` e per ciascun elemento
    // trova la posizione (colonna di partenza) che minimizza l'altezza
    // a cui finirebbe — esattamente l'euristica "colonna più corta"
    // già usata dal vecchio sistema round-robin, generalizzata per
    // supportare elementi larghi più di 1 colonna (che occupano lo
    // start più corto tra quelli disponibili per il loro span,
    // considerando il MASSIMO delle colonne coinvolte, non la somma).
    computePacking(order: any[]): PackedLayout {
      const colHeights = new Array(this.columnCount).fill(0);
      const layout: Record<number, { x: number; y: number; width: number; height: number }> = {};
      const colW = this.columnWidthPx > 0 ? this.columnWidthPx : COLUMN_MIN_WIDTH;
      for (const el of order) {
        const span = this.isLarge(el.type)
          ? this.columnCount
          : this.isDouble(el.type)
          ? Math.min(2, this.columnCount)
          : 1;
        const h = this.itemHeights[el.__idx] || FALLBACK_ITEM_HEIGHT;
        let bestStart = 0;
        let bestTop = Infinity;
        for (let start = 0; start <= this.columnCount - span; start++) {
          let top = 0;
          for (let c = start; c < start + span; c++) top = Math.max(top, colHeights[c]);
          if (top < bestTop) {
            bestTop = top;
            bestStart = start;
          }
        }
        const x = bestStart * (colW + COLUMN_GAP);
        const width = span * colW + (span - 1) * COLUMN_GAP;
        layout[el.__idx] = { x, y: bestTop, width, height: h };
        for (let c = bestStart; c < bestStart + span; c++) colHeights[c] = bestTop + h + COLUMN_GAP;
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
      const targetCol = Math.min(this.columnCount - 1, Math.max(0, Math.floor(cursorX / colUnit)));

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
      if (this.dragging && this.dragging.idx === idx) {
        const d = this.dragging;
        return {
          position: 'absolute',
          left: d.cursorX + 'px',
          top: d.cursorY + 'px',
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
      const finalOrder = this.previewOrder;
      window.removeEventListener('mousemove', this.onDragMouseMove);
      window.removeEventListener('mouseup', this.onDragMouseUp);
      this.dragging = null;
      this.commitOrder(finalOrder);
    },
    commitOrder(order: any[]) {
      if (!this.view) return;
      this.elements = order.map((el: any) => ({ type: el.type, props: el.props }));
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
      let props = {};
      if (type === 'custom_vis') {
        const visname = prompt(this.$t('activityView.promptWatcher') as string, 'aw-watcher-');
        if (!visname) return;
        const title = prompt(this.$t('activityView.promptTitle') as string);
        if (!title) return;
        props = { visname, title };
      }
      await useViewsStore().editView({ view_id: this.view.id, el_id: id, type, props });
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

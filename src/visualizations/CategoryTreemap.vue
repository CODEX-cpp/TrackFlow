
<template lang="pug">
div.cat-treemap-root
  div.cat-treemap-wrap
    // v-if guarda haDatiGrezzi (solo "ci sono app con del tempo",
    // INDIPENDENTE dalla larghezza misurata), NON categoriesWithApps
    // (che invece dipende da this.width, per il filtro ad area — vedi
    // sotto) — bug reale trovato dal vivo, riapparso due volte: se il
    // v-if dipende da qualcosa che a sua volta serve la larghezza, e la
    // larghezza si misura SOLO quando .cat-treemap esiste nel DOM, il
    // .cat-treemap non esiste mai finché non è già misurato — un
    // vicolo cieco che restava bloccato per sempre sullo stato vuoto
    // anche con app davvero categorizzate.
    div.cat-treemap-empty(v-if="!haDatiGrezzi") {{ $t('visualizations.categoryTreemap.empty') }}
    div.cat-treemap(v-else ref="treemapEl")
      div.cat-treemap-category(
        v-for="cat in categoryRects"
        :key="cat.key"
        :style="{ ...insetStyle(cat), borderColor: cat.color, backgroundColor: cat.bgColor }"
        :title="cat.name + ' — ' + cat.durationLabel"
      )
        div.cat-treemap-category-header(:style="{ color: cat.color }")
          span.cat-treemap-category-name {{ cat.name }}
          span.cat-treemap-category-total {{ cat.durationLabel }}
        div.cat-treemap-category-body
          div.cat-treemap-app(
            v-for="app in cat.apps"
            :key="app.key"
            :style="{ ...insetStyle(app), borderColor: cat.color }"
            :title="app.displayName + ' — ' + app.durationLabel"
          )
            span.cat-treemap-app-name {{ app.displayName }}
            span.cat-treemap-app-duration {{ app.durationLabel }}
  // TEMP: dump testuale per verificare i numeri senza bisogno di
  // screenshot — FUORI da .cat-treemap-wrap apposta (quello ha altezza
  // fissa + overflow:hidden per il treemap stesso, altrimenti il dump
  // veniva tagliato via e restava invisibile — bug reale trovato dal
  // vivo). Nascosto (non rimosso, richiesta esplicita dell'utente: può
  // servire di nuovo in futuro) dietro SHOW_DEBUG_DUMP nello script —
  // basta rimettere quella costante a true per farlo ricomparire.
  div.cat-treemap-debug(v-if="showDebugDump && debugDump.length > 0")
    div(v-for="cat in debugDump" :key="'debug-' + cat.name")
      div {{ cat.name }}{{ cat.hidden ? ' *' : '' }}: {{ cat.percent }}%
      div(v-for="app in cat.apps" :key="'debug-' + cat.name + '-' + app.name" style="padding-left: 16px")
        | {{ app.name }}{{ app.hidden ? ' *' : '' }}: {{ app.percent }}%
</template>

<style lang="scss" scoped>
@import '../style/theme.css';

.cat-treemap-empty {
  font-size: var(--font-size-sm);
  color: var(--color-text-faint);
}

// Altezza proporzionale alla larghezza (aspect-ratio), NON un pixel
// fisso — a differenza degli altri moduli (che crescono in base al
// contenuto), un treemap impacca un'area DATA: deve conoscerla in
// anticipo per calcolare il layout, non può adattarsi al contenuto
// dopo. Richiesta esplicita dell'utente: un'altezza fissa in px non
// andava bene, si adatta invece alla larghezza reale del modulo (che
// varia da schermo a schermo) — HEIGHT_RATIO nello script deve restare
// coerente con questo stesso rapporto. overflow:hidden come rete di
// sicurezza — bug reale segnalato dall'utente: una categoria/app sotto
// l'area minima (vedi MIN_CATEGORY_AREA/MIN_APP_AREA più sotto, nello
// script) va accorpata in "Altro" PRIMA di calcolare il layout, non
// dopo — questo resta comunque come secondo livello di protezione
// contro qualunque overflow imprevisto.
.cat-treemap-wrap {
  aspect-ratio: 2.2 / 1;
  overflow: hidden;
}

// TEMP: solo per il dump di debug — rimuovere insieme al blocco nel
// template quando non serve più.
.cat-treemap-debug {
  margin-top: 10px;
  padding-top: 10px;
  border-top: 1px dashed var(--color-border);
  font-family: 'SFMono-Regular', Consolas, monospace;
  font-size: var(--font-size-xs);
  color: var(--color-text-dim);
}

.cat-treemap {
  position: relative;
  width: 100%;
  height: 100%;
}

.cat-treemap-category {
  position: absolute;
  box-sizing: border-box;
  border: 1.5px solid;
  border-radius: var(--radius-md);
  padding: 6px;
  overflow: hidden;
  transition: transform 0.25s ease, width 0.25s ease, height 0.25s ease;
}

.cat-treemap-category-header {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 6px;
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-semibold);
  padding: 0 2px 4px 2px;
  white-space: nowrap;
  overflow: hidden;
}

.cat-treemap-category-name {
  overflow: hidden;
  text-overflow: ellipsis;
}

.cat-treemap-category-total {
  flex-shrink: 0;
  opacity: 0.85;
}

.cat-treemap-category-body {
  position: relative;
  width: 100%;
  height: calc(100% - 20px);
}

.cat-treemap-app {
  position: absolute;
  box-sizing: border-box;
  border: 1px solid;
  border-radius: var(--radius-sm);
  background-color: var(--color-surface);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 2px;
  padding: 4px;
  overflow: hidden;
  text-align: center;
  transition: transform 0.25s ease, width 0.25s ease, height 0.25s ease;
}

.cat-treemap-app-name {
  font-size: var(--font-size-sm);
  color: var(--color-text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 100%;
}

.cat-treemap-app-duration {
  font-size: var(--font-size-xs);
  color: var(--color-text-faint);
  font-variant-numeric: tabular-nums;
}
</style>

<script lang="ts">
// Treemap a due livelli (categorie, poi app dentro ciascuna categoria) —
// richiesto esplicitamente dall'utente sul modello di un mockup
// (categorie come riquadri colorati proporzionali al tempo totale, app
// dentro come sotto-riquadri proporzionali al proprio tempo). Riceve gli
// stessi eventi "app" già filtrati dal genitore di aw-category-bar
// (top_apps_filtered — sempre il giorno corrente, mai un periodo più
// ampio, per richiesta esplicita) e fa da sé tutto il lavoro app→
// categoria→layout.
import { useAppCategoriesStore } from '~/stores/appCategories';
import { formatHoursMinutes } from '~/util/projectTime';
import { displayNameForApp } from '~/util/appNames';
import { squarify, TreemapInput, TreemapRect } from '~/util/treemap';

// Richiesta esplicita dell'utente: al massimo 3 categorie individuali
// (e, dentro ciascuna, al massimo 3 app individuali) — il resto (se
// presente) si accorpa in un'unica voce "Altro" con la percentuale
// sommata, non sparisce silenziosamente.
const MAX_CATEGORIES = 3;
const MAX_APPS_PER_CATEGORY = 3;

// Nasconde il dump testuale di debug sotto il treemap — richiesta
// esplicita dell'utente: non va rimosso (può tornare utile per
// verificare i numeri senza screenshot), solo nascosto. Rimettere a
// true per farlo ricomparire.
const SHOW_DEBUG_DUMP = false;

// Deve combaciare ESATTAMENTE col rapporto "aspect-ratio: 2.2 / 1" di
// .cat-treemap-wrap nel CSS — serve per calcolare in JS l'altezza reale
// che il browser darà al contenitore.
const HEIGHT_RATIO = 1 / 2.2;

// Sotto queste dimensioni (pixel VERI) un riquadro non ha spazio per il
// proprio contenuto in modo leggibile.
const MIN_CATEGORY_WIDTH = 90;
const MIN_CATEGORY_HEIGHT = 55;
const MIN_APP_WIDTH = 46;
const MIN_APP_HEIGHT = 30;

// Richiesta esplicita dell'utente: in generale, una voce sotto questa
// percentuale del totale (categoria o app) va nascosta a prescindere,
// non solo quando lo spazio reale risulta troppo piccolo — una soglia
// semplice e prevedibile, non solo "se per caso il layout la stringe
// troppo". Applicata PRIMA di provare il layout vero, come tetto al
// numero di candidati; il controllo sulle dimensioni reali (sotto)
// resta comunque come rete di sicurezza per i casi limite. Vale anche
// per il riquadro "Altro" stesso: se il resto accorpato pesa comunque
// meno di questa soglia sul totale, non merita un riquadro tutto suo e
// sparisce anch'esso, invece di occupare comunque spazio per una
// percentuale minuscola — richiesta esplicita dell'utente dopo aver
// visto un "Altro" all'1% ancora disegnato.
const MIN_PERCENT = 0.15;

// Prova a disporre le prime `maxCount` voci (più "Altro" per il resto,
// se presente) nell'area data, e verifica le dimensioni VERE che
// squarify assegna a ciascuna — non una stima per percentuale/area: un
// primo tentativo con una soglia di area STIMATA lasciava comunque
// passare voci che, disposte per davvero accanto a sorelle molto più
// grandi, finivano su una striscia sottilissima nonostante l'area
// "sulla carta" sembrasse sufficiente (bug reale, confermato dal vivo:
// un'app All'1% con area stimata sopra soglia risultava comunque un
// filo illeggibile). Se anche solo UNA voce reale (non "Altro") risulta
// troppo piccola, il conteggio si riduce di uno e si riprova daccapo —
// finché tutte le voci reali rientrano, o non ne resta che una sola
// (che a quel punto occupa l'intera area, sempre abbastanza grande).
function capConLayoutReale<T>(
  ordinati: T[],
  valoreDi: (item: T) => number,
  keyDi: (item: T) => string,
  maxCount: number,
  canvasWidth: number,
  canvasHeight: number,
  minWidth: number,
  minHeight: number,
  altroKey: string
): { layout: TreemapRect[]; usedCount: number; restoValore: number } {
  const totale = ordinati.reduce((a, item) => a + valoreDi(item), 0);
  // `ordinati` è già ordinato per valore decrescente (vedi
  // categorieGrezze/appsOrdinate) — basta contare quante voci in testa
  // superano la soglia percentuale, il resto è comunque sotto.
  let maxByPercent = 0;
  if (totale > 0) {
    while (maxByPercent < ordinati.length && valoreDi(ordinati[maxByPercent]) / totale >= MIN_PERCENT) {
      maxByPercent++;
    }
  }
  maxCount = Math.min(maxCount, maxByPercent);
  for (let count = Math.min(maxCount, ordinati.length); count >= 0; count--) {
    const principali = ordinati.slice(0, count);
    const restoValore = ordinati.slice(count).reduce((a, item) => a + valoreDi(item), 0);
    const inputItems: TreemapInput[] = principali.map(p => ({ key: keyDi(p), value: valoreDi(p) }));
    // "Altro" stesso segue la stessa soglia percentuale delle voci
    // singole: se il resto accorpato è comunque troppo poco rispetto al
    // totale, non gli si dedica un riquadro — sparisce e basta, invece
    // di occupare comunque spazio per una percentuale minuscola.
    if (restoValore > 0 && totale > 0 && restoValore / totale >= MIN_PERCENT) {
      inputItems.push({ key: altroKey, value: restoValore });
    }
    if (inputItems.length === 0) break;
    const layout = squarify(inputItems, 0, 0, canvasWidth, canvasHeight);
    const realiOk = layout
      .filter(r => r.key !== altroKey)
      .every(r => r.width >= minWidth && r.height >= minHeight);
    if (realiOk || count === 0) {
      return { layout, usedCount: count, restoValore };
    }
  }
  return { layout: [], usedCount: 0, restoValore: totale };
}

// Spazio visivo tra un riquadro e l'altro (sia tra categorie che tra
// app dentro una categoria) — richiesta esplicita dell'utente: prima si
// toccavano esattamente, senza nessuna separazione. Applicato "verso
// dentro" (ogni riquadro si restringe di metà GAP per lato) invece che
// spostando le posizioni calcolate da squarify — così la matematica del
// treemap resta quella verificata ed esatta, lo spazio è solo un
// dettaglio del disegno finale.
const GAP = 6;

// Deve combaciare ESATTAMENTE con `padding: 6px` e `border: 1.5px
// solid` di .cat-treemap-category nel CSS (box-sizing: border-box, quindi
// entrambi "mangiano" spazio dal riquadro invece di aggiungersene) —
// altrimenti l'area calcolata qui per le app è più grande di quella
// DAVVERO disponibile nel DOM, e le app dell'ultima riga/colonna
// sconfinano oltre il bordo della categoria: overflow:hidden le taglia
// via silenziosamente, facendole sembrare "senza spazio" sul lato
// destro/basso mentre in alto/a sinistra il gap è visibile (bug reale
// segnalato dall'utente via screenshot).
const CATEGORY_PADDING = 6;
const CATEGORY_BORDER = 1.5;
const CATEGORY_INSET = 2 * (CATEGORY_PADDING + CATEGORY_BORDER);

interface CategoryRect extends TreemapRect {
  name: string;
  color: string;
  bgColor: string;
  // Durata vera (secondi), NON `value` — dopo squarify `value` è
  // riscalato in area px² (vedi treemap.ts), non più la durata
  // originale: usare `value` qui avrebbe fatto leggere aree al posto
  // di durate a chiunque consumi questo campo (bug reale, dump di
  // debug che mostrava 0% ovunque perché leggeva un `total`/`duration`
  // che non esisteva affatto sull'oggetto restituito da squarify).
  total: number;
  durationLabel: string;
  apps: (TreemapRect & { name: string; displayName: string; durationLabel: string; duration: number })[];
}

// Percentuale arrotondata di `value` su `total` — helper condiviso da
// durationLabel (percentuale accanto al minutaggio, richiesta esplicita
// dell'utente) e dal dump di debug.
function pct(value: number, total: number): number {
  return total > 0 ? Math.round((value / total) * 100) : 0;
}

interface DebugApp {
  name: string;
  duration: number;
  percent: number;
  hidden: boolean;
}

interface DebugCategory {
  name: string;
  total: number;
  percent: number;
  hidden: boolean;
  apps: DebugApp[];
}

export default {
  name: 'aw-category-treemap',
  props: {
    apps: { type: Array, default: () => [] },
  },
  data() {
    return {
      width: 0,
      ro: null as ResizeObserver | null,
    };
  },
  mounted() {
    this.ro = new ResizeObserver(entries => {
      for (const entry of entries) {
        this.width = entry.contentRect.width;
      }
    });
    if (this.$refs.treemapEl) (this.ro as ResizeObserver).observe(this.$refs.treemapEl as HTMLElement);
  },
  beforeDestroy() {
    if (this.ro) this.ro.disconnect();
  },
  watch: {
    // `apps` arriva dal genitore in modo asincrono (HomeModulesSection
    // carica activityStore dopo il mount) — se al momento del mount i
    // dati non erano ancora arrivati, .cat-treemap non esisteva ancora
    // nel DOM (v-if sullo stato vuoto) e l'observe() fatto una sola
    // volta in mounted() non l'ha mai visto. .observe() è comunque
    // sicuro da richiamare più volte sullo stesso nodo (nessuna
    // osservazione duplicata), quindi riprovare ad ogni cambio dati non
    // ha effetti collaterali quando il nodo era già osservato.
    haDatiGrezzi() {
      this.$nextTick(() => {
        if (this.$refs.treemapEl && this.ro) this.ro.observe(this.$refs.treemapEl as HTMLElement);
      });
    },
  },
  computed: {
    categorieStore() {
      return useAppCategoriesStore();
    },
    showDebugDump(): boolean {
      return SHOW_DEBUG_DUMP;
    },
    // Indipendente da this.width apposta — vedi il commento sul v-if
    // nel template per il perché. Non guarda se le app sono davvero
    // CATEGORIZZATE (quello lo decide già il genitore,
    // SelectableVisualization.vue, nascondendo l'intero modulo se non
    // c'è nulla di categorizzato) — solo se c'è qualcosa da mostrare
    // una volta che la larghezza sarà nota.
    haDatiGrezzi(): boolean {
      return (this.apps as any[]).some(e => e.data && e.data.app && (e.duration || 0) > 0);
    },
    // Solo il raggruppamento grezzo app→categoria (nessun taglio,
    // nessun layout) — la base condivisa da cui categoryRects sotto
    // ricava sia le categorie mostrate sia, per ciascuna, le sue app.
    // Indipendente da this.width apposta (i dati grezzi non dipendono
    // dalla larghezza — solo il taglio/layout ne ha bisogno).
    categorieGrezze(): { name: string; total: number; perApp: Map<string, number> }[] {
      const perCategoria = new Map<string, Map<string, number>>();
      for (const e of this.apps as any[]) {
        const app = e.data && e.data.app;
        if (!app) continue;
        const durata = e.duration || 0;
        if (durata <= 0) continue;
        const categoria =
          this.categorieStore.categoryForApp(app) ||
          (this.$t('visualizations.categoryBar.uncategorized') as string);
        if (!perCategoria.has(categoria)) perCategoria.set(categoria, new Map());
        const perApp = perCategoria.get(categoria) as Map<string, number>;
        perApp.set(app, (perApp.get(app) || 0) + durata);
      }
      const risultato: { name: string; total: number; perApp: Map<string, number> }[] = [];
      for (const [nome, perApp] of perCategoria) {
        const total = [...perApp.values()].reduce((a, b) => a + b, 0);
        if (total > 0) risultato.push({ name: nome, total, perApp });
      }
      risultato.sort((a, b) => b.total - a.total);
      return risultato;
    },
    // Layout finale (posizioni/dimensioni in px) e dump di debug,
    // costruiti insieme da computeLayout() in un unico passaggio — così
    // il dump può elencare ANCHE le categorie/app nascoste (marcate con
    // * nel template) usando esattamente le stesse decisioni di
    // taglio del layout vero, senza ricalcolarle una seconda volta in
    // modo scollegato (e quindi potenzialmente incoerente).
    categoryRects(): CategoryRect[] {
      return this.computeLayout().rects;
    },
    debugDump(): DebugCategory[] {
      return this.computeLayout().debug;
    },
  },
  methods: {
    // Vedi il commento sul computed categoryRects/debugDump sopra: le
    // due viste (layout vero, dump completo con voci nascoste) nascono
    // dalle stesse identiche decisioni di taglio, calcolate una sola
    // volta qui.
    computeLayout(): { rects: CategoryRect[]; debug: DebugCategory[] } {
      const categorieGrezze = this.categorieGrezze as { name: string; total: number; perApp: Map<string, number> }[];
      const grandTotale = categorieGrezze.reduce((a, c) => a + c.total, 0);

      // Larghezza non ancora misurata (primo giro) o nessun dato: nessun
      // layout possibile, ma il dump deve comunque poter elencare i dati
      // grezzi (tutti marcati nascosti, dato che non c'è ancora nulla di
      // disegnato).
      if (this.width <= 0 || grandTotale <= 0) {
        const debug: DebugCategory[] = categorieGrezze.map(cat => ({
          name: cat.name,
          total: cat.total,
          percent: pct(cat.total, grandTotale),
          hidden: true,
          apps: [...cat.perApp.entries()].map(([name, duration]) => ({
            name,
            duration,
            percent: pct(duration, cat.total),
            hidden: true,
          })),
        }));
        return { rects: [], debug };
      }

      const height = this.width * HEIGHT_RATIO;
      const altroLabel = this.$t('visualizations.categoryBar.other') as string;

      const {
        layout: catLayout,
        usedCount,
        restoValore: restoCategorie,
      } = capConLayoutReale(
        categorieGrezze,
        c => c.total,
        c => c.name,
        MAX_CATEGORIES,
        this.width,
        height,
        MIN_CATEGORY_WIDTH,
        MIN_CATEGORY_HEIGHT,
        altroLabel
      );
      // Le categorie DAVVERO scelte da capConLayoutReale — usate sotto
      // per ritrovare i dati sorgente (apps/perApp) di ogni rect reale,
      // e per distinguere un rect "Altro" sintetico da una categoria
      // che l'utente ha chiamato per davvero "Altro" (caso limite raro
      // ma possibile).
      const categoriePrincipali = categorieGrezze.slice(0, usedCount);

      const rects: CategoryRect[] = [];
      const debug: DebugCategory[] = [];

      for (const rect of catLayout) {
        const sorgente = categoriePrincipali.find(c => c.name === rect.key);
        const isAltroSintetico = !sorgente && rect.key === altroLabel;
        const isUncategorized = !isAltroSintetico && sorgente!.name === (this.$t('visualizations.categoryBar.uncategorized') as string);
        const color = isUncategorized || isAltroSintetico ? 'var(--color-text-faint)' : this.categorieStore.colorForCategoryName(sorgente!.name);
        const bgColor = `color-mix(in srgb, ${color} 8%, var(--color-surface))`;

        if (isAltroSintetico) {
          const durationLabel = `${formatHoursMinutes(restoCategorie)} · ${pct(restoCategorie, grandTotale)}%`;
          rects.push({
            ...rect,
            name: altroLabel,
            color,
            bgColor,
            total: restoCategorie,
            durationLabel,
            apps: [
              {
                key: altroLabel,
                x: 0,
                y: 0,
                width: Math.max(0, rect.width - GAP - CATEGORY_INSET),
                height: Math.max(0, rect.height - GAP - CATEGORY_INSET - 20),
                name: altroLabel,
                displayName: altroLabel,
                durationLabel,
                duration: restoCategorie,
              },
            ],
          });
          continue;
        }

        // Area REALMENTE disponibile per le app dentro la categoria —
        // rect.width/height sono le dimensioni PRIMA dello spazio tra i
        // riquadri (insetStyle le riduce di GAP al momento del
        // rendering, vedi sopra); usare le dimensioni piene qui
        // calcolava posizioni per uno spazio più grande di quello
        // davvero renderizzato, facendo sforare leggermente le app
        // vicine al bordo destro/basso oltre il riquadro categoria
        // (che ha overflow:hidden) — bug reale segnalato dall'utente:
        // il bordo in quel punto spariva, tagliato via insieme
        // all'eccesso. calc(100% - 20px) nel CSS parte già dalla
        // larghezza/altezza ridotta (quella vera, post-inset) — questi
        // due valori devono restare sempre coerenti con quello.
        const innerWidth = Math.max(0, rect.width - GAP - CATEGORY_INSET);
        const innerHeight = Math.max(0, rect.height - GAP - CATEGORY_INSET);
        const bodyHeight = Math.max(0, innerHeight - 20);
        const appsOrdinate = [...sorgente!.perApp.entries()]
          .map(([name, duration]) => ({ name, duration }))
          .sort((a, b) => b.duration - a.duration);
        const {
          layout: appLayout,
          usedCount: appsUsedCount,
          restoValore: restoApp,
        } = capConLayoutReale(
          appsOrdinate,
          a => a.duration,
          a => a.name,
          MAX_APPS_PER_CATEGORY,
          innerWidth,
          bodyHeight,
          MIN_APP_WIDTH,
          MIN_APP_HEIGHT,
          altroLabel
        );
        const apps = appLayout.map(a => {
          const src = appsOrdinate.find(x => x.name === a.key);
          const durata = src ? src.duration : restoApp;
          return {
            ...a,
            name: a.key,
            displayName: src ? displayNameForApp(src.name) : altroLabel,
            durationLabel: `${formatHoursMinutes(durata)} · ${pct(durata, sorgente!.total)}%`,
            duration: durata,
          };
        });

        rects.push({
          ...rect,
          name: sorgente.name,
          color,
          bgColor,
          total: sorgente.total,
          durationLabel: `${formatHoursMinutes(sorgente.total)} · ${pct(sorgente.total, grandTotale)}%`,
          apps,
        });

        debug.push({
          name: sorgente.name,
          total: sorgente.total,
          percent: pct(sorgente.total, grandTotale),
          hidden: false,
          apps: appsOrdinate.map((a, i) => ({
            name: a.name,
            duration: a.duration,
            percent: pct(a.duration, sorgente!.total),
            hidden: i >= appsUsedCount,
          })),
        });
      }

      // Categorie accorpate in "Altro" (o del tutto scartate, se anche
      // "Altro" era sotto soglia) — mai mostrate come riquadro
      // individuale, quindi tutte le loro app sono per definizione
      // nascoste (non compaiono da nessuna parte nel disegno, nemmeno
      // dentro un "Altro").
      for (const cat of categorieGrezze.slice(usedCount)) {
        debug.push({
          name: cat.name,
          total: cat.total,
          percent: pct(cat.total, grandTotale),
          hidden: true,
          apps: [...cat.perApp.entries()].map(([name, duration]) => ({
            name,
            duration,
            percent: pct(duration, cat.total),
            hidden: true,
          })),
        });
      }

      return { rects, debug };
    },
    // Restringe un rettangolo di GAP/2 per lato — vedi il commento su
    // GAP più sopra per il perché "verso dentro" invece di spostare le
    // posizioni. Usato sia per le categorie che per le app dentro
    // ciascuna, stesso identico spazio in entrambi i casi.
    insetStyle(rect: { x: number; y: number; width: number; height: number }) {
      const w = Math.max(0, rect.width - GAP);
      const h = Math.max(0, rect.height - GAP);
      return {
        transform: `translate(${rect.x + GAP / 2}px, ${rect.y + GAP / 2}px)`,
        width: w + 'px',
        height: h + 'px',
      };
    },
  },
};
</script>

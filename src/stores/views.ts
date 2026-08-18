import { defineStore } from 'pinia';
import { useSettingsStore } from './settings';

interface IElement {
  type: string;
  size?: number;
  props?: Record<string, unknown>;
}

export interface View {
  id: string;
  name: string;
  elements: IElement[];
}

// Only "summary" survives — HomeModulesSection.vue only ever reads
// views[0], and its own edit UI ("+ Aggiungi modulo") already offers
// every visualization type (including the ones the old Window/Browser/
// Editor tabs used to pre-populate — top_domains, top_editor_files,
// etc.) directly from this one view. Those three extra tabs, and the
// whole concept of switching between named views, only existed via the
// now-removed Activity.vue/ActivityView.vue Bootstrap page — explicit
// request to not carry that over.
// Ordine e selezione di default al primo avvio (nessun'impostazione
// salvata ancora) — richiesta esplicita: Top Applications, Top Window
// Titles, Top Editor Projects, Top Editor Files, Categorie, in
// quest'ordine (Uso Claude e Top Clienti VPN tolti, richiesta
// esplicita). L'utente può comunque personalizzare da "Modifica
// moduli" in qualunque momento — questo è solo il punto di partenza,
// non un vincolo permanente.
const desktopViews: View[] = [
  {
    id: 'summary',
    name: 'Summary',
    elements: [
      { type: 'top_apps', size: 3 },
      { type: 'top_titles', size: 3 },
      { type: 'top_editor_projects', size: 3 },
      { type: 'top_editor_files', size: 3 },
      { type: 'top_categories', size: 3 },
    ],
  },
];

// Nessun target Android reale in questo progetto (app desktop Windows
// pura) — mirror di desktopViews invece di un proprio elenco, dopo che i
// tipi category_tree/category_sunburst/top_categories/timeline_barchart
// che usava sono stati rimossi col resto del vecchio sistema di
// categorie (2026-08-12, richiesta esplicita dell'utente).
export const androidViews: View[] = desktopViews;

export const defaultViews = !process.env.VUE_APP_ON_ANDROID ? desktopViews : androidViews;

interface State {
  views: View[];
}

export const useViewsStore = defineStore('views', {
  state: (): State => ({
    views: [],
  }),
  actions: {
    async load() {
      const settingsStore = useSettingsStore();
      await settingsStore.ensureLoaded();
      const views = settingsStore.views;
      this.loadViews(views);
    },
    async save() {
      const settingsStore = useSettingsStore();
      settingsStore.update({ views: this.views });
      await this.load();
    },
    loadViews(views: View[]) {
      // Deep copy — bug reale confermato: senza, viewsStore.views
      // punta agli STESSI oggetti di settingsStore.views (per
      // riferimento, non per valore), quindi editView()/setElements()/
      // addVisualization()/removeVisualization() mutavano in-place
      // anche i dati di settingsStore. Il tasto "Annulla"
      // (HomeModulesSection.vue's cancelEdit()) richiama load() per
      // tornare all'ultimo stato salvato, ma quello stato era già
      // stato modificato a sua insaputa — Annulla non annullava
      // davvero niente.
      this.$patch({ views: JSON.parse(JSON.stringify(views)) });
      console.log('Loaded views:', this.views);
    },
    clearViews(this: State) {
      this.views = [];
    },
    setElements(this: State, { view_id, elements }: { view_id: string; elements: IElement[] }) {
      this.views.find(v => v.id == view_id).elements = elements;
    },
    editView(
      this: State,
      {
        view_id,
        el_id,
        type,
        props,
      }: { view_id: string; el_id: number; type: string; props: Record<string, unknown> }
    ) {
      console.log(view_id, el_id, type, props);
      console.log(this.views);
      const element = this.views.find(v => v.id == view_id).elements[el_id];
      element.type = type;
      element.props = props;
    },
    addVisualization(this: State, { view_id, type }) {
      this.views.find(v => v.id == view_id).elements.push({ type: type });
    },
    // Come addVisualization, ma con props già pronti — usato dalla
    // creazione di un watcher personalizzato per aggiungere subito il
    // suo modulo in Home (bucketId/title/gridWidth), senza dover passare
    // dal flusso "+ Aggiungi modulo" → scelta tipo a vuoto.
    addVisualizationWithProps(
      this: State,
      { view_id, type, props }: { view_id: string; type: string; props: Record<string, unknown> }
    ) {
      this.views.find(v => v.id == view_id).elements.push({ type, props });
    },
    removeVisualization(this: State, { view_id, el_id }) {
      this.views.find(v => v.id == view_id).elements.splice(el_id, 1);
    },
  },
});

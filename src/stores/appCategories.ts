import { defineStore } from 'pinia';
import { useSettingsStore } from './settings';
import { colorVarForName } from '~/util/hashColor';

// Nuovo sistema di categorie, deliberatamente semplice: una categoria è
// solo un nome + un elenco di app assegnate (nessuna regola regex, a
// differenza del vecchio sistema ereditato da ActivityWatch upstream,
// rimosso del tutto su richiesta esplicita dell'utente 2026-08-12 — vedi
// BLUEPRINT.md sezione 3). Stessa identica forma già scritta lato Rust
// dalla categorizzazione automatica AI (`src-tauri/src/
// categorization.rs`), sotto la stessa chiave impostazioni
// "appCategories" — questo store è solo un'interfaccia CRUD sopra quel
// dato condiviso, non una copia separata.
export interface AppCategory {
  name: string;
  apps: string[];
  // Colore scelto a mano dal popup "Modifica categoria" (Impostazioni →
  // Categorizzazione, vedi CATEGORY_COLOR_PALETTE in util/hashColor.ts)
  // — assente per ogni categoria mai modificata così, nel qual caso
  // colorForCategoryName() sotto ricade sul colore automatico calcolato
  // da colorVarForName. Opzionale (non un default vuoto) apposta: così
  // le categorie salvate prima di questa funzione restano valide senza
  // bisogno di alcuna migrazione.
  color?: string;
}

export const useAppCategoriesStore = defineStore('appCategories', {
  state: () => ({}),
  getters: {
    categories(): AppCategory[] {
      return useSettingsStore().appCategories;
    },
    // Nome della categoria a cui è assegnata un'app, o null se non
    // ancora categorizzata — confronto case-insensitive, stessa
    // convenzione già usata lato Rust (i nomi app sono sempre i nomi
    // grezzi dell'exe, minuscoli).
    categoryForApp() {
      return (app: string): string | null => {
        const cat = this.categories.find(c =>
          c.apps.some(a => a.toLowerCase() === app.toLowerCase())
        );
        return cat ? cat.name : null;
      };
    },
    // Colore da usare per QUALUNQUE visualizzazione basata su categoria
    // (treemap, "Flusso di lavoro", barra Categorie) — unico punto che
    // decide tra il colore scelto a mano (popup "Modifica categoria") e
    // quello automatico calcolato dal nome (colorVarForName, la stessa
    // funzione già usata per app/domini/client nella Timeline). Non
    // richiede che la categoria esista davvero: una categoria mai
    // creata (es. "Non categorizzato", che non è una voce reale in
    // `categories`) ricade comunque sul colore automatico.
    colorForCategoryName() {
      return (name: string): string => {
        const cat = this.categories.find(c => c.name === name);
        return (cat && cat.color) || colorVarForName(name);
      };
    },
  },
  actions: {
    async createCategory(name: string) {
      const trimmed = name.trim();
      if (!trimmed) return;
      const settingsStore = useSettingsStore();
      if (settingsStore.appCategories.some(c => c.name.toLowerCase() === trimmed.toLowerCase())) {
        return;
      }
      const updated = [...settingsStore.appCategories, { name: trimmed, apps: [] }];
      await settingsStore.update({ appCategories: updated });
    },
    // Elimina la categoria — le app che conteneva tornano semplicemente
    // "non categorizzate" (non sono entità proprie, solo un'etichetta),
    // non vengono cancellati dati di tracciamento reali.
    async deleteCategory(name: string) {
      const settingsStore = useSettingsStore();
      const updated = settingsStore.appCategories.filter(c => c.name !== name);
      await settingsStore.update({ appCategories: updated });
    },
    // Un solo metodo per assegnare, riassegnare o togliere la categoria
    // di un'app (categoryName = null) — toglie prima l'app da qualunque
    // categoria la contenga già, poi la aggiunge a quella nuova se
    // specificata. Unico punto di scrittura per questa relazione,
    // niente casi speciali diversi per "assegna per la prima volta" vs
    // "sposta".
    async assignApp(app: string, categoryName: string | null) {
      const settingsStore = useSettingsStore();
      const updated = settingsStore.appCategories.map(c => ({
        ...c,
        apps: c.apps.filter(a => a.toLowerCase() !== app.toLowerCase()),
      }));
      if (categoryName) {
        const target = updated.find(c => c.name === categoryName);
        if (target) target.apps.push(app);
      }
      await settingsStore.update({ appCategories: updated });
    },
    // `color: null` toglie la scelta manuale (torna al colore automatico
    // — vedi colorForCategoryName sopra), non un colore "nullo" vero e
    // proprio.
    async setCategoryColor(name: string, color: string | null) {
      const settingsStore = useSettingsStore();
      const updated = settingsStore.appCategories.map(c => {
        if (c.name !== name) return c;
        if (color === null) {
          const { color: _rimosso, ...senzaColore } = c;
          return senzaColore;
        }
        return { ...c, color };
      });
      await settingsStore.update({ appCategories: updated });
    },
  },
});

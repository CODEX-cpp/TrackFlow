import { defineStore } from 'pinia';
import { useSettingsStore } from './settings';

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
  },
});

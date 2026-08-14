import { defineStore } from 'pinia';
import { useSettingsStore } from './settings';

// Un "progetto" qui è solo IDENTITÀ e METADATI (nome, budget ore,
// se è chiuso/archiviato) — NON contiene le sessioni cronometrate
// vere, quelle vivono come eventi nel bucket ActivityWatch
// "aw-stopwatch", collegate tramite project.id (non tramite il
// nome, così rinominare un progetto non "perde" le sessioni
// passate).
export interface Project {
  id: string;
  name: string;
  // null = nessun budget impostato per questo progetto
  budgetHours: number | null;
  // Data prevista di fine lavori (YYYY-MM-DD), null se non impostata
  targetDate: string | null;
  closed: boolean;
  // Data ISO di quando è stato chiuso, null se non è mai stato chiuso
  closedDate: string | null;
}

interface State {
  projects: Project[];
  // In-memory signal (never saved to/loaded from settingsStore): the
  // Topbar sets it when you pick a search suggestion while on the
  // Projects page, Progetti.vue watches it to open that project's
  // detail popup, then clears it. Only exists to let two sibling
  // components talk to each other without an event bus.
  requestedDetailProjectId: string | null;
}

function generaId(): string {
  // Semplice id univoco: timestamp + un pezzo casuale. Non serve
  // nulla di più robusto, è solo una chiave interna, l'utente non
  // la vede mai.
  return Date.now().toString(36) + Math.random().toString(36).slice(2, 8);
}

export const useProjectsStore = defineStore('projects', {
  state: (): State => ({
    projects: [],
    requestedDetailProjectId: null,
  }),
  getters: {
    getProjectById: (state: State) => (id: string) => state.projects.find(p => p.id === id),
    activeProjects: (state: State) => state.projects.filter(p => !p.closed),
    closedProjects: (state: State) => state.projects.filter(p => p.closed),
  },
  actions: {
    async load() {
      const settingsStore = useSettingsStore();
      await settingsStore.ensureLoaded();
      this.$patch({ projects: settingsStore.projects });
    },
    async save() {
      const settingsStore = useSettingsStore();
      settingsStore.update({ projects: this.projects });
      await this.load();
    },
    addProject(this: State, { name }: { name: string }): Project {
      const nuovo: Project = {
        id: generaId(),
        name,
        budgetHours: null,
        targetDate: null,
        closed: false,
        closedDate: null,
      };
      this.projects.push(nuovo);
      return nuovo;
    },
    renameProject(this: State, { id, name }: { id: string; name: string }) {
      const p = this.projects.find(p => p.id === id);
      if (p) p.name = name;
    },
    setBudget(this: State, { id, budgetHours }: { id: string; budgetHours: number | null }) {
      const p = this.projects.find(p => p.id === id);
      if (p) p.budgetHours = budgetHours;
    },
    setTargetDate(this: State, { id, targetDate }: { id: string; targetDate: string | null }) {
      const p = this.projects.find(p => p.id === id);
      if (p) p.targetDate = targetDate;
    },
    closeProject(this: State, { id }: { id: string }) {
      const p = this.projects.find(p => p.id === id);
      if (p) {
        p.closed = true;
        p.closedDate = new Date().toISOString();
      }
    },
    reopenProject(this: State, { id }: { id: string }) {
      const p = this.projects.find(p => p.id === id);
      if (p) {
        p.closed = false;
        p.closedDate = null;
      }
    },
    removeProject(this: State, { id }: { id: string }) {
      const idx = this.projects.findIndex(p => p.id === id);
      if (idx !== -1) this.projects.splice(idx, 1);
    },
    requestProjectDetail(this: State, id: string) {
      this.requestedDetailProjectId = id;
    },
    clearRequestedProjectDetail(this: State) {
      this.requestedDetailProjectId = null;
    },
  },
});

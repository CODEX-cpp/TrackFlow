<template lang="pug">
div.projects-page
  //- ------------------------------------------------------------
  //- CARD GRANDI — una per ogni progetto che ha un segmento aperto
  //- in questo momento (può essere più di uno, o nessuno).
  //- ------------------------------------------------------------
  div.hero-row(v-if="playingProjects.length > 0")
    div.hero-card(v-for="p in playingProjects" :key="p.id")
      div.hero-left
        div.live-dot
        div
          div.hero-label {{ $t('projects.inProgressNow') }}
          div.hero-name(@click="openProjectDetail(p)") {{ p.name }}
          div.hero-time {{ formatDuration(liveSecondsFor(p)) }}
      div.hero-controls
        div.pill-btn(@click="togglePlay(p)") {{ isPlaying(p) ? $t('home.activeProjects.pause') : $t('home.activeProjects.resume') }}
        div.pill-btn-ghost(@click="closeProject(p)") {{ $t('projects.closeProject') }}

  div.grid-head {{ $t('projects.allProjects') }}
  div.projects-grid
    project-card(
      v-for="p in projectsStore.activeProjects"
      :key="p.id"
      :project="p"
      :is-playing="isPlaying(p)"
      :stats="statsFor(p)"
      :is-over-budget="isOverBudget(p)"
      @toggle-play="togglePlay(p)"
      @open-detail="openProjectDetail(p)"
      @edit="openEdit(p)"
      @close="closeProject(p)"
    )

    div.new-project-card(@click="onAddProject")
      div.new-plus +
      div {{ $t('home.activeProjects.newProject') }}

  div.closed-section(v-if="projectsStore.closedProjects.length > 0")
    div.grid-head {{ $t('projects.closedProjects') }}
    div.closed-grid
      div.closed-card(v-for="p in projectsStore.closedProjects" :key="p.id" @click="openProjectDetail(p)")
        div.closed-card-dot
        div.closed-card-name {{ p.name }}
        div.closed-card-total {{ formatDuration(statsFor(p).totalSeconds) }} {{ $t('projects.total') }}
        div.closed-card-date(v-if="p.closedDate") {{ $t('projects.closedOn') }} {{ formatLongDate(p.closedDate) }}

  project-edit-modal(
    v-if="editingProject"
    :project="editingProject"
    @save="saveEdit"
    @cancel="closeEdit"
  )

  project-detail-modal(
    v-if="viewingProject"
    :project="viewingProject"
    :total-seconds="statsFor(viewingProject).totalSeconds"
    :days-worked="daysWorkedFor(viewingProject)"
    :chart-data="detailChartData"
    :sessions="detailSessions"
    :overage-lines="detailOverageLines"
    :is-playing="isPlaying(viewingProject)"
    @close="closeProjectDetail"
    @toggle-play="togglePlay(viewingProject)"
    @close-project="closeProjectFromDetail(viewingProject)"
    @delete="deleteProjectForever(viewingProject)"
    @reopen="reopenProject(viewingProject)"
  )

  confirm-modal(
    v-if="pendingPlayProject"
    :title="$t('home.activeProjects.overBudgetTitle')"
    :confirm-label="$t('home.activeProjects.startAnyway')"
    :cancel-label="$t('home.activeProjects.cancel')"
    @confirm="confirmPendingPlay"
    @cancel="cancelPendingPlay"
  )
    div.detail-warning
      div.detail-warning-line(v-for="(line, i) in pendingPlayLines" :key="i") ⚠ {{ line }}
    div.confirm-play-text {{ $t('home.activeProjects.confirmStartAnyway') }}

  confirm-modal(
    v-if="pendingDeleteProject"
    :title="$t('projects.deleteConfirmTitle')"
    :confirm-label="$t('projects.delete')"
    :cancel-label="$t('projects.cancel')"
    @confirm="confirmPendingDelete"
    @cancel="cancelPendingDelete"
  )
    div.confirm-play-text
      | {{ $t('projects.deleteConfirmText', { name: pendingDeleteProject.name }) }}
</template>

<style lang="scss" scoped>
@import '../style/theme.css';
@import '../style/modals.css';
@import '../style/projectCards.css';

$overBudgetColor: #d9534f;

.projects-page {
  padding: 24px 28px;
}

.closed-section {
  margin-top: 32px;
}

.closed-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
  gap: 12px;
}

.closed-card {
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  padding: 18px;
  cursor: pointer;
  display: flex;
  flex-direction: column;
  gap: 3px;

  &:hover {
    filter: brightness(1.15);
  }
}

.closed-card-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background-color: var(--color-accent1);
  margin-bottom: 5px;
}

.closed-card-name {
  font-size: var(--font-size-base);
  font-weight: var(--font-weight-bold);
  color: var(--color-text-dim);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.closed-card-total,
.closed-card-date {
  font-size: var(--font-size-xs);
  color: var(--color-text-faint);
}

// ------------------------------------------------------------
// Contenuto dei popup di conferma (sforamento / cancellazione) —
// sono passati come slot a <confirm-modal>, quindi restano stilati
// da questo file anche se visivamente compaiono dentro al componente
// figlio (le regole scoped di Vue seguono il template in cui il
// contenuto è stato scritto, non quello in cui viene renderizzato).
// ------------------------------------------------------------
.detail-warning {
  background-color: rgba(217, 83, 79, 0.15);
  border: 1px solid $overBudgetColor;
  border-radius: var(--radius-md);
  padding: 10px 14px;
  margin-bottom: 16px;
}

.detail-warning-line {
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-semibold);
  color: $overBudgetColor;

  & + & {
    margin-top: 4px;
  }
}

.confirm-play-text {
  font-size: var(--font-size-sm);
  color: var(--color-text-dim);
  margin-top: 14px;
}
</style>

<script lang="ts">
import moment from 'moment';
import { useProjectsStore, Project } from '~/stores/projects';
import { formatDuration, formatLongDate } from '~/util/projectTime';
import projectTimerMixin, { Segment } from '~/mixins/projectTimerMixin';

export default {
  name: 'Progetti',
  mixins: [projectTimerMixin],
  components: {
    'project-card': () => import('~/components/ProjectCard.vue'),
    'project-edit-modal': () => import('~/components/ProjectEditModal.vue'),
    'project-detail-modal': () => import('~/components/ProjectDetailModal.vue'),
    'confirm-modal': () => import('~/components/ConfirmModal.vue'),
  },
  data() {
    return {
      editingProject: null as Project | null,
      viewingProject: null as Project | null,
      // Progetto in attesa di conferma "Elimina" (cancellazione
      // definitiva) — null quando nessun popup è aperto.
      pendingDeleteProject: null as Project | null,
    };
  },
  computed: {
    // Ricalcolati ad ogni render a partire dal progetto aperto nel
    // popup: così, se dentro al popup premi Pausa/Riprendi su un
    // progetto ancora attivo, grafico e storico si aggiornano subito
    // invece di restare fermi allo stato di quando l'hai aperto.
    detailChartData(): any[] {
      return this.viewingProject ? this.buildChartData(this.viewingProject) : [];
    },
    detailSessions(): any[] {
      return this.viewingProject ? this.buildSessions(this.viewingProject) : [];
    },
    detailOverageLines(): string[] {
      return this.viewingProject ? this.overageLines(this.viewingProject) : [];
    },
  },
  watch: {
    // Set either by the Topbar (search suggestion, page already
    // mounted) or by navigating in from Home's "Progetti attivi" card
    // (set before this page even mounts — hence `immediate: true`, so
    // the already-set value is caught too, not just future changes):
    // open the detail popup for that project, then clear the signal.
    'projectsStore.requestedDetailProjectId': {
      immediate: true,
      handler(id: string | null) {
        if (!id) return;
        const project = this.projectsStore.getProjectById(id);
        if (project) this.viewingProject = project;
        this.projectsStore.clearRequestedProjectDetail();
      },
    },
  },
  methods: {
    openEdit(project: Project) {
      this.editingProject = project;
    },
    closeEdit() {
      this.editingProject = null;
    },
    async saveEdit(payload: { name: string; budgetHours: number | null; targetDate: string | null }) {
      const id = this.editingProject.id;
      this.projectsStore.renameProject({ id, name: payload.name });
      this.projectsStore.setBudget({ id, budgetHours: payload.budgetHours });
      this.projectsStore.setTargetDate({ id, targetDate: payload.targetDate });
      await this.projectsStore.save();
      this.editingProject = null;
    },

    openProjectDetail(project: Project) {
      this.viewingProject = project;
    },
    // Costruisce i 14 giorni prima della chiusura (o prima di oggi,
    // se per qualche motivo non c'è una data di chiusura), con le ore
    // lavorate in ciascuno e l'altezza della barra in percentuale
    // rispetto al giorno più carico del gruppo.
    buildChartData(project: Project) {
      const fine = project.closedDate ? moment(project.closedDate) : moment();
      const blocchiDelProgetto = this.allBlocks.filter(e => e.data.projectId === project.id);
      const giorni = [];
      for (let i = 13; i >= 0; i--) {
        const d = fine.clone().subtract(i, 'days');
        const dateStr = d.format('YYYY-MM-DD');
        let secondi = 0;
        for (const blocco of blocchiDelProgetto) {
          for (const s of blocco.data.segments as Segment[]) {
            if (moment(s.start).format('YYYY-MM-DD') === dateStr) {
              const fineSeg = s.end ? moment(s.end) : this.now;
              secondi += fineSeg.diff(moment(s.start), 'seconds');
            }
          }
        }
        giorni.push({
          dateStr,
          dayLabel: d.format('D/M'),
          oreLabel: (secondi / 3600).toFixed(1),
          secondi,
          heightPct: 0,
        });
      }
      const massimo = Math.max(...giorni.map(g => g.secondi), 1);
      giorni.forEach(g => (g.heightPct = (g.secondi / massimo) * 100));
      return giorni;
    },
    // "Srotola" tutti i segmenti (ogni singolo play->pausa) di tutti i
    // blocchi del progetto in righe di tabella, dalla più recente.
    buildSessions(project: Project) {
      const blocchiDelProgetto = this.allBlocks.filter(e => e.data.projectId === project.id);
      const righe = [];
      for (const blocco of blocchiDelProgetto) {
        for (const s of blocco.data.segments as Segment[]) {
          const inizio = moment(s.start);
          const fine = s.end ? moment(s.end) : this.now;
          righe.push({
            date: inizio.format('D MMM YYYY'),
            range:
              inizio.format('HH:mm') +
              ' - ' +
              (s.end ? fine.format('HH:mm') : (this.$t('projects.detail.inProgress') as string)),
            duration: formatDuration(fine.diff(inizio, 'seconds')),
            sortKey: inizio.valueOf(),
          });
        }
      }
      return righe.sort((a, b) => b.sortKey - a.sortKey);
    },
    closeProjectDetail() {
      this.viewingProject = null;
    },
    // Chiude il progetto direttamente dal popup di dettaglio (stessa
    // azione della "✕" sulla card), poi chiude anche il popup — dopo
    // l'archiviazione non avrebbe senso restarci dentro.
    async closeProjectFromDetail(project: Project) {
      await this.closeProject(project);
      this.viewingProject = null;
    },
    daysWorkedFor(project: Project): number {
      const blocchiDelProgetto = this.allBlocks.filter(e => e.data.projectId === project.id);
      const giorni = new Set<string>();
      for (const blocco of blocchiDelProgetto) {
        for (const s of blocco.data.segments as Segment[]) {
          giorni.add(moment(s.start).format('YYYY-MM-DD'));
        }
      }
      return giorni.size;
    },
    async reopenProject(project: Project) {
      this.projectsStore.reopenProject({ id: project.id });
      await this.projectsStore.save();
      this.viewingProject = null;
    },
    // Elimina il progetto per sempre — diverso da "Chiudi progetto"
    // (che è solo un'archiviazione reversibile). Non tocca le
    // sessioni già registrate nel bucket aw-stopwatch: restano lì
    // come blocchi orfani, semplicemente non più collegati a nessun
    // progetto visibile. Non usiamo il confirm() nativo del browser
    // (si è visto essere inaffidabile) — apriamo invece il nostro
    // popup di conferma in stile con il resto dell'app.
    deleteProjectForever(project: Project) {
      this.pendingDeleteProject = project;
    },
    async confirmPendingDelete() {
      const project = this.pendingDeleteProject;
      this.pendingDeleteProject = null;
      if (!project) return;
      this.projectsStore.removeProject({ id: project.id });
      await this.projectsStore.save();
      this.viewingProject = null;
    },
    cancelPendingDelete() {
      this.pendingDeleteProject = null;
    },
  },
};
</script>

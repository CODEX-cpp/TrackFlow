<template lang="pug">
div.active-projects-section
  div.section-label {{ $t('home.activeProjects.title') }}

  div.projects-grid
    div.pa-card(v-for="p in projectsStore.activeProjects" :key="p.id")
      div.pa-header
        div.live-dot
        div.pa-name(@click="goToProjectDetail(p)") {{ p.name }}
        div.pa-edit-icon(@click="openEdit(p)") ✎
      div.hero-label {{ $t('home.activeProjects.workedToday') }}
      div.hero-time {{ formatDuration(liveSecondsFor(p)) }}
      div.pa-total(:class="{ 'pa-total-warning': isOverBudget(p) }") {{ formatHoursMinutes(statsFor(p).totalSeconds) }} {{ $t('home.activeProjects.totalHistoric') }}
      div.pill-btn.pa-play-btn(@click="togglePlay(p)") {{ isPlaying(p) ? $t('home.activeProjects.pause') : $t('home.activeProjects.resume') }}

    div.new-project-card(@click="onAddProject")
      div.new-plus +
      div {{ $t('home.activeProjects.newProject') }}

  project-edit-modal(
    v-if="editingProject"
    :project="editingProject"
    @save="saveEdit"
    @cancel="closeEdit"
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
</template>

<style lang="scss" scoped>
@import '../style/theme.css';
@import '../style/modals.css';
@import '../style/projectCards.css';

.active-projects-section {
  // Sits directly in .app-main now (see App.vue), not inside the old
  // padded Bootstrap container — so it needs its own horizontal
  // padding, matching the same rhythm as the Projects page itself.
  padding: 24px 28px 8px;
}

.section-label {
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-bold);
  color: var(--color-text-faint);
  text-transform: uppercase;
  letter-spacing: var(--letter-spacing-wide);
  margin-bottom: 12px;
}

// Custom card for this section only (see the note in <script> for why
// it isn't the shared ProjectCard.vue) — same visual language as the
// old hero card (.hero-label/.hero-time reused as-is from
// projectCards.css) but stacked to fit a grid cell, and with only two
// controls: play/pause and edit.
.pa-card {
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-xl);
  padding: 22px;
  display: flex;
  flex-direction: column;
}

.pa-header {
  display: flex;
  align-items: center;
  gap: 10px;
}

.pa-name {
  flex: 1;
  min-width: 0;
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-bold);
  color: var(--color-text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  cursor: pointer;
}

.pa-name:hover {
  color: var(--color-accent1);
}

.pa-edit-icon {
  flex-shrink: 0;
  width: 30px;
  height: 30px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-md);
  background-color: var(--color-surface2);
  color: var(--color-text-dim);
  font-size: 14px;
  cursor: pointer;
}

.pa-edit-icon:hover {
  filter: brightness(1.15);
  color: var(--color-text);
}

.hero-label {
  margin-top: 18px;
}

.pa-total {
  font-size: var(--font-size-sm);
  color: var(--color-text-faint);
  margin-top: 4px;
}

// Same warning treatment as the old ProjectCard.vue's over-budget
// total — kept even though the reference photo didn't happen to show
// an over-budget example, this signal is relied on elsewhere (section
// 7.2) and dropping it here would be a real regression, not a style
// choice.
.pa-total-warning {
  color: #d9534f;
  font-weight: var(--font-weight-bold);
}

.pa-play-btn {
  width: 100%;
  box-sizing: border-box;
  margin-top: 22px;
}

.detail-warning {
  background-color: rgba(217, 83, 79, 0.15);
  border: 1px solid #d9534f;
  border-radius: var(--radius-md);
  padding: 10px 14px;
  margin-bottom: 16px;
}

.detail-warning-line {
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-semibold);
  color: #d9534f;
}

.detail-warning-line + .detail-warning-line {
  margin-top: 4px;
}

.confirm-play-text {
  font-size: var(--font-size-sm);
  color: var(--color-text-dim);
  margin-top: 14px;
}
</style>

<script lang="ts">
// "Active projects" overview — same live timer card/grid as the
// Projects page (via projectTimerMixin), placed at the top of Home.
// Deliberately lighter than the full Projects page: no closed-projects
// section, no inline detail popup, no delete. Clicking a project's
// name navigates to the Projects page and opens its detail popup
// there (same cross-component signal the topbar search suggestion
// uses), instead of duplicating that popup here.
//
// No big "in corso ora" hero card anymore (explicit request) — just
// the grid below. Cards here are custom (not the shared ProjectCard.vue
// used by Progetti.vue's own "Tutti i progetti" grid — that one stays
// untouched): explicit request to restyle only this section to the
// hero-like look (big live time, "totali storici" subtext) with just
// two controls per card — play/pause and edit. "Chiudi progetto" isn't
// on the card anymore; still reachable from the project's detail popup
// (goToProjectDetail), same as before this change for any control not
// exposed directly on the card.
import { Project } from '~/stores/projects';
import { formatHoursMinutes } from '~/util/projectTime';
import projectTimerMixin from '~/mixins/projectTimerMixin';

export default {
  name: 'ActiveProjectsSection',
  components: {
    'project-edit-modal': () => import('./ProjectEditModal.vue'),
    'confirm-modal': () => import('./ConfirmModal.vue'),
  },
  mixins: [projectTimerMixin],
  data() {
    return {
      editingProject: null as Project | null,
      // Auto-refresh (explicit request, 30s, same as HomeTimelineSection
      // and the Moduli panels): re-fetch aw-stopwatch while this section
      // is mounted, so a play/pause/close done elsewhere (another tab,
      // another device) shows up here without a manual reload. No
      // "skip if unchanged" short-circuit here unlike the other two —
      // pause/resume/close all replaceEvent() the SAME event id with new
      // segment data, so a cheap count/last-id fingerprint (safe for the
      // append-only raw watcher events elsewhere) would wrongly call a
      // real pause/resume "unchanged" and hide it. fetchBlocks() itself
      // is a single lightweight request either way.
      refreshInterval: null as ReturnType<typeof setInterval> | null,
    };
  },
  mounted() {
    this.refreshInterval = setInterval(() => this.fetchBlocks(), 30000);
  },
  beforeDestroy() {
    if (this.refreshInterval) clearInterval(this.refreshInterval);
  },
  methods: {
    formatHoursMinutes,
    goToProjectDetail(project: Project) {
      this.projectsStore.requestProjectDetail(project.id);
      this.$router.push('/stopwatch');
    },
    openEdit(project: Project) {
      this.editingProject = project;
    },
    closeEdit() {
      this.editingProject = null;
    },
    async saveEdit(payload: {
      name: string;
      budgetHours: number | null;
      targetDate: string | null;
    }) {
      const id = this.editingProject.id;
      this.projectsStore.renameProject({ id, name: payload.name });
      this.projectsStore.setBudget({ id, budgetHours: payload.budgetHours });
      this.projectsStore.setTargetDate({ id, targetDate: payload.targetDate });
      await this.projectsStore.save();
      this.editingProject = null;
    },
  },
};
</script>

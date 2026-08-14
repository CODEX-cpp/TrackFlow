<template lang="pug">
div
  div.modal-backdrop(@click="$emit('close')")
  div.detail-modal
    div.edit-modal-title {{ project.name }}

    div.detail-warning(v-if="overageLines.length > 0")
      div.detail-warning-line(v-for="(line, i) in overageLines" :key="i") ⚠ {{ line }}

    div.detail-stats-row
      div.detail-stat
        div.detail-stat-label {{ $t('projects.detail.totalHours') }}
        div.detail-stat-value {{ formatDuration(totalSeconds) }}
      div.detail-stat
        div.detail-stat-label {{ $t('projects.detail.daysWorked') }}
        div.detail-stat-value {{ daysWorked }}
      div.detail-stat
        div.detail-stat-label {{ $t('projects.detail.budgetHours') }}
        div.detail-stat-value {{ project.budgetHours ? project.budgetHours + 'h' : $t('projects.detail.notSet') }}
      div.detail-stat
        div.detail-stat-label {{ $t('projects.detail.deadline') }}
        div.detail-stat-value {{ project.targetDate ? formatLongDate(project.targetDate) : $t('projects.detail.notSet') }}
      div.detail-stat(v-if="project.closedDate")
        div.detail-stat-label {{ $t('projects.detail.closedOn') }}
        div.detail-stat-value {{ formatLongDate(project.closedDate) }}

    div.detail-chart-head {{ $t('projects.detail.last14Days') }}
    div.chart-row
      div.chart-col(v-for="d in chartData" :key="d.dateStr")
        div.chart-value(v-if="d.secondi > 0") {{ d.oreLabel }}h
        div.chart-bar(:style="{ height: d.heightPct + '%' }")
        div.chart-day-label {{ d.dayLabel }}

    div.detail-chart-head {{ $t('projects.detail.sessionHistory') }}
    div.sessions-empty(v-if="sessions.length === 0") {{ $t('projects.detail.noSessions') }}
    table.sessions-table(v-else)
      thead
        tr
          th {{ $t('projects.detail.colDate') }}
          th {{ $t('projects.detail.colTime') }}
          th {{ $t('projects.detail.colDuration') }}
      tbody
        tr(v-for="s in sessions" :key="s.sortKey")
          td {{ s.date }}
          td {{ s.range }}
          td {{ s.duration }}

    div.edit-modal-actions
      div.pill-btn-ghost(@click="$emit('close')") {{ $t('projects.detail.close') }}
      template(v-if="project.closed")
        div.pill-btn-danger(@click="$emit('delete')") {{ $t('projects.detail.deleteForever') }}
        div.pill-btn(@click="$emit('reopen')") {{ $t('projects.detail.reopen') }}
      template(v-else)
        div.pill-btn(@click="$emit('toggle-play')") {{ isPlaying ? $t('home.activeProjects.pause') : $t('home.activeProjects.resume') }}
        div.pill-btn-ghost(@click="$emit('close-project')") {{ $t('projects.closeProject') }}
</template>

<style lang="scss" scoped>
@import '../style/theme.css';
@import '../style/modals.css';

$overBudgetColor: #d9534f;

.detail-modal {
  position: fixed;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  width: 560px;
  max-height: 80vh;
  overflow-y: auto;
  background-color: var(--color-bg-elev);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-xl);
  box-shadow: var(--shadow-elevated);
  padding: 24px;
  z-index: 50;
}

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

.detail-stats-row {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  margin-bottom: 24px;
}

.detail-stat {
  flex: 1 1 100px;
  background-color: var(--color-surface2);
  border-radius: var(--radius-md);
  padding: 10px 12px;
}

.detail-stat-label {
  font-size: 10.5px;
  color: var(--color-text-faint);
  text-transform: uppercase;
  letter-spacing: var(--letter-spacing-wide);
  margin-bottom: 4px;
}

.detail-stat-value {
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-bold);
  color: var(--color-text);
}

.detail-chart-head {
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-bold);
  color: var(--color-text-dim);
  margin: 20px 0 12px;
}

.chart-row {
  display: flex;
  align-items: flex-end;
  gap: 4px;
  height: 130px;
}

.chart-col {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: flex-end;
  height: 100%;
}

.chart-value {
  font-size: 9px;
  color: var(--color-text-faint);
  margin-bottom: 3px;
  white-space: nowrap;
}

.chart-bar {
  width: 100%;
  min-height: 2px;
  background-color: var(--color-accent1);
  border-radius: 3px 3px 0 0;
}

.chart-day-label {
  font-size: 9px;
  color: var(--color-text-faint);
  margin-top: 4px;
}

.sessions-empty {
  font-size: var(--font-size-sm);
  color: var(--color-text-faint);
}

.sessions-table {
  width: 100%;
  border-collapse: collapse;
  font-size: var(--font-size-sm);

  th {
    text-align: left;
    font-size: 10.5px;
    color: var(--color-text-faint);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-wide);
    padding-bottom: 8px;
    border-bottom: 1px solid var(--color-border);
  }

  td {
    padding: 8px 0;
    border-bottom: 1px solid var(--color-border);
    color: var(--color-text-dim);
  }

  tr:last-child td {
    border-bottom: none;
  }
}
</style>

<script lang="ts">
import { Project } from '~/stores/projects';
import { formatDuration, formatLongDate } from '~/util/projectTime';

// Detail popup for a single project — 14-day chart, full session
// history, and (for still-active projects) the overage warning banner
// plus play/pause controls. Purely presentational: all the numbers
// (totals, chart data, sessions) are computed by the parent from the
// live aw-stopwatch events and passed in as props, since that data
// depends on page-level state (all blocks, the ticking clock) this
// component doesn't own.
export default {
  name: 'ProjectDetailModal',
  props: {
    project: { type: Object as () => Project, required: true },
    totalSeconds: { type: Number, required: true },
    daysWorked: { type: Number, required: true },
    chartData: { type: Array, required: true },
    sessions: { type: Array, required: true },
    overageLines: { type: Array as () => string[], required: true },
    isPlaying: { type: Boolean, required: true },
  },
  methods: {
    formatDuration,
    formatLongDate,
  },
};
</script>

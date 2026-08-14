<template lang="pug">
div.project-card
  div.card-title-row
    div.card-name(@click="$emit('open-detail')") {{ project.name }}
    div.card-btn(@click="$emit('toggle-play')") {{ isPlaying ? '⏸' : '▶' }}
  div.card-total(:class="{ 'card-total-warning': isOverBudget }") {{ formatDuration(stats.totalSeconds) }} {{ $t('home.projectCard.total') }}
  div.card-today {{ $t('home.projectCard.today') }} {{ formatDuration(stats.todaySeconds) }}
  div.card-last {{ stats.lastSessionLabel }}

  div.card-icon-row
    div.icon-btn(@click="$emit('edit')") ✎
    div.icon-btn(@click="$emit('close')") ✕
</template>

<style lang="scss" scoped>
@import '../style/theme.css';

$onAccentText: #241a12;
$overBudgetColor: #d9534f;

.project-card {
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-xl);
  padding: 24px;
  min-height: 220px;
  display: flex;
  flex-direction: column;
}

.card-title-row {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 16px;
}

.card-btn {
  width: 38px;
  height: 38px;
  flex-shrink: 0;
  border-radius: 50%;
  background-color: var(--color-accent1);
  color: $onAccentText;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 14px;
  cursor: pointer;

  &:hover {
    filter: brightness(1.1);
  }
}

.card-name {
  flex: 1;
  min-width: 0;
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-bold);
  color: var(--color-text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  cursor: pointer;

  &:hover {
    color: var(--color-accent1);
  }
}

.card-total {
  font-size: var(--font-size-sm);
  color: var(--color-text-faint);
  font-variant-numeric: tabular-nums;
  margin-top: 6px;
}

// Same warning treatment whether the project is over its hour budget
// or past its deadline — the caller collapses both into one flag.
.card-total.card-total-warning {
  color: $overBudgetColor;
  font-weight: var(--font-weight-bold);
}

.card-today,
.card-last {
  font-size: var(--font-size-xs);
  color: var(--color-text-faint);
  margin-top: 4px;
}

.card-last {
  margin-bottom: 14px;
}

.card-icon-row {
  margin-top: auto;
  padding-top: 14px;
  display: flex;
  gap: 8px;
}

.icon-btn {
  flex: 1;
  height: 38px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-md);
  background-color: var(--color-surface2);
  color: var(--color-text-dim);
  font-size: 16px;
  cursor: pointer;

  &:hover {
    filter: brightness(1.15);
    color: var(--color-text);
  }
}
</style>

<script lang="ts">
import { Project } from '~/stores/projects';
import { formatDuration } from '~/util/projectTime';

// One card in the "All projects" grid. Purely presentational: the
// parent computes isPlaying/stats/isOverBudget from the live
// aw-stopwatch events (which this component has no access to) and
// passes them in as props.
export default {
  name: 'ProjectCard',
  props: {
    project: { type: Object as () => Project, required: true },
    isPlaying: { type: Boolean, required: true },
    stats: {
      type: Object as () => { totalSeconds: number; todaySeconds: number; lastSessionLabel: string },
      required: true,
    },
    isOverBudget: { type: Boolean, required: true },
  },
  methods: {
    formatDuration,
  },
};
</script>

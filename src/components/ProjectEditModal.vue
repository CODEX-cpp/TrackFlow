<template lang="pug">
div
  div.modal-backdrop(@click="$emit('cancel')")
  div.edit-modal
    div.edit-modal-title {{ $t('projects.editTitle') }}

    div.edit-field-label {{ $t('projects.projectName') }}
    input.edit-field(v-model="form.name")

    div.edit-field-label {{ $t('projects.budgetHours') }}
    div.budget-input-row
      input.edit-field.budget-field(type="number" step="0.5" min="1" max="500" v-model.number="form.budgetHoursInput" :placeholder="$t('projects.budgetPlaceholder')")
      div.budget-step-btn(@click="stepBudget(-0.5)") −
      div.budget-step-btn(@click="stepBudget(0.5)") +

    div.edit-field-label {{ $t('projects.targetDate') }}
    date-segment-input(v-model="form.targetDateInput" :is-date-disabled="isPastDate")

    div.edit-modal-actions
      div.pill-btn-ghost(@click="$emit('cancel')") {{ $t('projects.cancel') }}
      div.pill-btn(:class="{ 'pill-btn-disabled': !!budgetError }" @click="!budgetError && save()") {{ $t('projects.save') }}
</template>

<style lang="scss" scoped>
@import '../style/theme.css';
@import '../style/modals.css';

.budget-input-row {
  display: flex;
  gap: 8px;
  align-items: stretch;
}

.budget-field {
  flex: 1;
  width: auto;

  // Hide the native browser spin buttons (blue, off-theme, not
  // restylable in a cross-browser way) — replaced by the two custom
  // step buttons next to the field.
  &::-webkit-inner-spin-button,
  &::-webkit-outer-spin-button {
    -webkit-appearance: none;
    margin: 0;
  }

  -moz-appearance: textfield;
}

.budget-step-btn {
  width: 38px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-md);
  background-color: var(--color-surface2);
  color: var(--color-text-dim);
  font-size: 18px;
  cursor: pointer;
  user-select: none;

  &:hover {
    background-color: var(--color-accent1);
    color: #241a12;
  }
}
</style>

<script lang="ts">
import moment from 'moment';
import { Project } from '~/stores/projects';

// Popup form for renaming a project and setting its budget/deadline.
// Owns its own draft state (form) so the parent only has to react to
// a single "save" event with the already-validated payload, the same
// way it worked inline in Progetti.vue before this was split out.
export default {
  name: 'ProjectEditModal',
  components: {
    'date-segment-input': () => import('./DateSegmentInput.vue'),
  },
  props: {
    project: { type: Object as () => Project, required: true },
  },
  data() {
    return {
      form: {
        name: this.project.name,
        budgetHoursInput: this.project.budgetHours as number | null,
        // If the project has no deadline yet, prefill with today
        // instead of leaving it empty — so you start from a sensible
        // value and only adjust what you need (e.g. just the day, or
        // move to next month).
        targetDateInput: this.project.targetDate || moment().format('YYYY-MM-DD'),
      },
    };
  },
  computed: {
    budgetError(): string {
      const v = this.form.budgetHoursInput;
      if (v != null && !isNaN(v) && (v < 1 || v > 500)) {
        return this.$t('projects.budgetError') as string;
      }
      return '';
    },
  },
  methods: {
    isPastDate(dateStr: string): boolean {
      return moment(dateStr).isBefore(moment(), 'day');
    },
    // Mirrors the native number-input spinner behaviour (respects
    // step/min/max) since we hide the native buttons and replace them
    // with our own themed ones.
    stepBudget(delta: number) {
      const current = this.form.budgetHoursInput;
      if (current == null || isNaN(current)) {
        if (delta > 0) this.form.budgetHoursInput = 1;
        return;
      }
      const next = Math.round((current + delta) * 2) / 2;
      this.form.budgetHoursInput = Math.min(500, Math.max(1, next));
    },
    save() {
      // Budget hours must be a number between 1 and 500, otherwise
      // treat it as "no budget set".
      let budget = this.form.budgetHoursInput;
      if (budget != null && (isNaN(budget) || budget < 1 || budget > 500)) {
        budget = null;
      }
      this.$emit('save', {
        name: this.form.name,
        budgetHours: budget,
        targetDate: this.form.targetDateInput || null,
      });
    },
  },
};
</script>

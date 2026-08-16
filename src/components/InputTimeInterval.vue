<template lang="pug">
div
  div.time-interval-warning(v-if="invalidDaterange")
    | {{ $t('modals.inputTimeInterval.invalidRange') }}
  div.time-interval-warning(v-if="daterangeTooLong")
    | {{ $t('modals.inputTimeInterval.tooLongRange', { days: maxDuration/(24*60*60) }) }}

  div.input-time-interval
    div.time-interval-grid
      label.time-interval-label {{ $t('modals.inputTimeInterval.mode') }}
      div.mode-toggle
        div.mode-toggle-option(
          v-for="opt in modeOptions"
          :key="opt.value"
          :class="{ 'mode-toggle-option-active': mode === opt.value }"
          @click="mode = opt.value; valueChanged()"
        )
          | {{ opt.text }}

      label.time-interval-label {{ $t('modals.inputTimeInterval.range') }}
      div.duration-row(v-if="mode == 'last_duration'")
        div.duration-pill(
          v-for="(dur, idx) in durations"
          :key="idx"
          :class="{ 'duration-pill-active': duration === dur.seconds }"
          @click="duration = dur.seconds; applyLastDuration()"
          v-html="dur.label"
        )
      div.range-row(v-else)
        input.time-interval-field(
          type="date", v-model="start", :max="end || undefined"
          :aria-label="$t('modals.inputTimeInterval.startDate')"
        )
        input.time-interval-field(
          type="date", v-model="end", :min="start || undefined"
          :aria-label="$t('modals.inputTimeInterval.endDate')"
        )
        div.pill-btn(
          :class="{ 'pill-btn-disabled': invalidDaterange || emptyDaterange || daterangeTooLong }"
          @click="applyRange"
        ) {{ $t('modals.inputTimeInterval.apply') }}

    div.time-interval-update(v-if="showUpdate")
      div.pill-btn-ghost(@click="refresh()")
        icon.mr-1(name="sync")
        | {{ $t('modals.inputTimeInterval.refresh') }}
      div.time-interval-last-update(v-if="lastUpdate")
        | {{ $t('modals.inputTimeInterval.lastUpdate') }} #[time(:datetime="lastUpdate.format()") {{lastUpdate | friendlytime}}]
</template>

<style scoped lang="scss">
@import '../style/theme.css';

.time-interval-warning {
  background-color: var(--color-surface2);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  padding: 8px 12px;
  font-size: var(--font-size-sm);
  color: #d9534f;
  margin-bottom: 10px;
}

.input-time-interval {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-start;
  justify-content: space-between;
  row-gap: 12px;
  margin-bottom: 16px;
}

.time-interval-grid {
  display: grid;
  grid-template-columns: 4rem 1fr;
  column-gap: 12px;
  row-gap: 10px;
  align-items: center;
}

.time-interval-label {
  font-size: var(--font-size-xs);
  color: var(--color-text-faint);
}

.mode-toggle {
  display: inline-flex;
  border-radius: var(--radius-md);
  overflow: hidden;
  border: 1px solid var(--color-border);
  width: fit-content;
}

.mode-toggle-option {
  padding: 6px 12px;
  font-size: var(--font-size-sm);
  color: var(--color-text-dim);
  cursor: pointer;
  background-color: var(--color-surface2);
}

.mode-toggle-option-active {
  background-color: var(--color-accent1);
  color: #241a12;
}

.duration-row {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.duration-pill {
  padding: 5px 10px;
  border-radius: var(--radius-pill);
  font-size: var(--font-size-xs);
  color: var(--color-text-dim);
  background-color: var(--color-surface2);
  border: 1px solid var(--color-border);
  cursor: pointer;
}

.duration-pill-active {
  background-color: var(--color-accent1);
  color: #241a12;
  border-color: var(--color-accent1);
}

.range-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
}

.time-interval-field {
  padding: 6px 10px;
  border-radius: var(--radius-md);
  background-color: var(--color-surface2);
  border: 1px solid var(--color-border);
  color: var(--color-text);
  font-size: var(--font-size-sm);
  font-family: inherit;
  color-scheme: dark;
}

.time-interval-update {
  text-align: right;
}

.time-interval-last-update {
  margin-top: 6px;
  font-size: var(--font-size-xs);
  color: var(--color-text-faint);
}
</style>

<script lang="ts">
import moment from 'moment';
import 'vue-awesome/icons/sync';
export default {
  name: 'input-timeinterval',
  props: {
    defaultDuration: {
      type: Number,
      default: 60 * 60,
    },
    maxDuration: {
      type: Number,
      default: null,
    },
    showUpdate: {
      type: Boolean,
      default: true,
    },
  },
  data() {
    return {
      duration: null,
      mode: 'last_duration',
      start: null,
      end: null,
      lastUpdate: null,
      durations: [
        { seconds: 0.25 * 60 * 60, label: '&frac14;h' },
        { seconds: 0.5 * 60 * 60, label: '&frac12;h' },
        { seconds: 60 * 60, label: '1h' },
        { seconds: 2 * 60 * 60, label: '2h' },
        { seconds: 3 * 60 * 60, label: '3h' },
        { seconds: 4 * 60 * 60, label: '4h' },
        { seconds: 6 * 60 * 60, label: '6h' },
        { seconds: 12 * 60 * 60, label: '12h' },
        { seconds: 24 * 60 * 60, label: '24h' },
        { seconds: 48 * 60 * 60, label: '48h' },
      ],
    };
  },
  computed: {
    modeOptions() {
      return [
        { text: this.$t('modals.inputTimeInterval.lastDuration'), value: 'last_duration' },
        { text: this.$t('modals.inputTimeInterval.dateRange'), value: 'range' },
      ];
    },
    value: {
      get() {
        if (this.mode == 'range' && this.start) {
          const startDate = moment(this.start);
          // If only start date is set, show that single day
          const endDate = this.end
            ? moment(this.end).add(1, 'day')
            : startDate.clone().add(1, 'day');
          return [startDate, endDate];
        } else {
          return [moment().subtract(this.duration, 'seconds'), moment()];
        }
      },
    },
    emptyDaterange() {
      return !this.start;
    },
    invalidDaterange() {
      if (!this.end) return false;
      return moment(this.start) > moment(this.end);
    },
    daterangeTooLong() {
      if (!this.end) return false;
      return moment(this.start).add(this.maxDuration, 'seconds').isBefore(moment(this.end));
    },
  },
  mounted() {
    this.duration = this.defaultDuration;
    this.valueChanged();

    // We want our lastUpdated text to update every ~500ms
    // We can do this by setting it to null and then the previous value.
    this.lastUpdateTimer = setInterval(() => {
      const _lastUpdate = this.lastUpdate;
      this.lastUpdate = null;
      this.lastUpdate = _lastUpdate;
    }, 500);
  },
  beforeDestroy() {
    clearInterval(this.lastUpdateTimer);
  },
  methods: {
    valueChanged() {
      if (
        this.mode == 'last_duration' ||
        (!this.emptyDaterange && !this.invalidDaterange && !this.daterangeTooLong)
      ) {
        this.lastUpdate = moment();
        this.$emit('input', this.value);
      }
    },
    refresh() {
      const tmpMode = this.mode;
      this.mode = '';
      this.mode = tmpMode;
      this.valueChanged();
    },
    applyRange() {
      if (this.invalidDaterange || this.emptyDaterange || this.daterangeTooLong) return;
      this.mode = 'range';
      this.duration = 0;
      this.valueChanged();
    },
    applyLastDuration() {
      this.mode = 'last_duration';
      this.valueChanged();
    },
  },
};
</script>

<template lang="pug">
div.date-input-row
  input.edit-field.date-text-input(
    type="text"
    ref="dateInput"
    :value="displayValue"
    @focus="onFocus"
    @click="onClick"
    @keydown="onKeydown"
    @blur="onBlur"
  )
  div.date-calendar-toggle(:class="{ 'date-calendar-toggle-active': calendarOpen }" @click="calendarOpen = !calendarOpen")
    icon(name="calendar-day")
  div.date-popover-backdrop(v-if="calendarOpen" @click="calendarOpen = false")
  div.date-field-anchor(v-if="calendarOpen")
    calendar-picker(
      :selected-date="value"
      :initial-view-date="value"
      :is-date-disabled="isDateDisabled"
      @select="onPickDate"
    )
</template>

<style lang="scss" scoped>
@import '../style/theme.css';
@import '../style/modals.css';

.date-input-row {
  display: flex;
  gap: 8px;
  align-items: stretch;
  position: relative;
}

.date-text-input {
  flex: 1;
  width: auto;
}

.date-calendar-toggle {
  width: 38px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-md);
  background-color: var(--color-surface2);
  color: var(--color-text-dim);
  cursor: pointer;

  &:hover {
    background-color: var(--color-accent1);
    color: #241a12;
  }
}

.date-calendar-toggle-active {
  background-color: var(--color-accent1);
  color: #241a12;
}

.date-field-anchor {
  position: absolute;
  top: 100%;
  left: 0;
  margin-top: 8px;
  z-index: 60;
}

.date-popover-backdrop {
  position: fixed;
  inset: 0;
  z-index: 59;
}
</style>

<script lang="ts">
import moment from 'moment';
import 'vue-awesome/icons/calendar-day';

// A date field split into three typed segments (day/month/year)
// instead of one free-text box, with keyboard auto-advance and an
// attached calendar popover — used wherever the native
// <input type="date"> isn't controllable enough (styling, cross-
// browser quirks) and a free-text field would be too error-prone.
//
// v-model works on the outer date string (YYYY-MM-DD, or '' for no
// date) — this is Vue 2, so that means prop `value` / event `input`,
// not the Vue 3 `modelValue`/`update:modelValue` pair. The three
// segments are purely internal display state, kept in sync with the
// model via a watcher so external changes (e.g. the parent
// pre-filling the field when a modal opens) are reflected too.
export default {
  name: 'DateSegmentInput',
  components: {
    'calendar-picker': () => import('./CalendarPicker.vue'),
  },
  props: {
    value: { type: String, default: '' },
    // Same contract as CalendarPicker's own prop: given a
    // YYYY-MM-DD string, return whether that day should be
    // unselectable. Defaults to "nothing is disabled".
    isDateDisabled: { type: Function, default: () => false },
  },
  data() {
    return {
      calendarOpen: false,
      day: '',
      month: '',
      year: '',
      activeSegment: 'day' as 'day' | 'month' | 'year',
    };
  },
  computed: {
    displayValue(): string {
      const d = this.day || 'DD';
      const m = this.month || 'MM';
      const y = this.year || 'YYYY';
      return `${d}/${m}/${y}`;
    },
  },
  watch: {
    value: {
      immediate: true,
      handler(value: string) {
        this.setSegmentsFromDate(value);
      },
    },
  },
  methods: {
    onPickDate(dateStr: string) {
      this.$emit('input', dateStr);
      this.calendarOpen = false;
    },
    // Reads the free-typed text (DD/MM/YYYY) and converts it to the
    // model format (YYYY-MM-DD), or null if invalid/disabled.
    parseDisplayDate(text: string): string | null {
      if (!text) return null;
      const m = moment(text, 'DD/MM/YYYY', true);
      if (!m.isValid() || this.isDateDisabled(m.format('YYYY-MM-DD'))) return null;
      return m.format('YYYY-MM-DD');
    },
    // Fills the three segments from a model-format date, or clears
    // them if there is no date.
    setSegmentsFromDate(dateStr: string) {
      if (!dateStr) {
        this.day = '';
        this.month = '';
        this.year = '';
        return;
      }
      const m = moment(dateStr);
      this.day = m.format('DD');
      this.month = m.format('MM');
      this.year = m.format('YYYY');
    },
    // Highlights the given segment inside the field, so you can type
    // straight over it without selecting it with the mouse first.
    selectSegment(segment: 'day' | 'month' | 'year') {
      this.activeSegment = segment;
      this.$nextTick(() => {
        const el = this.$refs.dateInput as HTMLInputElement;
        if (!el) return;
        const ranges = { day: [0, 2], month: [3, 5], year: [6, 10] };
        const [start, end] = ranges[segment];
        el.setSelectionRange(start, end);
      });
    },
    onFocus() {
      this.selectSegment('day');
    },
    onClick() {
      const el = this.$refs.dateInput as HTMLInputElement;
      const pos = el.selectionStart || 0;
      if (pos <= 2) this.selectSegment('day');
      else if (pos <= 5) this.selectSegment('month');
      else this.selectSegment('year');
    },
    onKeydown(e: KeyboardEvent) {
      if (/^[0-9]$/.test(e.key)) {
        e.preventDefault();
        this.typeDigitIntoSegment(e.key);
      } else if (e.key === 'Backspace') {
        e.preventDefault();
        this.clearActiveSegment();
      } else if (e.key === 'ArrowLeft') {
        e.preventDefault();
        this.goToPreviousSegment();
      } else if (e.key === 'ArrowRight' || e.key === '/') {
        e.preventDefault();
        this.goToNextSegment();
      } else if (e.key !== 'Tab') {
        e.preventDefault();
      }
    },
    // Writes the digit into the active segment. After filling day or
    // month (2 digits) it auto-advances to the next segment — the
    // year (4 digits) stays put, being the last one.
    typeDigitIntoSegment(digit: string) {
      if (this.activeSegment === 'day') {
        this.day = (this.day.length >= 2 ? '' : this.day) + digit;
        this.day.length >= 2 ? this.selectSegment('month') : this.selectSegment('day');
      } else if (this.activeSegment === 'month') {
        this.month = (this.month.length >= 2 ? '' : this.month) + digit;
        this.month.length >= 2 ? this.selectSegment('year') : this.selectSegment('month');
      } else {
        this.year = (this.year.length >= 4 ? '' : this.year) + digit;
        this.selectSegment('year');
      }
    },
    clearActiveSegment() {
      if (this.activeSegment === 'day') this.day = '';
      else if (this.activeSegment === 'month') this.month = '';
      else this.year = '';
      this.selectSegment(this.activeSegment);
    },
    goToPreviousSegment() {
      if (this.activeSegment === 'year') this.selectSegment('month');
      else this.selectSegment('day');
    },
    goToNextSegment() {
      if (this.activeSegment === 'day') this.selectSegment('month');
      else this.selectSegment('year');
    },
    // On blur: if the three segments form a valid, non-disabled date,
    // commit it to the model — otherwise clear everything. Same rule
    // applied on save by the parent form.
    onBlur() {
      const text = `${this.day}/${this.month}/${this.year}`;
      const parsed =
        this.day.length === 2 && this.month.length === 2 && this.year.length === 4
          ? this.parseDisplayDate(text)
          : null;
      this.$emit('input', parsed || '');
      this.setSegmentsFromDate(parsed || '');
    },
  },
};
</script>

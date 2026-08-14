<template lang="pug">
div.calendar-popover
  div.calendar-header
    span.calendar-nav-arrow(@click="prevMonth")
      span ‹
    span.calendar-month-label {{ monthLabel }}
    span.calendar-nav-arrow(@click="nextMonth")
      span ›

  div.calendar-weekday-row
    span.calendar-weekday(v-for="wd in weekdayLabels" :key="wd") {{ wd }}

  div.calendar-week(v-for="(week, i) in calendarWeeks" :key="i")
    div.calendar-cell(
      v-for="day in week"
      :key="day.dateStr"
      :class="{ 'calendar-cell-outside': !day.inMonth, 'calendar-cell-today': day.isToday, 'calendar-cell-selected': day.isSelected, 'calendar-cell-disabled': day.disabled }"
      @click="day.disabled || $emit('select', day.dateStr)"
    ) {{ day.dayNum }}
</template>

<style lang="scss" scoped>
@import '../style/theme.css';

.calendar-popover {
  width: 280px;
  padding: 14px;
  display: flex;
  flex-direction: column;
  gap: 4px;
  background-color: var(--color-bg-elev);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-elevated);
}

.calendar-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}

.calendar-month-label {
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-semibold);
  color: var(--color-text);
  text-transform: capitalize;
}

.calendar-nav-arrow {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  color: var(--color-text-faint);
  font-size: var(--font-size-md);
  line-height: 1;
  cursor: pointer;
  border-radius: var(--radius-sm);

  span {
    margin-top: -1px;
  }

  &:hover {
    background-color: var(--color-surface2);
    color: var(--color-text);
  }
}

.calendar-weekday-row,
.calendar-week {
  display: grid;
  grid-template-columns: repeat(7, 1fr);
}

.calendar-weekday-row {
  margin-bottom: 4px;
}

.calendar-weekday {
  text-align: center;
  font-size: 10.5px;
  font-weight: var(--font-weight-bold);
  color: var(--color-text-faint);
  text-transform: uppercase;
}

.calendar-cell {
  text-align: center;
  padding: 6px 0;
  font-size: var(--font-size-sm);
  color: var(--color-text-dim);
  cursor: pointer;
  border-radius: var(--radius-sm);

  &:hover {
    background-color: var(--color-surface2);
  }
}

.calendar-cell-outside {
  color: var(--color-text-faint);
  opacity: 0.4;
}

.calendar-cell-disabled {
  color: var(--color-text-faint);
  opacity: 0.3;
  cursor: default;
  pointer-events: none;
}

.calendar-cell-today {
  font-weight: var(--font-weight-bold);
  color: var(--color-accent1);
  background-color: var(--color-surface2);
}

.calendar-cell-selected {
  background-color: var(--color-accent1);
  color: #241a12;
  font-weight: var(--font-weight-bold);
}
</style>

<script lang="ts">
import moment from 'moment';

export default {
  name: 'CalendarPicker',
  props: {
    // Data attualmente selezionata (YYYY-MM-DD), o null se nessuna.
    selectedDate: { type: String, default: null },
    // Da quale mese partire ad aprire il calendario — se non
    // specificato, usa selectedDate o, in mancanza, oggi.
    initialViewDate: { type: String, default: null },
    // Funzione che dice, per ogni giorno (YYYY-MM-DD), se va
    // disabilitato (grigio, non cliccabile). Di chi usa il
    // componente decide la regola — qui non la conosciamo.
    isDateDisabled: {
      type: Function,
      default: () => false,
    },
  },
  data() {
    return {
      viewMonth: moment(this.initialViewDate || this.selectedDate || undefined),
    };
  },
  computed: {
    weekdayLabels(): string[] {
      return [
        this.$t('home.calendarPicker.weekdayMon') as string,
        this.$t('home.calendarPicker.weekdayTue') as string,
        this.$t('home.calendarPicker.weekdayWed') as string,
        this.$t('home.calendarPicker.weekdayThu') as string,
        this.$t('home.calendarPicker.weekdayFri') as string,
        this.$t('home.calendarPicker.weekdaySat') as string,
        this.$t('home.calendarPicker.weekdaySun') as string,
      ];
    },
    monthLabel(): string {
      const label = this.viewMonth.clone().format('MMMM YYYY');
      return label.charAt(0).toUpperCase() + label.slice(1);
    },
    calendarWeeks() {
      const inizioMese = this.viewMonth.clone().startOf('month');
      const inizioGriglia = inizioMese.clone().startOf('isoWeek');
      const oggi = moment().format('YYYY-MM-DD');
      const giorni = [];
      for (let i = 0; i < 42; i++) {
        const d = inizioGriglia.clone().add(i, 'days');
        const dateStr = d.format('YYYY-MM-DD');
        giorni.push({
          dateStr,
          dayNum: d.date(),
          inMonth: d.month() === this.viewMonth.month(),
          isToday: dateStr === oggi,
          isSelected: dateStr === this.selectedDate,
          disabled: this.isDateDisabled(dateStr),
        });
      }
      const settimane = [];
      for (let i = 0; i < 42; i += 7) {
        settimane.push(giorni.slice(i, i + 7));
      }
      return settimane;
    },
  },
  methods: {
    prevMonth() {
      this.viewMonth = this.viewMonth.clone().subtract(1, 'months');
      this.$emit('month-change', this.viewMonth.format('YYYY-MM'));
    },
    nextMonth() {
      this.viewMonth = this.viewMonth.clone().add(1, 'months');
      this.$emit('month-change', this.viewMonth.format('YYYY-MM'));
    },
  },
};
</script>

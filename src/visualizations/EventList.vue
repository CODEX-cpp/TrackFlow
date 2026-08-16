<template lang="pug">
div
  event-editor(
    v-if="editable"
    :event="editableEvent", :bucket_id="bucket_id",
    @save="(e) => $emit('save', e)", @delete="removeEvent", @close="editableEvent = null"
  )
  div.event-list-card
    div.event-list-head
      h4.event-list-title {{ $t('visualizations.eventList.title') }}
      span.event-list-count
        | {{ $t('visualizations.eventList.showing', { shown: displayed_events.length }) }} #[span(v-if="events.length > displayed_events.length") {{ $t('visualizations.eventList.outOf', { total: events.length }) }}]
      div.pill-btn-ghost.event-list-expand(@click="expandList")
        span(v-if="!isListExpanded") {{ $t('visualizations.eventList.expand') }}
        span(v-else) {{ $t('visualizations.eventList.condense') }}

    div.event-table(v-if="displayed_events.length")
      div.event-table-head(:style="{ gridTemplateColumns: gridTemplate }")
        span {{ $t('visualizations.eventList.time') }}
        span {{ $t('visualizations.eventList.duration') }}
        span(v-for="col in columns" :key="col") {{ col }}
        span
      div.event-table-body.themed-scroll(:class="{ 'event-table-body-expanded': isListExpanded }")
        div.event-table-row(v-for="event in displayed_events" :key="event.id" :style="{ gridTemplateColumns: gridTemplate }")
          span.event-table-time(:title="event.timestamp") {{ event.timestamp | friendlytime }}
          span.event-table-duration {{ event.duration | friendlyduration }}
          span(v-for="col in columns" :key="col")
            span.event-table-pill(v-if="col === primaryKey" :style="{ backgroundColor: pillBg(event), color: pillColor(event) }")
              span.event-table-pill-dot(:style="{ backgroundColor: pillColor(event) }")
              | {{ event.data[col] }}
            span.event-table-text(v-else :title="String(event.data[col])") {{ event.data[col] }}
          span(v-if="editable")
            span.event-table-edit(@click="editEvent(event)") {{ $t('visualizations.eventList.edit') }}
</template>

<style scoped lang="scss">
@import '../style/theme.css';
@import '../style/modals.css';

.event-list-card {
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-xl);
  padding: 16px 20px;
  margin-bottom: 16px;
}

.event-list-head {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 10px;
  margin-bottom: 12px;
}

.event-list-title {
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
  color: var(--color-text);
  margin: 0;
}

.event-list-count {
  font-size: var(--font-size-sm);
  color: var(--color-text-faint);
}

.event-list-expand {
  margin-left: auto;
}

.event-table-head {
  display: grid;
  gap: 10px;
  padding: 0 10px 6px;
  font-size: var(--font-size-xs);
  color: var(--color-text-faint);
  border-bottom: 1px solid var(--color-border);
}

// Altezza fissa (richiesta esplicita) — con molti eventi la tabella
// scorre al suo interno invece di allungare tutta la pagina.
// L'espansione ("espandi"/"condensa", già esistente) resta l'unico modo
// per vederla tutta in una volta.
.event-table-body {
  height: 20em;
  overflow-y: auto;
}

.event-table-body-expanded {
  height: 100%;
  max-height: 60vh;
}

.event-table-row {
  display: grid;
  gap: 10px;
  align-items: center;
  padding: 7px 10px;
  font-size: var(--font-size-sm);
  color: var(--color-text-dim);
  border-bottom: 1px solid var(--color-border);
}

.event-table-row:last-child {
  border-bottom: none;
}

.event-table-row:hover {
  background-color: var(--color-surface2);
}

.event-table-time {
  color: var(--color-text-faint);
  white-space: nowrap;
}

.event-table-duration {
  color: var(--color-text-faint);
  white-space: nowrap;
}

.event-table-text {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.event-table-pill {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 2px 9px;
  border-radius: var(--radius-pill);
  font-size: var(--font-size-xs);
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.event-table-pill-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  flex-shrink: 0;
}

.event-table-edit {
  color: var(--color-accent1);
  font-size: var(--font-size-xs);
  cursor: pointer;
  white-space: nowrap;
}

.event-table-edit:hover {
  text-decoration: underline;
}
</style>

<script lang="ts">
import Color from 'color';
import { getColorFromString, getPrimaryDataKey } from '~/util/color';
import EventEditor from '~/components/EventEditor.vue';

export default {
  name: 'EventList',
  components: {
    'event-editor': EventEditor,
  },
  props: {
    bucket_id: String,
    // Serve solo a capire quale campo dei dati è quello "principale"
    // (per la pillola colorata) — vedi getPrimaryDataKey. Se non
    // passato, si comporta come se il bucket non avesse un type noto
    // (fallback generico, va bene anche per i watcher personalizzati).
    bucket: { type: Object, default: () => ({}) },
    events: Array,
    editable: {
      default: false,
      type: Boolean,
    },
  },
  data: function () {
    return {
      isListExpanded: false,
      limit: 100,
      editableEvent: null,
    };
  },
  computed: {
    displayed_events: function () {
      return this.events.slice(0, this.limit);
    },
    // Le colonne dei dati sono le stesse per ogni evento di un bucket
    // (stesso script/watcher) — basta guardare il primo evento mostrato
    // invece di ricalcolarle per ognuno.
    columns(): string[] {
      const first = this.displayed_events[0];
      return first?.data ? Object.keys(first.data) : [];
    },
    primaryKey(): string | null {
      const first = this.displayed_events[0];
      return first ? getPrimaryDataKey(this.bucket, first) : null;
    },
    gridTemplate(): string {
      const secondary = Math.max(this.columns.length - (this.primaryKey ? 1 : 0), 0);
      const parts = ['90px', '64px'];
      this.columns.forEach(col => {
        parts.push(col === this.primaryKey ? '140px' : '1fr');
      });
      if (this.editable) parts.push('60px');
      void secondary;
      return parts.join(' ');
    },
  },
  methods: {
    pillColor(event: any): string {
      return getColorFromString(String(event.data[this.primaryKey as string]));
    },
    pillBg(event: any): string {
      return Color(this.pillColor(event)).alpha(0.15).string();
    },
    editEvent: function (event) {
      this.editableEvent = event;
    },
    expandList: function () {
      this.isListExpanded = !this.isListExpanded;
    },
    removeEvent: function (_event) {
      // FIXME: Illegal mutation of prop, need to propagate upwards or move into vuex.
      //this.events = this.events.filter(e => e.id != event.id);
    },
  },
};
</script>

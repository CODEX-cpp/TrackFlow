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

    ul.event-list(:class="{ 'expand': isListExpanded }")
      li(v-for="event in displayed_events")
        span.event
          span.field(:title="event.timestamp")
            icon(name="calendar")
            | {{ event.timestamp | friendlytime }}
          span.field
            icon(name="clock")
            | {{ event.duration | friendlyduration }}
          span(v-for="(val, key) in event.data").field
            icon(name="tags")
            | {{ key }}: {{ val }}
          span(v-if="editable")
            div.field.event-edit-btn(@click="editEvent(event)")
              icon(name="edit")
              | {{ $t('visualizations.eventList.edit') }}
</template>

<style scoped lang="scss">
@import '../style/theme.css';

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

.event-list {
  list-style-type: none;
  padding: 0;
  border-radius: var(--radius-md);
  height: 25em;
  overflow-y: auto;
  white-space: nowrap;
  margin-bottom: 0;

  li {
    border-bottom: 1px solid var(--color-border);
    padding: 4px 0;

    &:last-child {
      border-bottom: none;
    }
  }

  &.expand {
    height: 100%;
  }
}

.event {
  display: inline-block;
  padding: 0.3em;
  clear: both;
}

.field {
  display: inline-block;
  margin: 0 5px 0 0;
  font-size: var(--font-size-xs);
  padding: 3px 7px;
  background-color: var(--color-surface2);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  color: var(--color-text-dim);

  &:last-child {
    margin-right: 0;
  }
}

.event-edit-btn {
  cursor: pointer;
  padding: 3px 7px;
}

.event-edit-btn:hover {
  color: var(--color-accent1);
}
</style>

<script lang="ts">
import 'vue-awesome/icons/edit';
import 'vue-awesome/icons/tags';
import 'vue-awesome/icons/clock';
import 'vue-awesome/icons/calendar';

import EventEditor from '~/components/EventEditor.vue';

export default {
  name: 'EventList',
  components: {
    'event-editor': EventEditor,
  },
  props: {
    bucket_id: String,
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
  },
  methods: {
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

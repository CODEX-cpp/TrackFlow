<template lang="pug">
div(v-if="event && event.id")
  div.modal-backdrop(@click="close")
  div.edit-modal.event-editor-modal.themed-scroll
    div.edit-modal-title {{ $t('modals.eventEditor.title') }}

    div(v-if="!editedEvent") {{ $t('modals.eventEditor.loading') }}

    div(v-else)
      div.event-editor-row
        span.event-editor-label {{ $t('modals.eventEditor.bucket') }}
        span.event-editor-value {{ bucket_id }}
      div.event-editor-row
        span.event-editor-label {{ $t('modals.eventEditor.id') }}
        span.event-editor-value {{ event.id }}
      div.event-editor-row
        span.event-editor-label {{ $t('modals.eventEditor.start') }}
        datetime(type="datetime" v-model="start")
      div.event-editor-row
        span.event-editor-label {{ $t('modals.eventEditor.end') }}
        datetime(type="datetime" v-model="end")
      div.event-editor-row
        span.event-editor-label {{ $t('modals.eventEditor.duration') }}
        span.event-editor-value {{ editedEvent.duration | friendlyduration }}

      div.event-editor-divider

      div.event-editor-data-row(v-for="(v, k) in editedEvent.data" :key="k")
        input.edit-field.event-editor-key(disabled, :value="k")
        input.edit-field(
          v-if="typeof event.data[k] === typeof true"
          type="checkbox"
          v-model="editedEvent.data[k]"
          style="width: auto"
        )
        input.edit-field(v-if="typeof event.data[k] === typeof 'string'", v-model="editedEvent.data[k]")
        input.edit-field(v-if="typeof event.data[k] === 'number'", v-model.number="editedEvent.data[k]", type="number")

      div.edit-modal-actions
        div.pill-btn-danger(@click="delete_(); close();")
          icon.mr-1(name="trash")
          | {{ $t('modals.eventEditor.delete') }}
        div.pill-btn-ghost(@click="close") {{ $t('modals.eventEditor.cancel') }}
        div.pill-btn(@click="save(); close();") {{ $t('modals.eventEditor.save') }}
</template>

<style lang="scss" scoped>
@import '../style/theme.css';
@import '../style/modals.css';

.event-editor-modal {
  width: 420px;
  max-height: 80vh;
  overflow-y: auto;
}

.event-editor-row {
  display: flex;
  align-items: center;
  padding: 6px 0;
  border-bottom: 1px solid var(--color-border);
  font-size: var(--font-size-sm);
}

.event-editor-row:last-child {
  border-bottom: none;
}

.event-editor-label {
  width: 90px;
  flex-shrink: 0;
  color: var(--color-text-faint);
}

.event-editor-value {
  color: var(--color-text-dim);
  word-break: break-all;
}

.event-editor-divider {
  border-top: 1px solid var(--color-border);
  margin: 14px 0;
}

.event-editor-data-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
  margin-bottom: 8px;
}

.event-editor-key {
  opacity: 0.7;
}
</style>

<script lang="ts">
// This EventEditor can be used to edit events in a specific bucket.
//
// It is used in:
//  - Stopwatch
//  - Bucket viewer
//  - Timeline (on event-click)
//  - Search (soon)

import moment from 'moment';

import 'vue-awesome/icons/trash';

export default {
  name: 'EventEditor',
  props: {
    event: { type: Object },
    bucket_id: { type: String, required: true },
  },
  data() {
    return {
      editedEvent: null,
    };
  },
  computed: {
    start: {
      get: function () {
        return moment(this.editedEvent.timestamp).format();
      },
      set: function (dt) {
        // Duration needs to be set first since otherwise the computed for end will use the new timestamp
        this.editedEvent.duration = moment(this.end).diff(dt, 'seconds');
        this.editedEvent.timestamp = new Date(dt);
      },
    },
    end: {
      get: function () {
        const end = moment(this.editedEvent.timestamp).add(this.editedEvent.duration, 'seconds');
        return end.format();
      },
      set: function (dt) {
        this.editedEvent.duration = moment(dt).diff(this.editedEvent.timestamp, 'seconds');
      },
    },
  },
  watch: {
    async event() {
      await this.getEvent();
    },
  },
  mounted: async function () {
    await this.getEvent();
  },
  methods: {
    async save() {
      // This emit needs to be called first, otherwise it won't occur for some reason
      // FIXME: but what if the replace fails? Then UI will incorrectly think event was replaced?
      this.$emit('save', this.editedEvent);
      await this.$aw.replaceEvent(this.bucket_id, this.editedEvent);
    },
    async delete_() {
      // This emit needs to be called first, otherwise it won't occur for some reason
      // FIXME: but what if the replace fails? Then UI will incorrectly think event was deleted?
      this.$emit('delete', this.event);
      await this.$aw.deleteEvent(this.bucket_id, this.event.id);
    },
    async getEvent() {
      if (this.bucket_id && this.event && this.event.id) {
        this.editedEvent = await this.$aw.getEvent(this.bucket_id, this.event.id);
      } else {
        this.editedEvent = null;
      }
    },
    close() {
      this.$emit('close', this.event);
    },
  },
};
</script>

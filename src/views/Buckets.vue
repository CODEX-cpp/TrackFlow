<template lang="pug">
div
  h3 {{ $t('buckets.title') }}

  b-alert(show)
    | {{ $t('buckets.moreWatchers') }} #[a(href="https://docs.activitywatch.net/en/latest/watchers.html") {{ $t('buckets.docsLink') }}].

  b-table.bucket-table(
    small, hover,
    :items="mainBuckets",
    :fields="fields"
  )
    template(v-slot:cell(id)="data")
      small.text-monospace.bucket-id(:title="data.item.id") {{ data.item.id }}
    template(v-slot:cell(last_updated)="data")
      small(v-if="bucketHasEvents(data.item)", :class="{'text-success': isRecent(data.item.last_updated)}")
        | {{ data.item.last_updated | friendlytime }}
      small.text-muted(v-else) {{ $t('buckets.noEvents') }}
    template(v-slot:cell(actions)="data")
      b-button-group(size="sm")
        b-button(variant="primary", :to="'/buckets/' + data.item.id", :title="$t('buckets.openBucket')")
          icon.d-none.d-md-inline-block.mr-1(name="folder-open")
          | {{ $t('common.open') }}
        b-dropdown.kebab-dropdown(variant="outline-secondary", toggle-class="border-0", size="sm", right, no-caret, boundary="window", :title="$t('common.more')")
          template(v-slot:button-content)
            icon(name="ellipsis-v")
          b-dropdown-item(@click="export_bucket_json(data.item.id)", :title="$t('buckets.exportBucketJson')")
            icon.mr-1(name="download")
            | {{ $t('buckets.exportBucketJson') }}
          b-dropdown-item(@click="export_csv(data.item.id)", :title="$t('buckets.exportEventsCsv')")
            icon.mr-1(name="download")
            | {{ $t('buckets.exportEventsCsv') }}
          b-dropdown-divider
          b-dropdown-item-button(@click="openDeleteBucketModal(data.item.id)", :title="$t('buckets.deleteBucket')", button-class="text-danger")
            icon.mr-1(name="trash")
            | {{ $t('buckets.deleteBucket') }}

  div(v-if="orphanBuckets.length > 0")
    div.d-flex.justify-content-between.align-items-center.mt-4.mb-1
      h5.mb-0 {{ $t('buckets.orphanedTitle') }}
      b-button(size="sm" variant="outline-danger" @click="openDeleteOrphansModal()")
        icon.mr-1(name="trash")
        | {{ $t('buckets.deleteAllOrphaned') }}
    p.small.text-muted {{ $t('buckets.orphanedHelp') }}
    b-table.bucket-table(
      small, hover,
      :items="orphanBuckets",
      :fields="fields"
    )
      template(v-slot:cell(id)="data")
        small.text-monospace.bucket-id(:title="data.item.id") {{ data.item.id }}
      template(v-slot:cell(last_updated)="data")
        small(v-if="bucketHasEvents(data.item)", :class="{'text-success': isRecent(data.item.last_updated)}")
          | {{ data.item.last_updated | friendlytime }}
        small.text-muted(v-else) {{ $t('buckets.noEvents') }}
      template(v-slot:cell(actions)="data")
        b-button-group(size="sm")
          b-button(variant="primary", :to="'/buckets/' + data.item.id", :title="$t('buckets.openBucket')")
            icon.d-none.d-md-inline-block.mr-1(name="folder-open")
            | {{ $t('common.open') }}
          b-dropdown.kebab-dropdown(variant="outline-secondary", toggle-class="border-0", size="sm", right, no-caret, boundary="window", :title="$t('common.more')")
            template(v-slot:button-content)
              icon(name="ellipsis-v")
            b-dropdown-item(@click="export_bucket_json(data.item.id)", :title="$t('buckets.exportBucketJson')")
              icon.mr-1(name="download")
              | {{ $t('buckets.exportBucketJson') }}
            b-dropdown-item(@click="export_csv(data.item.id)", :title="$t('buckets.exportEventsCsv')")
              icon.mr-1(name="download")
              | {{ $t('buckets.exportEventsCsv') }}
            b-dropdown-divider
            b-dropdown-item-button(@click="openDeleteBucketModal(data.item.id)", :title="$t('buckets.deleteBucket')", button-class="text-danger")
              icon.mr-1(name="trash")
              | {{ $t('buckets.deleteBucket') }}

  b-modal(id="delete-modal", :title="$t('buckets.deleteBucketTitle')", centered, hide-footer)
    | {{ $t('buckets.deleteConfirm', { id: delete_bucket_selected }) }}
    br
    br
    b {{ $t('buckets.deletePermanent') }}
    hr
    div.float-right
      b-button.mx-2(@click="$root.$emit('bv::hide::modal','delete-modal')")
        | {{ $t('common.cancel') }}
      b-button(@click="deleteBucket(delete_bucket_selected)", variant="danger")
        | {{ $t('common.confirm') }}

  b-modal(id="delete-orphans-modal", :title="$t('buckets.deleteOrphanedTitle')", centered, hide-footer, @hidden="delete_orphans_selected = null; delete_orphans_error = null")
    template(v-if="delete_orphans_selected")
      | {{ $t('buckets.deleteHostConfirmPrefix') }}
      |
      b {{ $t('buckets.deleteHostConfirmCount', { count: delete_orphans_selected.bucketCount }) }}
      br
      br
      b {{ $t('buckets.deletePermanent') }}
      div.small.text-muted.mt-2(style="max-height: 200px; overflow-y: auto;")
        | {{ $t('buckets.bucketsToDelete') }}
        ul.mb-0
          li(v-for="bucketId in delete_orphans_selected.bucketIds", :key="bucketId")
            code {{ bucketId }}
      b-alert.mt-2(v-if="delete_orphans_error" show variant="danger")
        | {{ delete_orphans_error }}
      hr
      div.float-right
        b-button.mx-2(@click="$root.$emit('bv::hide::modal','delete-orphans-modal')")
          | {{ $t('common.cancel') }}
        b-button(@click="deleteOrphanBuckets()",
                 :disabled="deleting_orphans",
                 variant="danger")
          template(v-if="deleting_orphans")
            | {{ $t('buckets.deleting') }}
          template(v-else)
            | {{ $t('common.confirm') }}

  h4.mt-4 {{ $t('buckets.importExportTitle') }}

  b-card-group.deck
    b-card(:header="$t('buckets.importBuckets')")
      b-alert(v-if="import_error" show variant="danger" dismissible)
        | {{ import_error }}
      b-form-file(v-model="import_file"
                  :placeholder="$t('buckets.importPlaceholder')"
                  :drop-placeholder="$t('buckets.importDrop')")
      div.mt-2(v-if="import_file")
        b-spinner.mr-2(small)
        small.text-muted {{ $t('buckets.importing') }}
      small.d-block.mt-2.text-muted
        | {{ $t('buckets.importHelpNew') }}
    b-card(:header="$t('buckets.exportBuckets')")
      p.small.text-muted {{ $t('buckets.exportHelp') }}
      b-button(@click="export_all_buckets_json()",
               :title="$t('buckets.exportAllJson')",
               variant="outline-secondary")
        icon.mr-1(name="download")
        | {{ $t('buckets.exportAllJson') }}
</template>

<style scoped lang="scss">
.bucket-table {
  table-layout: fixed;
}

::v-deep .bucket-table td {
  vertical-align: middle;
}

::v-deep .bucket-id {
  display: inline-block;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  vertical-align: middle;
}

::v-deep .kebab-dropdown > .btn {
  color: #6c757d;
  background: transparent;
}

::v-deep .kebab-dropdown > .btn:hover,
::v-deep .kebab-dropdown > .btn:focus,
::v-deep .kebab-dropdown.show > .btn {
  background: #f0f1f3;
  color: #212529;
}

.bucket-last-updated {
  color: #666;
}
</style>

<script lang="ts">
import 'vue-awesome/icons/trash';
import 'vue-awesome/icons/download';
import 'vue-awesome/icons/folder-open';
import 'vue-awesome/icons/desktop';
import 'vue-awesome/icons/mobile';
import 'vue-awesome/icons/question';
import 'vue-awesome/icons/exclamation-triangle';
import 'vue-awesome/icons/ellipsis-v';

import _ from 'lodash';
import Papa from 'papaparse';
import moment from 'moment';

import { useBucketsStore } from '~/stores/buckets';
import { downloadFile } from '~/util/export';

export default {
  name: 'Buckets',
  data() {
    return {
      bucketsStore: useBucketsStore(),

      import_file: null,
      import_error: null,
      delete_bucket_selected: null,
      delete_orphans_selected: null,
      deleting_orphans: false,
      delete_orphans_error: null,
    };
  },
  computed: {
    fields() {
      return [
        {
          key: 'id',
          label: this.$t('buckets.bucketId'),
          sortable: true,
          thStyle: { width: '65%' },
        },
        {
          key: 'last_updated',
          label: this.$t('buckets.updated'),
          sortable: true,
          thStyle: { width: '20%' },
        },
        {
          key: 'actions',
          label: '',
          thStyle: { width: '15%' },
          tdClass: 'text-right',
        },
      ];
    },
    // TrackFlow è mono-client: niente più card per dispositivo — tutti i
    // bucket dell'unico host vanno in una lista piatta, tranne quelli
    // "orfani" (hostname unknown, es. bucket legacy) che restano separati
    // così non spariscono silenziosamente.
    mainBuckets() {
      return _.orderBy(
        this.bucketsStore.buckets.filter(b => (b.hostname || b.data.hostname) !== 'unknown'),
        ['id'],
        ['asc']
      );
    },
    orphanBuckets() {
      return _.orderBy(
        this.bucketsStore.buckets.filter(b => (b.hostname || b.data.hostname) === 'unknown'),
        ['id'],
        ['asc']
      );
    },
  },
  watch: {
    import_file: async function (_new_value, _old_value) {
      if (this.import_file != null) {
        try {
          await this.importBuckets(this.import_file);
          this.import_error = null;
        } catch (err) {
          this.import_error = 'Import failed, see aw-server logs for more info';
        }
        await this.bucketsStore.loadBuckets();
        this.import_file = null;
      }
    },
  },
  mounted: async function () {
    await this.bucketsStore.loadBuckets();
  },
  methods: {
    isRecent: function (date) {
      return moment().diff(date) / 1000 < 120;
    },
    bucketHasEvents: function (bucket) {
      return Boolean(bucket && bucket.last_updated);
    },
    openDeleteBucketModal: function (bucketId: string) {
      this.delete_bucket_selected = bucketId;
      this.$root.$emit('bv::show::modal', 'delete-modal');
    },
    deleteBucket: async function (bucketId: string) {
      await this.bucketsStore.deleteBucket({ bucketId });
      this.$root.$emit('bv::hide::modal', 'delete-modal');
    },
    openDeleteOrphansModal: function () {
      this.delete_orphans_selected = {
        bucketCount: this.orphanBuckets.length,
        bucketIds: this.orphanBuckets.map(b => b.id),
      };
      this.$root.$emit('bv::show::modal', 'delete-orphans-modal');
    },
    deleteOrphanBuckets: async function () {
      if (!this.delete_orphans_selected) return;
      this.deleting_orphans = true;
      this.delete_orphans_error = null;
      try {
        await this.bucketsStore.deleteBuckets({
          bucketIds: this.delete_orphans_selected.bucketIds,
        });
        this.$root.$emit('bv::hide::modal', 'delete-orphans-modal');
      } catch (err) {
        this.delete_orphans_error =
          err?.message || 'Deletion failed. Some buckets may not have been deleted.';
      } finally {
        this.deleting_orphans = false;
      }
    },
    importBuckets: async function (importFile) {
      const formData = new FormData();
      formData.append('buckets.json', importFile);
      const headers = { 'Content-Type': 'multipart/form-data' };
      return this.$aw.req.post('/0/import', formData, { headers });
    },

    async export_bucket_json(bucketId: string) {
      const response = await this.$aw.req.get(`/0/buckets/${bucketId}/export`);
      const data = JSON.stringify(response.data, null, 2);
      await downloadFile(`aw-bucket-export-${bucketId}.json`, data, 'application/json');
    },

    async export_all_buckets_json() {
      const response = await this.$aw.req.get('/0/export');
      const data = JSON.stringify(response.data, null, 2);
      await downloadFile('aw-bucket-export.json', data, 'application/json');
    },

    async export_csv(bucketId: string) {
      const bucket = await this.bucketsStore.getBucketWithEvents({ id: bucketId });
      const events = bucket.events;
      const datakeys = events.length > 0 ? Object.keys(events[0].data) : [];
      const columns = ['timestamp', 'duration'].concat(datakeys);
      const data = events.map(e => {
        return Object.assign(
          { timestamp: e.timestamp, duration: e.duration },
          Object.fromEntries(datakeys.map(k => [k, e.data[k]]))
        );
      });
      const csv = Papa.unparse(data, { columns, header: true });
      const filename = `aw-events-export-${bucketId}-${new Date()
        .toISOString()
        .substring(0, 10)}.csv`;
      await downloadFile(filename, csv, 'text/csv');
    },
  },
};
</script>

<template lang="pug">
div.custom-watcher-view
  div(v-if="loading") {{ $t('customModuleWizard.loading') }}
  div(v-else-if="!latestData") {{ $t('customModuleWizard.waitingForData') }}
  table(v-else)
    tr(v-for="(value, key) in latestData" :key="key")
      td.key {{ key }}
      td.value {{ value }}
</template>

<script lang="js">
// Visualizzazione generica per i watcher personalizzati (vedi
// CustomModuleWizard.vue): mostra l'ultimo oggetto "data" ricevuto dal
// bucket del watcher come una semplice lista chiave/valore, senza che
// l'utente debba scrivere HTML/CSS — pensata per la modalità
// semplificata del wizard ("a prova di scemo"). Un modulo HTML vero e
// proprio (per chi vuole un rendering diverso) resta disponibile a parte
// come tipo modulo "Modulo HTML personalizzato" (CustomVisualization.vue).
import { getHomeClient } from '~/util/awclient';

export default {
  name: 'aw-custom-watcher-view',
  props: {
    // ID completo del bucket da mostrare — in modalità semplificata è
    // TrackFlow stesso a costruirlo (custom-watcher-<slug>, vedi
    // CustomModuleWizard.vue), in modalità esperta è quello scelto
    // liberamente dall'utente nel proprio script.
    bucketId: String,
    title: String,
  },
  data() {
    return {
      loading: true,
      latestData: null,
      refreshInterval: null,
    };
  },
  mounted() {
    this.load();
    this.refreshInterval = setInterval(() => this.load(), 30000);
  },
  beforeDestroy() {
    if (this.refreshInterval) clearInterval(this.refreshInterval);
  },
  watch: {
    bucketId() {
      this.load();
    },
  },
  methods: {
    async load() {
      if (!this.bucketId) {
        this.loading = false;
        return;
      }
      const query = [
        `events = sort_by_timestamp(flood(query_bucket("${this.bucketId}")));`,
        'RETURN = events;',
      ];
      try {
        const data = await getHomeClient().query(
          ['2000-01-01T00:00:00.000Z/2100-01-01T00:00:00.000Z'],
          query
        );
        const events = data[0] || [];
        const last = events[events.length - 1];
        this.latestData = last ? last.data : null;
      } catch (e) {
        this.latestData = null;
      } finally {
        this.loading = false;
      }
    },
  },
};
</script>

<style lang="scss" scoped>
.custom-watcher-view {
  font-size: 13px;
  color: var(--color-text-dim);
}
table {
  width: 100%;
}
.key {
  color: var(--color-text-faint);
  padding-right: 12px;
}
.value {
  color: var(--color-text);
  text-align: right;
}
</style>

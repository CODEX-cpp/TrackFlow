<template lang="pug">
div
  div.input-time-interval
    // Stesso identico markup/stile di .zoom-tabs nella Topbar della
    // Home (selettore Giorno/4h/1h della sua timeline) — richiesta
    // esplicita di copiarlo pari pari, invece di riproporre lo stile a
    // pillole separate usato qui prima.
    div.zoom-tabs
      div.zoom-tab(
        v-for="(dur, idx) in durations"
        :key="idx"
        :class="{ 'zoom-tab-active': value === dur.seconds }"
        @click="$emit('input', dur.seconds)"
        v-html="dur.label"
      )
</template>

<style scoped lang="scss">
@import '../style/theme.css';

.input-time-interval {
  margin-bottom: 16px;
}

.zoom-tabs {
  display: inline-flex;
  gap: 4px;
  background-color: var(--color-surface2);
  border-radius: var(--radius-md);
  padding: 3px;
}

.zoom-tab {
  padding: 5px 12px;
  border-radius: var(--radius-sm);
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-semibold);
  color: var(--color-text-faint);
  cursor: pointer;
}

.zoom-tab:hover {
  color: var(--color-text);
}

.zoom-tab-active {
  background-color: var(--color-accent1);
  color: #241a12;
}

.zoom-tab-active:hover {
  color: #241a12;
}
</style>

<script lang="ts">
// Puramente presentazionale: sceglie SOLO la durata (in secondi), niente
// più calcolo/emissione di una finestra [inizio, fine] con moment() al
// suo interno. Prima questo componente calcolava la finestra "adesso -
// durata" da solo e la faceva scorrere con un proprio timer interno —
// il ciclo v-model (emit → watch nel genitore → refetch) per qualche
// motivo non si aggiornava in automatico come previsto (funzionava solo
// il click manuale, mai il timer). Spostata la finestra scorrevole nel
// genitore (Bucket.vue), che la ricalcola col suo stesso setInterval già
// usato e testato per il log del watcher — un solo, semplice canale di
// v-model standard (value: Number, evento input) invece di un
// meccanismo di cache-busting fragile.
export default {
  name: 'input-timeinterval',
  props: {
    value: { type: Number, required: true },
  },
  computed: {
    // Solo 3 opzioni, stessi livelli di zoom della timeline della Home
    // (PX_PER_MINUTE in HomeTimelineSection.vue: 'day' | '4h' | '1h') —
    // richiesta esplicita, la lista precedente (da ¼h a 48h) aveva
    // troppe voci.
    durations() {
      return [
        { seconds: 60 * 60, label: '1h' },
        { seconds: 4 * 60 * 60, label: '4h' },
        { seconds: 24 * 60 * 60, label: this.$t('modals.inputTimeInterval.day') },
      ];
    },
  },
};
</script>

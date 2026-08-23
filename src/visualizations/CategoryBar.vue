<template lang="pug">
div.category-bar
  div.category-bar-empty(v-if="segments.length === 0") {{ $t('visualizations.topSummary.noData') }}
  template(v-else)
    div.category-bar-track
      div.category-bar-segment(
        v-for="seg in segments"
        :key="seg.name"
        :style="{ width: seg.percent + '%', backgroundColor: seg.color }"
        :title="seg.name + ' — ' + seg.durationLabel"
      )
    div.category-bar-legend
      div.category-bar-row(v-for="seg in segments" :key="'legend-' + seg.name")
        div.category-bar-row-left
          span.category-bar-dot(:style="{ backgroundColor: seg.color }")
          span.category-bar-name {{ seg.name }}
        span.category-bar-value {{ seg.durationLabel }} · {{ seg.percentLabel }}%
</template>

<style lang="scss" scoped>
@import '../style/theme.css';

.category-bar-empty {
  font-size: var(--font-size-sm);
  color: var(--color-text-faint);
}

.category-bar-track {
  display: flex;
  width: 100%;
  height: 10px;
  border-radius: var(--radius-sm);
  overflow: hidden;
  background-color: var(--color-surface2);
}

.category-bar-segment {
  height: 100%;
  // Same reasoning as .top-summary-bar (TopSummary.vue): the DOM node
  // is reused across re-renders for the same category, so animating
  // width here turns a hard value snap into a smooth glide when the
  // underlying data changes (e.g. switching day in the topbar).
  transition: width 0.35s ease, background-color 0.3s ease;
}

.category-bar-legend {
  display: flex;
  flex-direction: column;
  gap: 9px;
  margin-top: 14px;
}

.category-bar-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.category-bar-row-left {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.category-bar-dot {
  width: 10px;
  height: 10px;
  border-radius: 3px;
  flex-shrink: 0;
}

.category-bar-name {
  font-size: var(--font-size-sm);
  color: var(--color-text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.category-bar-value {
  flex-shrink: 0;
  font-size: var(--font-size-sm);
  color: var(--color-text-faint);
  font-variant-numeric: tabular-nums;
}
</style>

<script lang="ts">
// Barra impilata al 100% per il tempo speso per categoria app durante il
// periodo mostrato — vedi BLUEPRINT.md sezione 17 per il design completo
// (confrontato con ManicTime/RescueTime/Toggl/Timing prima di scegliere
// questo formato). Riceve gli eventi "app" già filtrati dal genitore
// (SelectableVisualization.vue's top_apps_filtered — stessa fonte di Top
// Applicazioni, niente doppia interrogazione dei dati) e fa da sola tutto
// il lavoro categoria→durata, non serve altro dal chiamante.
import { useAppCategoriesStore } from '~/stores/appCategories';
import { formatHoursMinutes } from '~/util/projectTime';

// Soglia sotto la quale una categoria (non "Non categorizzato", vedi
// sotto) finisce accorpata in "Altro" invece di occupare una riga a sé
// — stessa idea di Toggl Track nei suoi report a torta (ricerca fatta
// con l'utente prima di implementare), pensata per evitare che la
// legenda si riempia di categorie che pesano pochi minuti.
const ALTRO_THRESHOLD_PERCENT = 5;

interface Segment {
  name: string;
  duration: number;
  percent: number;
  percentLabel: number;
  durationLabel: string;
  color: string;
}

export default {
  name: 'aw-category-bar',
  props: {
    apps: { type: Array, default: () => [] },
  },
  computed: {
    categorieStore() {
      return useAppCategoriesStore();
    },
    segments(): Segment[] {
      const totals = new Map<string, number>();
      // "Non categorizzato" tenuto SEPARATO dalle categorie vere, non
      // dentro la stessa mappa con una chiave sentinella — a differenza
      // delle categorie sotto soglia, resta sempre visibile come riga a
      // sé (richiesta esplicita dell'utente: è un segnale utile — "vai a
      // categorizzare di più" — non rumore da nascondere), quindi non
      // deve mai passare per il ciclo di accorpamento in "Altro" sotto.
      let nonCategorizzato = 0;
      for (const e of this.apps as any[]) {
        const app = e.data && e.data.app;
        if (!app) continue;
        const durata = e.duration || 0;
        const categoria = this.categorieStore.categoryForApp(app);
        if (categoria) {
          totals.set(categoria, (totals.get(categoria) || 0) + durata);
        } else {
          nonCategorizzato += durata;
        }
      }

      const totale =
        [...totals.values()].reduce((a, b) => a + b, 0) + nonCategorizzato;
      if (totale <= 0) return [];

      const ordinate = [...totals.entries()].sort((a, b) => b[1] - a[1]);
      const principali: [string, number][] = [];
      let altro = 0;
      for (const [nome, durata] of ordinate) {
        if ((durata / totale) * 100 >= ALTRO_THRESHOLD_PERCENT) {
          principali.push([nome, durata]);
        } else {
          altro += durata;
        }
      }

      const build = (name: string, durata: number, color: string): Segment => ({
        name,
        duration: durata,
        percent: (durata / totale) * 100,
        percentLabel: Math.round((durata / totale) * 100),
        durationLabel: formatHoursMinutes(durata),
        color,
      });

      const risultato = principali.map(([nome, durata]) => build(nome, durata, this.categorieStore.colorForCategoryName(nome)));
      if (altro > 0) {
        risultato.push(
          build(this.$t('visualizations.categoryBar.other') as string, altro, 'var(--color-text-dim)')
        );
      }
      if (nonCategorizzato > 0) {
        risultato.push(
          build(
            this.$t('visualizations.categoryBar.uncategorized') as string,
            nonCategorizzato,
            'var(--color-text-faint)'
          )
        );
      }
      return risultato;
    },
  },
};
</script>

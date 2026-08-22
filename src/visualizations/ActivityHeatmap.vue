<template lang="pug">
div.activity-heatmap
  div.activity-heatmap-empty(v-if="!loading && totalSeconds === 0") {{ $t('visualizations.activityHeatmap.noData') }}
  template(v-else)
    div.activity-heatmap-body
      div.activity-heatmap-daylabels
        span.activity-heatmap-daylabel(v-for="d in dayLabels" :key="d") {{ d }}
      div.activity-heatmap-main
        div.activity-heatmap-months
          span.activity-heatmap-month(
            v-for="m in monthLabels"
            :key="m.key"
            :style="{ gridColumnStart: m.col + 1 }"
          ) {{ m.label }}
        div.activity-heatmap-grid
          template(v-for="(week, wi) in weeks")
            div.activity-heatmap-cell(
              v-if="!cell.futuro"
              v-for="cell in week"
              :key="cell.date"
              :class="'activity-heatmap-level-' + cell.level"
              :title="cell.tooltip"
            )
    div.activity-heatmap-legend
      span {{ $t('visualizations.activityHeatmap.less') }}
      div.activity-heatmap-legend-cell(v-for="lvl in [0, 1, 2, 3, 4]" :key="lvl" :class="'activity-heatmap-level-' + lvl")
      span {{ $t('visualizations.activityHeatmap.more') }}
</template>

<style lang="scss" scoped>
@import '../style/theme.css';

// Dimensione fissa, non elastica (un tentativo precedente di farla
// crescere elasticamente con 1fr, per riempire tutta la larghezza della
// card, produceva quadratini sproporzionati). Con il modulo a 2 colonne
// e 6 mesi di dati (182 giorni ≈ 27 settimane/colonne), 18px è il valore
// che riempie quasi per intero la larghezza disponibile della card senza
// andare in overflow.
$dimensione-quadratino: 18px;

.activity-heatmap-empty {
  font-size: var(--font-size-sm);
  color: var(--color-text-faint);
}

.activity-heatmap-months {
  display: grid;
  grid-auto-flow: column;
  // Stessa larghezza colonna E stesso gap di .activity-heatmap-grid
  // sotto — le etichette dei mesi devono allinearsi alla settimana
  // corrispondente. Senza lo stesso gap, le colonne di questa griglia
  // sono più strette (14px vs 17px) e lo scarto si accumula colonna
  // dopo colonna, spostando le etichette sempre più a sinistra rispetto
  // ai quadratini man mano che si avanza (bug riscontrato: "ago"
  // arrivava spostato di 72px).
  grid-auto-columns: $dimensione-quadratino;
  gap: 3px;
  margin-bottom: 4px;
}

.activity-heatmap-month {
  font-size: var(--font-size-sm);
  color: var(--color-text-faint);
  white-space: nowrap;
}

.activity-heatmap-body {
  display: flex;
  gap: 4px;
  align-items: flex-start;
}

.activity-heatmap-daylabels {
  display: flex;
  flex-direction: column;
  gap: 3px;
  width: 26px;
  flex-shrink: 0;
  // Sposta le etichette Lun/Mer/Ven in basso di un rigo — allinea con
  // .activity-heatmap-months sopra al resto (che occupa lo stesso spazio
  // sopra la griglia vera e propria).
  margin-top: 18px;
}

.activity-heatmap-daylabel {
  height: $dimensione-quadratino;
  font-size: var(--font-size-sm);
  color: var(--color-text-faint);
  line-height: $dimensione-quadratino;
}

.activity-heatmap-main {
  // flex-shrink:0 qui (non flex:1) — non deve crescere per riempire
  // .activity-heatmap-body, la griglia sotto ha già la sua larghezza
  // fissa (vedi $dimensione-quadratino sopra); niente sinistra vuota
  // extra da questo contenitore, solo dal margine naturale della card.
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
}

.activity-heatmap-grid {
  display: grid;
  grid-auto-flow: column;
  grid-auto-columns: $dimensione-quadratino;
  grid-template-rows: repeat(7, $dimensione-quadratino);
  gap: 3px;
}

.activity-heatmap-cell {
  width: $dimensione-quadratino;
  height: $dimensione-quadratino;
  border-radius: 2px;
  background-color: var(--color-surface2);
}

.activity-heatmap-legend-cell {
  width: $dimensione-quadratino;
  height: $dimensione-quadratino;
  flex-shrink: 0;
  border-radius: 2px;
  background-color: var(--color-surface2);
}

// Livello 0 riusa lo sfondo neutro sopra (nessuna attività quel giorno).
// 1-4 sono la stessa tinta d'accento del tema a opacità crescente, non
// il verde fisso di GitHub — resta coerente con temi chiaro/scuro
// diversi invece di stonare su uno dei due.
.activity-heatmap-level-1 {
  background-color: color-mix(in srgb, var(--color-accent1) 30%, var(--color-surface2));
}
.activity-heatmap-level-2 {
  background-color: color-mix(in srgb, var(--color-accent1) 55%, var(--color-surface2));
}
.activity-heatmap-level-3 {
  background-color: color-mix(in srgb, var(--color-accent1) 78%, var(--color-surface2));
}
.activity-heatmap-level-4 {
  background-color: var(--color-accent1);
}

.activity-heatmap-legend {
  display: flex;
  align-items: center;
  gap: 4px;
  margin-top: 8px;
  font-size: var(--font-size-sm);
  color: var(--color-text-faint);
}
</style>

<script lang="ts">
import { getHomeClient } from '~/util/awclient';
import { formatHoursMinutes } from '~/util/projectTime';
import { eventListSignature } from '~/util/timelineBlocks';
import { useSettingsStore } from '~/stores/settings';
import moment from 'moment';

// Finestra fissa, indipendente dal periodo scelto nella Topbar (un
// grafico "contributi" per un solo giorno o una sola settimana non
// avrebbe senso) — 6 mesi, non un anno intero come l'originale GitHub:
// resta leggero da interrogare e più utile per pattern recenti in
// un'app di tracciamento lavoro. Numero scelto in coppia con la
// larghezza del modulo (2 colonne, vedi DOUBLE_TYPES in
// HomeModulesSection.vue) — è la combinazione che riempie la card senza
// lasciarla vuota a destra né farla traboccare.
const GIORNI_FINESTRA = 182;

interface Cella {
  date: string;
  level: number;
  tooltip: string;
  // Solo i giorni dopo "oggi" (sempre in coda all'ultima settimana, mai
  // altrove) — il quadratino non viene proprio disegnato (v-if nel
  // template), a differenza dei giorni "senza dati" (prima del primo
  // evento tracciato) che restano visibili col colore neutro per non
  // rompere l'allineamento a colonne della griglia.
  futuro: boolean;
}

export default {
  name: 'aw-activity-heatmap',
  data() {
    return {
      loading: true,
      // Minuti attivi (non-afk) per giorno, chiave YYYY-MM-DD.
      secondiPerGiorno: {} as Record<string, number>,
      // Giorno del primo evento MAI registrato su questo bucket (qualsiasi
      // status, non solo not-afk) — prima di questa data il watcher AFK
      // non esisteva ancora, quindi "nessun dato" non significa "0 ore
      // lavorate": vedi weeks() sotto, dove questi giorni sono trattati
      // come i giorni futuri (livello -1, neutro, nessun tooltip) invece
      // di essere colorati come zero confermato.
      primoGiornoTracciato: null as string | null,
      lastSignature: null as string | null,
      refreshInterval: null as ReturnType<typeof setInterval> | null,
      settingsStore: useSettingsStore(),
    };
  },
  computed: {
    totalSeconds(): number {
      return Object.values(this.secondiPerGiorno).reduce((a: number, b: number) => a + b, 0);
    },
    // Obiettivo giornaliero (Impostazioni → Home → "Ore di lavoro stimate
    // al giorno", vicino alla pausa pranzo) — denominatore del colore
    // sotto, al posto del giorno più attivo del semestre. Il tempo
    // non-AFK già esclude le pause per definizione, quindi non serve
    // sottrarre nulla: è già "ore lavorative corrette".
    budgetSecondi(): number {
      const ore = this.settingsStore.dailyWorkHoursBudget;
      return Number.isFinite(ore) && ore > 0 ? ore * 3600 : 8 * 3600;
    },
    dayLabels(): string[] {
      // Solo Lun/Mer/Ven mostrate (come GitHub) — le altre restano vuote
      // per non affollare una colonna larga solo 26px.
      return [
        this.$t('home.calendarPicker.weekdayMon') as string,
        '',
        this.$t('home.calendarPicker.weekdayWed') as string,
        '',
        this.$t('home.calendarPicker.weekdayFri') as string,
        '',
        '',
      ];
    },
    // Griglia settimane×giorni, Lunedì come primo giorno (coerente con
    // CalendarPicker.vue) — la finestra di GIORNI_FINESTRA giorni viene
    // estesa all'indietro fino all'inizio della sua settimana e in
    // avanti fino a oggi, così ogni colonna è una settimana intera.
    weeks(): Cella[][] {
      const oggi = moment().startOf('day');
      const inizioFinestra = oggi.clone().subtract(GIORNI_FINESTRA - 1, 'days');
      const inizioGriglia = inizioFinestra.clone().startOf('isoWeek');

      const budgetSecondi = this.budgetSecondi;

      const livelloPer = (secondi: number): number => {
        if (secondi <= 0) return 0;
        const rapporto = secondi / budgetSecondi;
        if (rapporto <= 0.25) return 1;
        if (rapporto <= 0.5) return 2;
        if (rapporto <= 0.75) return 3;
        return 4;
      };

      const settimane: Cella[][] = [];
      let cursore = inizioGriglia.clone();
      while (cursore.isSameOrBefore(oggi, 'day')) {
        const settimana: Cella[] = [];
        for (let i = 0; i < 7; i++) {
          const chiave = cursore.format('YYYY-MM-DD');
          const secondi = this.secondiPerGiorno[chiave] || 0;
          const nelFuturo = cursore.isAfter(oggi, 'day');
          // Prima del primo evento mai registrato il watcher non esisteva
          // ancora — "nessun dato" qui non significa "0 ore lavorate",
          // quindi va trattato come i giorni futuri (neutro, non un vero
          // zero), non colorato a livello 0 come se fosse un giorno
          // tracciato ma inattivo.
          const primaDiTracciamento =
            this.primoGiornoTracciato !== null && chiave < this.primoGiornoTracciato;
          const senzaDati = nelFuturo || primaDiTracciamento;
          settimana.push({
            date: chiave,
            level: senzaDati ? -1 : livelloPer(secondi),
            tooltip: senzaDati ? '' : `${cursore.format('DD/MM/YYYY')} — ${formatHoursMinutes(secondi)}`,
            futuro: nelFuturo,
          });
          cursore = cursore.clone().add(1, 'day');
        }
        settimane.push(settimana);
      }
      // Le celle nel futuro E quelle precedenti al primo evento mai
      // tracciato restano nella griglia per non spezzare l'allineamento a
      // colonne, ma con livello -1: stessa classe del vuoto (nessun -1
      // dedicato in CSS, il selettore .activity-heatmap-level--1
      // semplicemente non esiste, quindi ricadono nel colore base) e
      // senza tooltip — in entrambi i casi "non sappiamo", non "zero".
      return settimane;
    },
    monthLabels(): { key: string; label: string; col: number }[] {
      const grezze: { key: string; label: string; col: number }[] = [];
      let ultimoMese = '';
      this.weeks.forEach((settimana, col) => {
        const primoGiorno = settimana[0].date;
        const mese = primoGiorno.slice(0, 7);
        if (mese !== ultimoMese) {
          ultimoMese = mese;
          grezze.push({ key: mese, label: moment(primoGiorno).format('MMM'), col });
        }
      });
      // Un mese che occupa solo 1 colonna a inizio finestra (es. gli
      // ultimi giorni di maggio prima che inizi giugno) non ha spazio
      // per la sua etichetta prima di quella del mese successivo — si
      // sovrappongono. Stessa soluzione di GitHub: salta l'etichetta se
      // il mese successivo comincia meno di 2 colonne dopo.
      // Soglia 3 (non 2): ogni colonna è larga solo 13px, un'etichetta
      // di 3 lettere ("May") ci sta a malapena in 2 colonne piene e
      // finisce comunque per toccare la successiva.
      return grezze.filter((m, i) => {
        const prossimo = grezze[i + 1];
        return !prossimo || prossimo.col - m.col >= 3;
      });
    },
  },
  mounted: async function () {
    await this.carica();
    this.refreshInterval = setInterval(() => this.carica(), 60000);
  },
  beforeDestroy: function () {
    if (this.refreshInterval) clearInterval(this.refreshInterval);
  },
  methods: {
    carica: async function () {
      const fine = moment().endOf('day');
      const inizio = moment().subtract(GIORNI_FINESTRA - 1, 'days').startOf('day');

      let eventi = [];
      try {
        // Client dedicato — vedi getHomeClient() per il motivo (quello
        // condiviso $aw annulla le richieste in corso ad ogni cambio di
        // data/periodo della vecchia pagina Activity).
        eventi = await getHomeClient().getEvents('aw-watcher-afk', {
          start: inizio.toDate(),
          end: fine.toDate(),
          limit: -1,
        });
      } catch (e) {
        // Bucket assente (watcher mai partito) — nessun dato, non un errore.
        eventi = [];
      }

      const signature = eventListSignature([eventi]);
      if (signature === this.lastSignature) {
        this.loading = false;
        return;
      }
      this.lastSignature = signature;

      const perGiorno: Record<string, number> = {};
      let primo: string | null = null;
      for (const ev of eventi as any[]) {
        const chiave = moment(ev.timestamp).format('YYYY-MM-DD');
        // Qualsiasi evento (anche 'afk') conta per "il watcher era vivo
        // quel giorno" — non solo i not-afk sommati sotto.
        if (primo === null || chiave < primo) primo = chiave;
        if (ev.data?.status !== 'not-afk') continue;
        perGiorno[chiave] = (perGiorno[chiave] || 0) + (ev.duration || 0);
      }
      this.secondiPerGiorno = perGiorno;
      this.primoGiornoTracciato = primo;
      this.loading = false;
    },
  },
};
</script>

// Shared "which day, which host, fetch these buckets safely" plumbing
// for the Home-page widgets that read raw ActivityWatch events (the
// timeline and the summary panels) — both need the exact same date/
// host resolution from the route and the same "bucket might not exist"
// handling, so it lives here once instead of twice.
import Vue from 'vue';
import moment from 'moment';
import { getHomeClient } from '~/util/awclient';
import { useBucketsStore } from '~/stores/buckets';
import { useSettingsStore } from '~/stores/settings';
import { get_today_with_offset, get_day_start_with_offset } from '~/util/time';

export default Vue.extend({
  computed: {
    // Synced with whatever date Topbar's calendar has selected — reads
    // the /activity/:periodLength?/:date? route param. Fallback is
    // offset-aware ("inizio giornata" nelle Impostazioni, es. 04:00) —
    // deve restare lo stesso identico calcolo di Topbar.vue e
    // HomeModulesSection.vue, altrimenti tra mezzanotte e l'orario di
    // inizio giornata Timeline e Moduli finiscono per mostrare due
    // giorni diversi (bug segnalato dall'utente: moduli vuoti dopo un
    // giro ieri->oggi mentre la Timeline mostrava correttamente i dati
    // di oggi).
    date(): string {
      return (
        (this.$route.params.date as string) ||
        get_today_with_offset(useSettingsStore().startOfDay)
      );
    },
    // TrackFlow is single-device (mono-client) — the host is never a route
    // param, it's always resolved from the buckets store.
    host(): string {
      return useBucketsStore().host;
    },
    // Offset-aware confine ("inizio giornata" nelle Impostazioni, es.
    // 04:00) — deve corrispondere esattamente al `timeperiod` calcolato
    // da HomeModulesSection.vue (get_day_start_with_offset). Prima
    // usava mezzanotte-a-mezzanotte del calendario: con `date` ora
    // allineato all'offset (vedi sopra), lasciare qui mezzanotte pura
    // avrebbe tagliato fuori proprio le ore appena passate (00:00 -
    // inizio giornata) che sono la ragione per cui l'offset esiste.
    dayStart(): moment.Moment {
      return moment(get_day_start_with_offset(this.date, useSettingsStore().startOfDay));
    },
    dayEnd(): moment.Moment {
      return this.dayStart.clone().add(1, 'day');
    },
  },
  methods: {
    async fetchEvents(bucketId: string, start: moment.Moment, end: moment.Moment): Promise<any[]> {
      try {
        // Uses a client detached from the shared $aw instance — see
        // getHomeClient() for why (the old Activity page's date/period
        // changes abort() the shared client wholesale).
        return await getHomeClient().getEvents(bucketId, {
          start: start.toDate(),
          end: end.toDate(),
          limit: -1,
        });
      } catch {
        // Bucket doesn't exist on this server — empty, not an error.
        return [];
      }
    },
    // Browser watcher buckets aren't per-host yet (there can be one
    // per browser, none of them guaranteed to exist) — try each known
    // id and use whichever actually has events, instead of guessing.
    async fetchFirstAvailable(
      bucketIds: string[],
      start: moment.Moment,
      end: moment.Moment
    ): Promise<any[]> {
      for (const id of bucketIds) {
        const events = await this.fetchEvents(id, start, end);
        if (events.length > 0) return events;
      }
      return [];
    },
  },
});

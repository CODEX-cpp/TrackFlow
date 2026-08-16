<template lang="pug">
div.bucket-page
  router-link.pill-btn-ghost.back-btn(to="/buckets")
    icon.mr-1(name="arrow-left")
    | {{ $t('buckets.title') }}

  div.page-title-row
    div.page-title {{ id }}
    div.pill-btn-ghost(v-if="customWatcherInfo" @click="apriCartellaWatcher")
      icon.mr-1(name="folder-open")
      | {{ $t('visualizations.bucketPage.openFolder') }}

  div.bucket-info-card
    div.bucket-info-row
      span.bucket-info-label {{ $t('visualizations.bucketPage.type') }}
      span.bucket-info-value {{ bucket.type }}
    div.bucket-info-row
      span.bucket-info-label {{ $t('visualizations.bucketPage.client') }}
      span.bucket-info-value {{ bucket.client }}
    div.bucket-info-row
      span.bucket-info-label {{ $t('visualizations.bucketPage.hostname') }}
      span.bucket-info-value {{ bucket.hostname }}
    div.bucket-info-row
      span.bucket-info-label {{ $t('visualizations.bucketPage.created') }}
      span.bucket-info-value {{ bucket.created | datait }}
    div.bucket-info-row(v-if="bucket.metadata")
      span.bucket-info-label {{ $t('visualizations.bucketPage.firstLastEvent') }}
      span.bucket-info-value {{ bucket.metadata.start | datait }} / {{ bucket.metadata.end | datait }}
    div.bucket-info-row
      span.bucket-info-label {{ $t('visualizations.bucketPage.eventCount') }}
      span.bucket-info-value {{ eventcount }}
    div.bucket-info-row(v-if="totalDurationSeconds !== null")
      span.bucket-info-label {{ $t('visualizations.bucketPage.totalDuration') }}
      span.bucket-info-value {{ totalDurationSeconds | friendlyduration }}
    template(v-if="customWatcherInfo")
      div.bucket-info-row
        span.bucket-info-label {{ $t('visualizations.bucketPage.processStatus') }}
        span.bucket-info-value {{ customWatcherInfo.running ? $t('visualizations.bucketPage.processRunning') : $t('visualizations.bucketPage.processStopped') }}
      div.bucket-info-row
        span.bucket-info-label {{ $t('visualizations.bucketPage.mode') }}
        span.bucket-info-value {{ customWatcherInfo.mode === 'interval' ? $t('visualizations.bucketPage.modeInterval') : $t('visualizations.bucketPage.modeRaw') }}
      div.bucket-info-row(v-if="customWatcherInfo.mode === 'interval'")
        span.bucket-info-label {{ $t('visualizations.bucketPage.pollInterval') }}
        span.bucket-info-value {{ intervalloPollingMinuti }} min
      div.bucket-info-row(v-if="customWatcherInfo.mode === 'interval'")
        span.bucket-info-label {{ $t('visualizations.bucketPage.scriptFile') }}
        input.edit-field.file-associato-input(
          v-model="fileAssociatoInput"
          @blur="salvaFileAssociato"
          @keyup.enter="$event.target.blur()"
          :title="$t('visualizations.bucketPage.scriptFileHint')"
        )
      div.bucket-info-row(v-else)
        span.bucket-info-label {{ $t('visualizations.bucketPage.command') }}
        span.bucket-info-value {{ comandoCompleto }}
      div.bucket-info-row
        span.bucket-info-label {{ $t('visualizations.bucketPage.separateTimeline') }}
        div.settings-toggle(:class="{ 'settings-toggle-on': timelineLaneToggle }" @click="timelineLaneToggle = !timelineLaneToggle")
          div.settings-toggle-thumb
      div.bucket-info-row
        span.bucket-info-label {{ $t('visualizations.bucketPage.excludeFromModules') }}
        div.settings-toggle(:class="{ 'settings-toggle-on': excludeFromModulesToggle }" @click="excludeFromModulesToggle = !excludeFromModulesToggle")
          div.settings-toggle-thumb

  input-timeinterval(v-model="daterange", :maxDuration="maxDuration")

  vis-timeline(:buckets="[bucket_with_events]", :showRowLabels="false", :queriedInterval="daterange", :updateTimelineWindow="true")

  aw-eventlist(:bucket_id="id", @save="updateEvent", :events="events" editable=true)

  div.log-panel(v-if="customWatcherInfo")
    div.log-panel-title {{ $t('visualizations.bucketPage.logTitle') }}
    pre.log-panel-content.themed-scroll(ref="logContent") {{ watcherLogContent || $t('visualizations.bucketPage.logEmpty') }}
</template>

<style lang="scss" scoped>
@import '../style/theme.css';
@import '../style/modals.css';

.bucket-page {
  padding: 24px 28px;
}

// Nessuno stile proprio qui: .pill-btn-ghost (modals.css, importato
// sopra) è lo stesso stile/hover di tutti gli altri pulsanti "ghost"
// dell'app (es. "Apri cartella watcher" più sotto in questa stessa
// pagina) — .back-btn resta solo per il margine sotto il pulsante.
.back-btn {
  display: inline-block;
  margin-bottom: 16px;
}

.page-title-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 16px;
}

.page-title {
  font-family: ui-monospace, SFMono-Regular, 'SF Mono', Menlo, Consolas, monospace;
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-bold);
  color: var(--color-text);
  word-break: break-all;
}

.bucket-info-card {
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-xl);
  padding: 16px 20px;
  margin-bottom: 20px;
}

.bucket-info-row {
  display: flex;
  align-items: center;
  padding: 6px 0;
  border-bottom: 1px solid var(--color-border);
  font-size: var(--font-size-sm);
}

.bucket-info-row:last-child {
  border-bottom: none;
}

.bucket-info-label {
  width: 160px;
  flex-shrink: 0;
  color: var(--color-text-faint);
}

.bucket-info-value {
  color: var(--color-text-dim);
  word-break: break-all;
}

.settings-toggle {
  width: 36px;
  height: 20px;
  border-radius: var(--radius-pill);
  background-color: var(--color-surface2);
  border: 1px solid var(--color-border);
  position: relative;
  cursor: pointer;
  flex-shrink: 0;
}

.settings-toggle-thumb {
  position: absolute;
  top: 1px;
  left: 1px;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background-color: var(--color-text-faint);
  transition: left 0.15s ease, background-color 0.15s ease;
}

.settings-toggle-on {
  background-color: var(--color-accent1);
}

.settings-toggle-on .settings-toggle-thumb {
  left: 17px;
  background-color: #241a12;
}

.file-associato-input {
  width: auto;
  max-width: 280px;
  padding: 4px 8px;
}

.log-panel {
  margin-top: 24px;
}

.log-panel-title {
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-semibold);
  color: var(--color-text-faint);
  margin-bottom: 8px;
}

.log-panel-content {
  background-color: var(--color-bg);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  padding: 12px 14px;
  margin: 0;
  font-family: ui-monospace, SFMono-Regular, 'SF Mono', Menlo, Consolas, monospace;
  font-size: var(--font-size-xs);
  line-height: 1.6;
  color: var(--color-text-dim);
  height: 600px;
  overflow-y: auto;
  white-space: pre-wrap;
  word-break: break-all;
}
</style>

<script lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { useBucketsStore } from '~/stores/buckets';
import { getClient } from '~/util/awclient';
import 'vue-awesome/icons/arrow-left';
import 'vue-awesome/icons/folder-open';

export default {
  name: 'Bucket',
  props: {
    id: String,
  },
  data: () => {
    return {
      bucketsStore: useBucketsStore(),

      events: [],
      eventcount: '?',
      daterange: null,
      maxDuration: 31 * 24 * 60 * 60,
      totalDurationSeconds: null,
      // Popolato solo se questo bucket appartiene a un watcher
      // personalizzato (match su bucket_id) — mostra la sezione
      // "informazioni sviluppatore" solo in quel caso, resta null per i
      // bucket dei moduli integrati.
      customWatcherInfo: null,
      // TEMPORANEO — solo UI, richiesta esplicita: "per ora mettili e
      // basta poi li collegheremo". Non letti/scritti da nessuna parte,
      // nessuna persistenza finché non vengono collegati a una vera
      // funzionalità.
      timelineLaneToggle: false,
      excludeFromModulesToggle: false,
      // Contenuto del log dedicato del watcher (creazione, avvio, dati
      // letti, timeout, chiusura...), riletto a intervalli regolari per
      // dare l'effetto di una finestra "in tempo reale" senza dover
      // costruire un canale di eventi push dedicato.
      watcherLogContent: '',
      logPollInterval: null,
      // Nome del file lanciato ad ogni giro (solo modalità semplificata),
      // modificabile in tabella — sincronizzato da customWatcherInfo.args
      // ogni volta che viene ricaricato, vedi loadCustomWatcherInfo.
      fileAssociatoInput: '',
    };
  },
  computed: {
    bucket() {
      return this.bucketsStore.getBucket(this.id) || { id: this.id };
    },
    bucket_with_events() {
      return {
        ...this.bucket,
        events: this.events,
      };
    },
    comandoCompleto() {
      if (!this.customWatcherInfo) return '';
      return [this.customWatcherInfo.command, ...(this.customWatcherInfo.args || [])]
        .filter(Boolean)
        .join(' ');
    },
    intervalloPollingMinuti() {
      if (!this.customWatcherInfo || this.customWatcherInfo.interval_seconds == null) return '';
      const minuti = this.customWatcherInfo.interval_seconds / 60;
      return Number.isInteger(minuti) ? String(minuti) : minuti.toFixed(1);
    },
  },
  watch: {
    daterange: async function () {
      await this.getEvents(this.id);
    },
  },
  mounted: async function () {
    await this.bucketsStore.ensureLoaded();
    await this.getEventCount(this.id);
    await this.loadCustomWatcherInfo();
    await this.loadTotalDuration();
    if (this.customWatcherInfo) {
      await this.fetchWatcherLog();
      this.logPollInterval = setInterval(this.fetchWatcherLog, 1500);
    }
  },
  beforeDestroy: function () {
    if (this.logPollInterval) {
      clearInterval(this.logPollInterval);
    }
  },
  methods: {
    getEvents: async function (bucket_id) {
      const bucket = await this.bucketsStore.getBucketWithEvents({
        id: bucket_id,
        start: this.daterange[0].format(),
        end: this.daterange[1].format(),
      });
      this.events = bucket.events;
    },
    getEventCount: async function (bucket_id) {
      // countEvents() risolve già al numero (aw-client spacchetta res.data
      // internamente) — leggere ".data" qui sopra restituiva sempre
      // undefined, la causa del "Numero eventi" vuoto in pagina.
      this.eventcount = await getClient().countEvents(bucket_id);
    },
    // Somma le durate di TUTTI gli eventi del bucket (non solo quelli
    // nell'intervallo attualmente selezionato nella timeline) — stesso
    // pattern AQL (sum_durations) già usato altrove nell'app, es.
    // queries.ts/activityQueryAndroid.
    loadTotalDuration: async function () {
      try {
        const inizio = this.bucket.created ? new Date(this.bucket.created) : new Date(0);
        const fine = new Date(Date.now() + 24 * 60 * 60 * 1000);
        const periodo = `${inizio.toISOString()}/${fine.toISOString()}`;
        const risultato = await getClient().query(
          [periodo],
          [`events = query_bucket("${this.id}");`, 'duration = sum_durations(events);', 'RETURN = duration;']
        );
        this.totalDurationSeconds = risultato[0];
      } catch (e) {
        this.totalDurationSeconds = null;
      }
    },
    loadCustomWatcherInfo: async function () {
      try {
        const lista = await invoke('elenca_watcher_personalizzati');
        this.customWatcherInfo = (lista || []).find(w => w.bucket_id === this.id) || null;
        if (this.customWatcherInfo && this.customWatcherInfo.mode === 'interval') {
          const args = this.customWatcherInfo.args || [];
          this.fileAssociatoInput = args.length > 0 ? args[args.length - 1] : '';
        }
      } catch (e) {
        this.customWatcherInfo = null;
      }
    },
    // Cambia il file lanciato ad ogni giro dal watcher (solo modalità
    // semplificata) — richiesta esplicita: campo modificabile in
    // tabella, purché il file resti dentro la cartella del watcher (il
    // comando rifiuta percorsi con "/", "\" o "..").
    salvaFileAssociato: async function () {
      if (!this.customWatcherInfo || this.customWatcherInfo.mode !== 'interval') return;
      const nuovo = (this.fileAssociatoInput || '').trim();
      const args = this.customWatcherInfo.args || [];
      const attuale = args.length > 0 ? args[args.length - 1] : '';
      if (!nuovo || nuovo === attuale) {
        this.fileAssociatoInput = attuale;
        return;
      }
      try {
        await invoke('imposta_file_watcher', { id: this.customWatcherInfo.id, nomeFile: nuovo });
        await this.loadCustomWatcherInfo();
      } catch (e) {
        console.error(e);
        this.fileAssociatoInput = attuale;
      }
    },
    apriCartellaWatcher: async function () {
      if (!this.customWatcherInfo) return;
      try {
        await invoke('apri_cartella_custom_watcher', { id: this.customWatcherInfo.id });
      } catch (e) {
        console.error(e);
      }
    },
    fetchWatcherLog: async function () {
      if (!this.customWatcherInfo) return;
      try {
        const testo = await invoke('leggi_log_watcher', { id: this.customWatcherInfo.id });
        if (testo !== this.watcherLogContent) {
          this.watcherLogContent = testo as string;
          this.$nextTick(() => this.scrollLogToBottom());
        }
      } catch (e) {
        // Best-effort: se il file non è ancora leggibile, la finestra
        // resta com'era fino al prossimo giro di polling.
      }
    },
    scrollLogToBottom: function () {
      const el = this.$refs.logContent as HTMLElement | undefined;
      if (el) {
        el.scrollTop = el.scrollHeight;
      }
    },
    updateEvent: function (event) {
      const i = this.events.findIndex(e => e.id == event.id);
      if (i != -1) {
        // This is needed instead of this.events[i] because insides of arrays
        // are not reactive in Vue.
        this.$set(this.events, i, event);
      } else {
        console.error(':(');
      }
    },
  },
};
</script>

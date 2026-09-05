<template lang="pug">
div
  div.settings-warning
    b {{ $t('settings.developer.note') }}
    |  {{ $t('settings.developer.noteBody') }}

  div.settings-row
    div
      div.settings-row-title {{ $t('settings.developer.masterToggle') }}
      div.settings-row-help {{ $t('settings.developer.masterToggleHelp') }}
    div.settings-toggle(:class="{ 'settings-toggle-on': developerModeEnabled }" @click="onToggleMaster")
      div.settings-toggle-thumb

  div.dev-expanded(v-if="developerModeEnabled")
    div.settings-row.dev-inner-row
      div
        div.settings-row-title {{ $t('settings.developer.devtoolsTitle') }}
        div.settings-row-help {{ $t('settings.developer.devtoolsHelp') }}
      div.settings-toggle(:class="{ 'settings-toggle-on': devtoolsEnabled }" @click="onToggleDevtools")
        div.settings-toggle-thumb

    div.settings-row.dev-inner-row
      div
        div.settings-row-title {{ $t('settings.developer.rawDataDiagnosticsTitle') }}
        div.settings-row-help {{ $t('settings.developer.rawDataDiagnosticsHelp') }}
      div.settings-toggle(:class="{ 'settings-toggle-on': rawDataDiagnosticsEnabled }" @click="onToggleRawDataDiagnostics")
        div.settings-toggle-thumb

    div.settings-row.dev-inner-row
      div
        div.settings-row-title {{ $t('settings.developer.diagnosticsLog.title') }}
        div.settings-row-help {{ $t('settings.developer.diagnosticsLog.help') }}
      div.settings-toggle(:class="{ 'settings-toggle-on': diagnosticsLoggingEnabled }" @click="onToggleDiagnosticsLog")
        div.settings-toggle-thumb

    div.settings-row.dev-inner-row(v-if="diagnosticsLoggingEnabled")
      div
        div.settings-row-title {{ $t('settings.developer.diagnosticsLog.folderTitle') }}
        div.settings-row-help {{ diagnosticsLogFolder || $t('settings.developer.diagnosticsLog.folderDefault') }}
      div.dev-diagnostics-folder-actions
        div.pill-btn-ghost(@click="scegliCartellaDiagnostica") {{ $t('settings.developer.diagnosticsLog.chooseFolder') }}
        div.pill-btn-ghost(v-if="diagnosticsLogFolder" @click="ripristinaCartellaDiagnosticaDefault") {{ $t('settings.developer.diagnosticsLog.resetFolder') }}

    div.settings-alert.settings-alert-danger(v-if="diagnosticsLogError") {{ diagnosticsLogError }}

    div.dev-section
      div.settings-row-title {{ $t('settings.developer.watcherStatus.title') }}
      div.settings-row-help {{ $t('settings.developer.watcherStatus.help') }}

      div.dev-watcher-loading(v-if="caricandoWatcher") {{ $t('settings.developer.watcherStatus.loading') }}
      div.dev-watcher-list(v-else)
        div.dev-watcher-row(v-for="w in watcherRows" :key="w.name")
          div.dev-watcher-names
            div.dev-watcher-name {{ w.label }}
            div.dev-watcher-meta {{ w.metaText }}
          div.dev-watcher-lastevent {{ w.lastEventText }}
          span.dev-watcher-badge(:class="'dev-badge-' + w.badgeClass") {{ w.badgeLabel }}
          //- Toggle "Log dettagliato" — solo per i watcher che lo
          //- supportano (per ora solo Excel, vedi
          //- WATCHER_CON_LOG_DETTAGLIATO in watcher_status.rs). Richiesta
          //- esplicita dell'utente dopo il bug "Trova e sostituisci"
          //- dell'issue #4: poter indagare a fondo un watcher specifico
          //- durante un uso reale prolungato, senza dover attivare/capire
          //- il log diagnostico generale dell'intera app.
          div.dev-watcher-detailed-log(v-if="w.log_dettagliato_disponibile" :title="$t('settings.developer.watcherStatus.detailedLogHelp')")
            span.dev-watcher-detailed-log-label {{ $t('settings.developer.watcherStatus.detailedLog') }}
            div.settings-toggle.dev-watcher-detailed-log-toggle(
              :class="{ 'settings-toggle-on': w.log_dettagliato_abilitato }"
              @click="toggleLogDettagliato(w.name, !w.log_dettagliato_abilitato)"
            )
              div.settings-toggle-thumb
            div.pill-btn-ghost.dev-watcher-detailed-log-open(@click="apriLogDettagliato(w.name)") {{ $t('settings.developer.watcherStatus.detailedLogOpen') }}
          div.dev-watcher-actions
            div.pill-btn-ghost.dev-watcher-btn(v-if="w.action === 'restart'" @click="riavvia(w.name)") {{ $t('settings.developer.watcherStatus.restart') }}
            div.pill-btn-ghost.dev-watcher-btn(v-if="w.action === 'start'" @click="avviaSessione(w.name)") {{ $t('settings.developer.watcherStatus.startSession') }}
            div.pill-btn-ghost.dev-watcher-btn(v-if="w.action === 'stop'" @click="fermaSessione(w.name)") {{ $t('settings.developer.watcherStatus.stopSession') }}

      div.dev-section.dev-custom-section
        div.settings-row-title {{ $t('settings.developer.watcherStatus.customTitle') }}
        div.settings-row-help {{ $t('settings.developer.watcherStatus.customHelp') }}

        div.settings-row-help(v-if="customBuckets.length === 0") {{ $t('settings.developer.watcherStatus.noCustom') }}
        div.dev-watcher-list(v-else)
          div.dev-watcher-row(v-for="b in customBuckets" :key="b.id")
            div.dev-watcher-names
              div.dev-watcher-name {{ b.id }}
              div.dev-watcher-meta {{ b.client }} · {{ b.type }}
            div.dev-watcher-lastevent {{ b.lastEventText }}

    div.dev-section
      div.dev-log-header
        div
          div.settings-row-title {{ $t('settings.developer.logPanel.title') }}
          div.settings-row-help {{ $t('settings.developer.logPanel.help') }}
        div.dev-log-controls
          select.settings-field.dev-log-level(v-model="logLevelFilter")
            option(value="all") {{ $t('settings.developer.logPanel.levelAll') }}
            option(value="warn") {{ $t('settings.developer.logPanel.levelWarnPlus') }}
            option(value="error") {{ $t('settings.developer.logPanel.levelErrorOnly') }}
          div.pill-btn-ghost.dev-log-btn(@click="logPaused = !logPaused")
            | {{ logPaused ? $t('settings.developer.logPanel.resume') : $t('settings.developer.logPanel.paused') }}
          div.pill-btn-ghost.dev-log-btn(@click="pulisciLog") {{ $t('settings.developer.logPanel.clear') }}

      div.dev-log-box(ref="logBox")
        div.settings-row-help(v-if="logRigheFiltrate.length === 0") {{ $t('settings.developer.logPanel.empty') }}
        div.dev-log-line(v-for="riga in logRigheFiltrate" :key="riga.id" :class="'dev-log-level-' + riga.levelClass")
          span.dev-log-time {{ riga.time }}
          span.dev-log-message {{ riga.message }}

    div.dev-section
      div.settings-row-title {{ $t('settings.developer.aqlConsole.title') }}
      div.settings-row-help {{ $t('settings.developer.aqlConsole.help') }}

      div.dev-aql-period
        span.dev-aql-period-label {{ $t('settings.developer.aqlConsole.periodLabel') }}
        input.settings-field.dev-aql-datetime(type="datetime-local" v-model="aqlStart")
        span.dev-aql-period-sep →
        input.settings-field.dev-aql-datetime(type="datetime-local" v-model="aqlEnd")

      textarea.settings-field.settings-textarea.dev-aql-textarea(
        v-model="aqlQuery"
        rows="6"
        spellcheck="false"
      )

      div.dev-aql-actions
        div.pill-btn(@click="!aqlRunning && eseguiAql()" :class="{ 'pill-btn-disabled': aqlRunning }")
          | {{ aqlRunning ? $t('settings.developer.aqlConsole.running') : $t('settings.developer.aqlConsole.run') }}
        div.pill-btn-ghost(v-if="aqlResultText || aqlError" @click="aqlResultText = ''; aqlError = ''")
          | {{ $t('settings.developer.aqlConsole.clearResult') }}

      div.settings-alert.settings-alert-danger(v-if="aqlError") {{ aqlError }}

      div(v-if="aqlResultText")
        div.settings-row-help.dev-aql-result-label {{ $t('settings.developer.aqlConsole.resultLabel') }}
        pre.dev-aql-result {{ aqlResultText }}

    div.dev-section
      div.settings-row-title {{ $t('settings.developer.folderShortcuts.title') }}
      div.settings-row-help {{ $t('settings.developer.folderShortcuts.help') }}

      div.settings-alert.settings-alert-danger(v-if="folderError") {{ folderError }}

      div.dev-folder-row
        div
          div.settings-row-title {{ $t('settings.developer.folderShortcuts.dataFolder') }}
          div.settings-row-help {{ $t('settings.developer.folderShortcuts.dataFolderHelp') }}
        div.pill-btn-ghost(@click="apriCartella('apri_cartella_dati')") {{ $t('settings.developer.folderShortcuts.open') }}
      div.dev-folder-row
        div
          div.settings-row-title {{ $t('settings.developer.folderShortcuts.logFolder') }}
          div.settings-row-help {{ $t('settings.developer.folderShortcuts.logFolderHelp') }}
        div.pill-btn-ghost(@click="apriCartella('apri_cartella_log')") {{ $t('settings.developer.folderShortcuts.open') }}
      div.dev-folder-row
        div
          div.settings-row-title {{ $t('settings.developer.folderShortcuts.afkConfigFolder') }}
          div.settings-row-help {{ $t('settings.developer.folderShortcuts.afkConfigFolderHelp') }}
        div.pill-btn-ghost(@click="apriCartella('apri_cartella_config_afk')") {{ $t('settings.developer.folderShortcuts.open') }}
      div.dev-folder-row
        div
          div.settings-row-title {{ $t('settings.developer.folderShortcuts.watcherFolder') }}
          div.settings-row-help {{ $t('settings.developer.folderShortcuts.watcherFolderHelp') }}
        div.pill-btn-ghost(@click="apriCartella('apri_cartella_watcher')") {{ $t('settings.developer.folderShortcuts.open') }}

  div.settings-commit-hash {{ $t('settings.developer.commitHash') }} {{ COMMIT_HASH }}

  confirm-modal(
    v-if="showConfirm"
    :title="$t('settings.developer.confirmTitle')"
    :confirm-label="$t('settings.developer.confirmEnable')"
    :cancel-label="$t('common.cancel')"
    @confirm="confirmEnableDeveloperMode"
    @cancel="showConfirm = false"
  )
    div {{ $t('settings.developer.confirmBody') }}
</template>

<script lang="ts">
import moment from 'moment';
import { invoke } from '@tauri-apps/api/core';
import { open as apriSelettoreCartella } from '@tauri-apps/plugin-dialog';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { setAbilitata as impostaDiagnosticaAbilitataJs } from '~/util/diagnostics';
import { useSettingsStore } from '~/stores/settings';
import { useBucketsStore } from '~/stores/buckets';
import { KNOWN_WATCHER_CLIENTS } from '~/util/knownWatchers';
import { querystr_to_array } from '~/queries';
import { getClient } from '~/util/awclient';

const DEFAULT_AQL_QUERY = `events = query_bucket(find_bucket("aw-watcher-window"));
events = sort_by_timestamp(events);
events = limit_events(events, 20);
RETURN = events;`;

interface WatcherStatusDto {
  name: string;
  label: string;
  enabled_in_config: boolean;
  running: boolean;
  pid: number | null;
  has_process: boolean;
  log_dettagliato_disponibile: boolean;
  log_dettagliato_abilitato: boolean;
}

// Alcuni moduli "virtuali" (VoiSpeed) usano un client diverso dal loro
// nome modulo/config per i bucket — vedi util/knownWatchers.ts.
const CLIENT_FOR_MODULE: Record<string, string> = {
  'aw-watcher-voispeed': 'trackflow-voispeed',
};

const REFRESH_MS = 4000;

// Stesso schema numerico di tauri-plugin-log::LogLevel (Trace=1 ...
// Error=5) — il payload dell'evento "log://log" manda solo questo
// numero, non una stringa (vedi RecordPayload in tauri-plugin-log).
interface LogEventPayload {
  message: string;
  level: number;
}

interface LogRow {
  id: number;
  level: number;
  levelClass: 'debug' | 'info' | 'warn' | 'error';
  time: string;
  message: string;
}

const MAX_LOG_LINES = 500;
let logIdCounter = 0;

export default {
  name: 'DeveloperSettings',
  components: {
    'confirm-modal': () => import('~/components/ConfirmModal.vue'),
  },
  data() {
    return {
      settingsStore: useSettingsStore(),
      bucketsStore: useBucketsStore(),
      showConfirm: false,
      caricandoWatcher: true,
      watchers: [] as WatcherStatusDto[],
      refreshInterval: null as ReturnType<typeof setInterval> | null,
      logLines: [] as LogRow[],
      logLevelFilter: 'all' as 'all' | 'warn' | 'error',
      logPaused: false,
      logUnlisten: null as UnlistenFn | null,
      aqlStart: moment().startOf('day').format('YYYY-MM-DDTHH:mm'),
      aqlEnd: moment().format('YYYY-MM-DDTHH:mm'),
      aqlQuery: DEFAULT_AQL_QUERY,
      aqlRunning: false,
      aqlError: '',
      aqlResultText: '',
      folderError: '',
      diagnosticsLogError: '',
    };
  },
  computed: {
    developerModeEnabled(): boolean {
      return this.settingsStore.developerModeEnabled;
    },
    devtoolsEnabled(): boolean {
      return this.settingsStore.devtoolsEnabled;
    },
    rawDataDiagnosticsEnabled(): boolean {
      return this.settingsStore.rawDataDiagnosticsEnabled;
    },
    diagnosticsLoggingEnabled(): boolean {
      return this.settingsStore.diagnosticsLoggingEnabled;
    },
    diagnosticsLogFolder(): string {
      return this.settingsStore.diagnosticsLogFolder;
    },
    watcherRows(this: any) {
      return this.watchers.map((w: WatcherStatusDto) => {
        const client = CLIENT_FOR_MODULE[w.name] || w.name;
        const bucket = this.bucketsStore.buckets.find((b: any) => b.client === client);
        const lastEventText = bucket && bucket.last_updated
          ? `${this.$t('settings.developer.watcherStatus.lastEventLabel')}: ${moment(bucket.last_updated).fromNow()}`
          : this.$t('settings.developer.watcherStatus.noEventsYet');

        let badgeClass: string;
        let badgeLabel: string;
        let action: 'restart' | 'start' | 'stop' | null;
        if (!w.has_process) {
          badgeClass = w.enabled_in_config ? 'up' : 'disabled';
          badgeLabel = w.enabled_in_config
            ? this.$t('settings.developer.watcherStatus.statusUp')
            : this.$t('settings.developer.watcherStatus.statusDisabled');
          action = null;
        } else if (w.running && w.enabled_in_config) {
          badgeClass = 'up';
          badgeLabel = this.$t('settings.developer.watcherStatus.statusUp');
          action = null;
        } else if (w.running && !w.enabled_in_config) {
          badgeClass = 'up-session';
          badgeLabel = this.$t('settings.developer.watcherStatus.statusUpSession');
          action = 'stop';
        } else if (!w.running && w.enabled_in_config) {
          badgeClass = 'stopped';
          badgeLabel = this.$t('settings.developer.watcherStatus.statusStopped');
          action = 'restart';
        } else {
          badgeClass = 'disabled';
          badgeLabel = this.$t('settings.developer.watcherStatus.statusDisabled');
          action = 'start';
        }

        const metaText = [w.name, w.pid ? `${this.$t('settings.developer.watcherStatus.pidLabel')} ${w.pid}` : null]
          .filter(Boolean)
          .join(' · ');

        return { ...w, metaText, lastEventText, badgeClass, badgeLabel, action };
      });
    },
    customBuckets(this: any) {
      return this.bucketsStore.buckets
        .filter((b: any) => !b.client || !KNOWN_WATCHER_CLIENTS.has(b.client))
        .map((b: any) => ({
          id: b.id,
          client: b.client || '?',
          type: b.type,
          lastEventText: b.last_updated
            ? `${this.$t('settings.developer.watcherStatus.lastEventLabel')}: ${moment(b.last_updated).fromNow()}`
            : this.$t('settings.developer.watcherStatus.noEventsYet'),
        }));
    },
    logRigheFiltrate(this: any): LogRow[] {
      const soglia = this.logLevelFilter === 'error' ? 5 : this.logLevelFilter === 'warn' ? 4 : 1;
      return this.logLines.filter((r: LogRow) => r.level >= soglia);
    },
  },
  watch: {
    developerModeEnabled(enabled: boolean) {
      if (enabled) {
        this.avviaAggiornamento();
        this.avviaLog();
      } else {
        this.fermaAggiornamento();
        this.fermaLog();
      }
    },
  },
  mounted() {
    if (this.developerModeEnabled) {
      this.avviaAggiornamento();
      this.avviaLog();
    }
  },
  beforeDestroy() {
    this.fermaAggiornamento();
    this.fermaLog();
  },
  methods: {
    avviaAggiornamento() {
      this.caricaStato();
      if (!this.refreshInterval) {
        this.refreshInterval = setInterval(() => this.caricaStato(), REFRESH_MS);
      }
    },
    fermaAggiornamento() {
      if (this.refreshInterval) {
        clearInterval(this.refreshInterval);
        this.refreshInterval = null;
      }
    },
    async caricaStato() {
      try {
        const [watchers] = await Promise.all([
          invoke<WatcherStatusDto[]>('stato_watcher'),
          this.bucketsStore.loadBuckets(),
        ]);
        this.watchers = watchers;
      } catch (e) {
        // Fuori da Tauri (dev server puro nel browser) invoke() non
        // esiste — stesso pattern già usato in CategorizationSettings.vue.
      } finally {
        this.caricandoWatcher = false;
      }
    },
    async avviaSessione(name: string) {
      try {
        await invoke('avvia_watcher_sessione', { name });
      } catch (e) {
        // Fuori da Tauri.
      }
      this.caricaStato();
    },
    async fermaSessione(name: string) {
      try {
        await invoke('ferma_watcher_sessione', { name });
      } catch (e) {
        // Fuori da Tauri.
      }
      this.caricaStato();
    },
    async riavvia(name: string) {
      try {
        await invoke('riavvia_watcher', { name });
      } catch (e) {
        // Fuori da Tauri.
      }
      this.caricaStato();
    },
    async toggleLogDettagliato(this: any, name: string, abilitato: boolean) {
      // Ottimistico: aggiorna subito la spunta invece di aspettare il
      // prossimo giro di caricaStato() (fino a REFRESH_MS di ritardo,
      // percepibile come "non ha funzionato" al primo click).
      const w = this.watchers.find((x: WatcherStatusDto) => x.name === name);
      if (w) w.log_dettagliato_abilitato = abilitato;
      try {
        await invoke('imposta_log_dettagliato_watcher', { nome: name, abilitato });
      } catch (e) {
        // Fuori da Tauri, o comando rifiutato — la prossima caricaStato()
        // (max REFRESH_MS) riallinea comunque lo stato mostrato a quello
        // vero letto dal backend.
      }
    },
    async apriLogDettagliato(this: any, name: string) {
      this.folderError = '';
      try {
        await invoke('apri_log_dettagliato_watcher', { nome: name });
      } catch (e: any) {
        this.folderError = `${this.$t('settings.developer.folderShortcuts.openError')} ${e?.message ?? e}`;
      }
    },
    async avviaLog(this: any) {
      if (this.logUnlisten) return;
      try {
        this.logUnlisten = await listen<LogEventPayload>('log://log', event => {
          // In pausa: la riga si perde invece di accodarsi — è solo una
          // coda dal vivo (max 500 righe comunque), non un registro
          // persistente: quello resta il file di log su disco, mai
          // toccato da questo pannello.
          if (this.logPaused) return;
          const level = event.payload.level;
          const levelClass = level >= 5 ? 'error' : level >= 4 ? 'warn' : level >= 3 ? 'info' : 'debug';
          this.logLines.push({
            id: logIdCounter++,
            level,
            levelClass,
            time: moment().format('HH:mm:ss'),
            message: event.payload.message,
          });
          if (this.logLines.length > MAX_LOG_LINES) {
            this.logLines.splice(0, this.logLines.length - MAX_LOG_LINES);
          }
          this.$nextTick(() => this.scrollLogToBottom());
        });
      } catch (e) {
        // Fuori da Tauri (dev server puro nel browser) — listen() via
        // IPC non esiste in quel caso.
      }
    },
    fermaLog(this: any) {
      if (this.logUnlisten) {
        this.logUnlisten();
        this.logUnlisten = null;
      }
    },
    pulisciLog(this: any) {
      this.logLines = [];
    },
    scrollLogToBottom(this: any) {
      const box = this.$refs.logBox as HTMLElement | undefined;
      if (box) box.scrollTop = box.scrollHeight;
    },
    async eseguiAql(this: any) {
      this.aqlError = '';
      const statements = querystr_to_array(this.aqlQuery);
      if (statements.length === 0) {
        this.aqlError = this.$t('settings.developer.aqlConsole.emptyQueryError');
        return;
      }
      const period = `${moment(this.aqlStart).format()}/${moment(this.aqlEnd).format()}`;
      this.aqlRunning = true;
      try {
        const data = await getClient().query([period], statements);
        this.aqlResultText = JSON.stringify(data[0], null, 2);
      } catch (e: any) {
        // Il server risponde con un messaggio d'errore AQL leggibile
        // (riga/colonna del problema) — stesso testo che vedresti nei
        // log del server, qui mostrato direttamente senza doverlo
        // andare a cercare.
        this.aqlError =
          e?.response?.data?.message || e?.response?.data || e?.message || String(e);
        this.aqlResultText = '';
      } finally {
        this.aqlRunning = false;
      }
    },
    async apriCartella(this: any, comando: string) {
      this.folderError = '';
      try {
        await invoke(comando);
      } catch (e: any) {
        this.folderError = `${this.$t('settings.developer.folderShortcuts.openError')} ${e?.message ?? e}`;
      }
    },
    onToggleMaster() {
      if (this.developerModeEnabled) {
        // Spegnere non richiede conferma (è la direzione sicura) — e
        // riporta anche le singole opzioni sotto al loro default sicuro,
        // così non restano attive "in silenzio" dopo aver richiuso la
        // sezione (vedi devtoolsGuard.ts: legge developerModeEnabled
        // solo indirettamente tramite devtoolsEnabled).
        this.settingsStore.update({
          developerModeEnabled: false,
          devtoolsEnabled: false,
          rawDataDiagnosticsEnabled: false,
          diagnosticsLoggingEnabled: false,
        });
        impostaDiagnosticaAbilitataJs(false);
        invoke('imposta_diagnostica', { abilitata: false, cartella: null }).catch(() => {});
      } else {
        this.showConfirm = true;
      }
    },
    confirmEnableDeveloperMode() {
      this.showConfirm = false;
      this.settingsStore.update({ developerModeEnabled: true });
    },
    async onToggleRawDataDiagnostics() {
      await this.settingsStore.update({ rawDataDiagnosticsEnabled: !this.rawDataDiagnosticsEnabled });
    },
    async onToggleDiagnosticsLog(this: any) {
      this.diagnosticsLogError = '';
      const next = !this.diagnosticsLoggingEnabled;
      await this.settingsStore.update({ diagnosticsLoggingEnabled: next });
      impostaDiagnosticaAbilitataJs(next);
      try {
        await invoke('imposta_diagnostica', {
          abilitata: next,
          cartella: this.diagnosticsLogFolder || null,
        });
      } catch (e: any) {
        this.diagnosticsLogError = `${this.$t('settings.developer.diagnosticsLog.error')} ${e?.message ?? e}`;
      }
    },
    async scegliCartellaDiagnostica(this: any) {
      this.diagnosticsLogError = '';
      let cartella: string | string[] | null;
      try {
        cartella = await apriSelettoreCartella({ directory: true });
      } catch (e) {
        // Fuori da Tauri (dev server puro nel browser) — il selettore
        // nativo non esiste in quel contesto, non bloccante.
        return;
      }
      if (!cartella || Array.isArray(cartella)) return;
      await this.settingsStore.update({ diagnosticsLogFolder: cartella });
      if (this.diagnosticsLoggingEnabled) {
        try {
          await invoke('imposta_diagnostica', { abilitata: true, cartella });
        } catch (e: any) {
          this.diagnosticsLogError = `${this.$t('settings.developer.diagnosticsLog.error')} ${e?.message ?? e}`;
        }
      }
    },
    async ripristinaCartellaDiagnosticaDefault(this: any) {
      this.diagnosticsLogError = '';
      await this.settingsStore.update({ diagnosticsLogFolder: '' });
      if (this.diagnosticsLoggingEnabled) {
        try {
          await invoke('imposta_diagnostica', { abilitata: true, cartella: null });
        } catch (e: any) {
          this.diagnosticsLogError = `${this.$t('settings.developer.diagnosticsLog.error')} ${e?.message ?? e}`;
        }
      }
    },
    async onToggleDevtools() {
      const next = !this.devtoolsEnabled;
      await this.settingsStore.update({ devtoolsEnabled: next });
      if (next) {
        try {
          await invoke('apri_devtools');
        } catch (e) {
          // Fuori da Tauri (dev server puro nel browser) invoke() non
          // esiste — stesso pattern già usato in CategorizationSettings.vue.
        }
      }
    },
  },
};
</script>

<style scoped>
.settings-warning {
  background-color: rgba(211, 163, 85, 0.15);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  padding: 10px 14px;
  font-size: var(--font-size-sm);
  color: var(--color-text);
  margin-bottom: 18px;
}

.dev-expanded {
  margin-top: 16px;
  padding: 18px 20px;
  border-radius: var(--radius-lg);
  background-color: var(--color-surface2);
}

.dev-inner-row {
  margin: 0;
}

.dev-section {
  margin-top: 20px;
  padding-top: 18px;
  border-top: 1px solid var(--color-border);
}

.dev-custom-section {
  margin-top: 18px;
}

.dev-watcher-loading {
  margin-top: 10px;
  font-size: var(--font-size-sm);
  color: var(--color-text-dim);
}

.dev-watcher-list {
  margin-top: 10px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  overflow: hidden;
}

.dev-watcher-row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 12px;
  border-bottom: 1px solid var(--color-border);
  background-color: var(--color-surface);
}

.dev-watcher-row:last-child {
  border-bottom: none;
}

.dev-watcher-names {
  flex: 1;
  min-width: 0;
}

.dev-watcher-name {
  font-size: var(--font-size-sm);
  color: var(--color-text);
}

.dev-watcher-meta {
  font-size: var(--font-size-xs);
  color: var(--color-text-faint);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.dev-watcher-lastevent {
  font-size: var(--font-size-xs);
  color: var(--color-text-faint);
  white-space: nowrap;
  flex-shrink: 0;
}

.dev-watcher-badge {
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-semibold);
  padding: 3px 9px;
  border-radius: var(--radius-sm);
  white-space: nowrap;
  flex-shrink: 0;
}

.dev-badge-up {
  background-color: rgba(90, 176, 110, 0.18);
  color: var(--color-success);
}

.dev-badge-up-session {
  background-color: rgba(211, 163, 85, 0.18);
  color: var(--color-accent1);
}

.dev-badge-stopped {
  background-color: rgba(217, 83, 79, 0.18);
  color: #ff8a80;
}

.dev-badge-disabled {
  background-color: var(--color-surface2);
  color: var(--color-text-faint);
}

.dev-watcher-detailed-log {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}

.dev-watcher-detailed-log-label {
  font-size: var(--font-size-xs);
  color: var(--color-text-faint);
  white-space: nowrap;
}

.dev-watcher-detailed-log-toggle {
  transform: scale(0.75);
  transform-origin: center;
}

.dev-watcher-detailed-log-open {
  font-size: var(--font-size-xs);
  padding: 4px 8px;
  white-space: nowrap;
}

.dev-watcher-actions {
  flex-shrink: 0;
  width: 168px;
  display: flex;
  justify-content: flex-end;
}

.dev-watcher-btn {
  font-size: var(--font-size-xs);
  padding: 5px 10px;
}

.settings-commit-hash {
  margin-top: 18px;
  font-size: var(--font-size-xs);
  color: var(--color-text-faint);
}

.dev-log-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
}

.dev-log-controls {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.dev-log-level {
  width: auto;
  font-size: var(--font-size-xs);
  padding: 5px 8px;
}

.dev-log-btn {
  font-size: var(--font-size-xs);
  padding: 5px 10px;
  white-space: nowrap;
}

.dev-log-box {
  margin-top: 10px;
  max-height: 320px;
  overflow-y: auto;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  background-color: var(--color-surface);
  padding: 8px 12px;
  font-family: 'SFMono-Regular', Consolas, 'Liberation Mono', Menlo, monospace;
  font-size: var(--font-size-xs);
}

.dev-log-line {
  display: flex;
  gap: 10px;
  padding: 2px 0;
  color: var(--color-text-dim);
  white-space: pre-wrap;
  word-break: break-word;
}

.dev-log-time {
  flex-shrink: 0;
  color: var(--color-text-faint);
}

.dev-log-level-warn {
  color: var(--color-accent1);
}

.dev-log-level-error {
  color: #ff8a80;
}

.dev-aql-period {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 10px;
}

.dev-aql-period-label {
  font-size: var(--font-size-sm);
  color: var(--color-text-dim);
  flex-shrink: 0;
}

.dev-aql-period-sep {
  color: var(--color-text-faint);
  flex-shrink: 0;
}

.dev-aql-datetime {
  width: auto;
  font-size: var(--font-size-sm);
}

.dev-aql-textarea {
  display: block;
  width: 100%;
  margin-top: 10px;
  font-family: 'SFMono-Regular', Consolas, monospace;
  resize: vertical;
}

.dev-aql-actions {
  display: flex;
  gap: 8px;
  margin-top: 10px;
}

.dev-aql-result-label {
  margin-top: 12px;
  margin-bottom: 4px;
}

.dev-aql-result {
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  padding: 10px 12px;
  font-family: 'SFMono-Regular', Consolas, monospace;
  font-size: var(--font-size-xs);
  color: var(--color-text-dim);
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 360px;
  overflow-y: auto;
  margin: 0;
}

.dev-folder-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.dev-folder-row + .dev-folder-row {
  margin-top: 14px;
}

.dev-diagnostics-folder-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}
</style>

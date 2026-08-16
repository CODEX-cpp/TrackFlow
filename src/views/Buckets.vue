<template lang="pug">
div.raw-data-page
  div.page-head
    div.page-title {{ $t('buckets.title') }}

  div.grid-head {{ $t('buckets.title') }}
  div.raw-data-empty(v-if="watcherRows.length === 0") {{ $t('buckets.noEvents') }}
  table.raw-data-table(v-else)
    thead
      tr
        th {{ $t('buckets.bucketId') }}
        th {{ $t('buckets.updated') }}
        th
    tbody
      tr(v-for="row in watcherRows" :key="row.key")
        td
          span.bucket-id(:title="row.bucketId || row.key") {{ row.displayName }}
        td
          span(v-if="row.hasBucket && row.lastUpdated" :class="{ 'raw-data-recent': isRecent(row.lastUpdated) }")
            | {{ row.lastUpdated | friendlytime }}
          span.raw-data-muted(v-else-if="row.hasBucket") {{ $t('buckets.noEvents') }}
          span.raw-data-muted(v-else) {{ $t('customModuleWizard.waitingForData') }}
        td.raw-data-actions
          router-link.pill-btn-ghost(v-if="row.hasBucket && settingsStore.rawDataDiagnosticsEnabled" :to="'/buckets/' + row.bucketId") {{ $t('common.open') }}
          span.icon-btn(v-if="row.hasBucket && !row.csvDisabilitato" @click="export_csv(row.bucketId)" :title="$t('buckets.exportEventsCsv')")
            icon(name="download")
          div.settings-toggle(v-if="!row.noToggle" :class="{ 'settings-toggle-on': row.enabled }" @click="toggleWatcher(row)")
            div.settings-toggle-thumb

  div.page-head.orphan-head
    div.grid-head {{ $t('buckets.customSectionTitle') }}
  div.page-hint.orphan-hint {{ $t('buckets.customSectionHelp') }}
  table.raw-data-table(v-if="customSectionRows.length > 0")
    thead
      tr
        th {{ $t('buckets.bucketId') }}
        th {{ $t('buckets.updated') }}
        th
    tbody
      tr(v-for="row in customSectionRows" :key="row.key")
        td
          span.bucket-id(:title="row.bucketId || row.key") {{ row.displayName }}
        td
          span(v-if="row.hasBucket && row.lastUpdated" :class="{ 'raw-data-recent': isRecent(row.lastUpdated) }")
            | {{ row.lastUpdated | friendlytime }}
          span.raw-data-muted(v-else-if="row.hasBucket") {{ $t('buckets.noEvents') }}
          span.raw-data-muted(v-else) {{ $t('customModuleWizard.waitingForData') }}
        td.raw-data-actions
          router-link.pill-btn-ghost(v-if="row.hasBucket" :to="'/buckets/' + row.bucketId") {{ $t('common.open') }}
          span.icon-btn(v-if="row.hasBucket" @click="export_csv(row.bucketId)" :title="$t('buckets.exportEventsCsv')")
            icon(name="download")
          span.icon-btn.icon-btn-danger(v-if="row.hasBucket || row.watcherId" @click="openDeleteBucketModal(row)" :title="$t('buckets.deleteBucket')")
            icon(name="trash")

  div.new-watcher-row
    div.pill-btn(@click="showWatcherWizard = true") {{ $t('buckets.newWatcher') }}

  div.grid-head.io-head {{ $t('buckets.importExportTitle') }}
  div.io-row
    div.io-card
      div.io-card-title {{ $t('buckets.importBuckets') }}
      div.field-error(v-if="import_error") {{ import_error }}
      div.dropzone(
        v-if="!import_file"
        :class="{ 'dropzone-active': isDraggingFile }"
        @click="triggerFileInput"
        @dragover.prevent="isDraggingFile = true"
        @dragleave.prevent="isDraggingFile = false"
        @drop.prevent="onFileDrop"
      )
        icon.dropzone-icon(name="download")
        div.dropzone-text {{ $t('buckets.dropzoneText') }}
        div.dropzone-hint {{ $t('buckets.dropzoneHint') }}
      input.dropzone-input(ref="fileInput" type="file" accept="application/json,.csv,text/csv" @change="onImportFileChange")
      div.progress-loading(v-if="import_file")
        div.progress-bar
          div.progress-bar-fill
        div.progress-loading-text {{ $t('buckets.importing') }}
      div.field-hint {{ $t('buckets.importHelpNew') }}
    div.io-card.io-card-export
      div.io-card-title {{ $t('buckets.exportBuckets') }}
      div.io-card-center
        div.progress-loading(v-if="exportState === 'loading'")
          div.progress-bar
            div.progress-bar-fill
          div.progress-loading-text {{ $t('buckets.exportInProgress') }}
        div.pill-btn(v-else-if="exportState === 'ready'" @click="downloadExportedJson()")
          icon.mr-1(name="download")
          | {{ $t('buckets.downloadExport') }}
        div.pill-btn-ghost(v-else @click="startExportAllBuckets()")
          icon.mr-1(name="download")
          | {{ $t('buckets.exportAllJson') }}
      div.io-card-bottom
        div.field-error(v-if="exportError") {{ exportError }}
        div.field-hint {{ $t('buckets.exportHelp') }}

  confirm-modal(
    v-if="delete_bucket_selected || delete_watcher_selected"
    :title="$t('buckets.deleteBucketTitle')"
    :confirm-label="$t('common.confirm')"
    :cancel-label="$t('common.cancel')"
    @confirm="deleteBucket()"
    @cancel="cancelDeleteBucket()"
  )
    | {{ $t('buckets.deleteConfirm', { id: delete_display_label }) }}
    br
    br
    b {{ $t('buckets.deletePermanent') }}

  //- TEMPORANEO — vedi CONFERME_TOGGLE_RISCHIOSO nello script: rimuovi
  //- quel blocco (o la singola voce del watcher interessato) quando il
  //- problema sottostante è risolto, questo popup sparisce da solo.
  confirm-modal(
    v-if="pendingToggle"
    :title="$t(pendingToggle.titleKey)"
    :confirm-label="$t(pendingToggle.confirmKey)"
    :cancel-label="$t('common.cancel')"
    @confirm="confirmPendingToggle()"
    @cancel="cancelPendingToggle()"
  )
    | {{ $t(pendingToggle.bodyKey) }}

  custom-watcher-wizard(
    v-if="showWatcherWizard"
    @close="showWatcherWizard = false"
    @done="showWatcherWizard = false; refreshWatchersAndBuckets()"
  )
</template>

<style lang="scss" scoped>
@import '../style/theme.css';
@import '../style/modals.css';

.raw-data-page {
  padding: 24px 28px;
}

.page-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}

.orphan-head {
  margin-top: 32px;
  margin-bottom: 2px;
}

.orphan-head .grid-head {
  margin-bottom: 0;
}

.orphan-hint {
  margin-top: 0;
}

.new-watcher-row {
  margin-top: 20px;
}

.page-title {
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-bold);
  color: var(--color-text);
}

.page-hint {
  font-size: var(--font-size-sm);
  color: var(--color-text-faint);
  margin-bottom: 20px;

  a {
    color: var(--color-accent1);
  }
}

.grid-head {
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-bold);
  color: var(--color-text-dim);
  margin-bottom: 12px;
}

.io-head {
  margin-top: 32px;
}

.raw-data-empty {
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-xl);
  padding: 24px;
  text-align: center;
  color: var(--color-text-faint);
  font-size: var(--font-size-sm);
}

.raw-data-table {
  width: 100%;
  border-collapse: collapse;
  font-size: var(--font-size-sm);
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-xl);
  overflow: hidden;

  th {
    text-align: left;
    font-size: 10.5px;
    color: var(--color-text-faint);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-wide);
    padding: 10px 16px;
    border-bottom: 1px solid var(--color-border);
  }

  td {
    padding: 10px 16px;
    border-bottom: 1px solid var(--color-border);
    color: var(--color-text-dim);
  }

  tr:last-child td {
    border-bottom: none;
  }
}

.bucket-id {
  font-family: ui-monospace, SFMono-Regular, 'SF Mono', Menlo, Consolas, monospace;
  color: var(--color-text);
}

.raw-data-recent {
  color: var(--color-accent1);
}

.raw-data-muted {
  color: var(--color-text-faint);
}

.raw-data-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
  white-space: nowrap;
}

.icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: var(--radius-md);
  color: var(--color-text-faint);
  cursor: pointer;
}

.icon-btn:hover {
  background-color: var(--color-surface2);
  color: var(--color-text);
}

.icon-btn-danger:hover {
  color: #d9534f;
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

.danger-ghost:hover {
  color: #d9534f;
}

.field-error {
  font-size: var(--font-size-xs);
  color: #d9534f;
  margin-bottom: 8px;
}

.field-hint {
  font-size: var(--font-size-xs);
  color: var(--color-text-faint);
  margin-top: 8px;
}

.io-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
}

.io-card {
  display: flex;
  flex-direction: column;
  // Le due card affiancate hanno contenuti di altezza diversa (la
  // dropzone di importazione è più "alta" del solo pulsante di
  // esportazione) — essendo comunque alte uguali (griglia, stretch di
  // default), centrare il contenuto invece di lasciarlo ancorato in
  // alto pareggia visivamente le due colonne.
  justify-content: center;
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-xl);
  padding: 20px;
}

.io-card-title {
  font-weight: var(--font-weight-semibold);
  color: var(--color-text);
  margin-bottom: 10px;
}

// Richiesta esplicita: in questa card (a differenza di quella
// import, dove il contenuto resta semplicemente centrato in blocco)
// titolo in alto, pulsante al centro del riquadro, testo di
// spiegazione in basso — "space-between" su tre figli diretti
// distribuisce esattamente così.
.io-card-export {
  justify-content: space-between;
}

// Il pulsante deve essere largo quanto il riquadro, come la dropzone
// della card di importazione affianco — niente flex/centratura qui,
// resta un blocco normale a piena larghezza.
.io-card-bottom .field-hint {
  text-align: left;
}

.dropzone {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 4px;
  text-align: center;
  padding: 24px 16px;
  border: 1px dashed var(--color-border);
  border-radius: var(--radius-md);
  background-color: var(--color-surface2);
  cursor: pointer;
}

.dropzone:hover {
  border-color: var(--color-text-faint);
}

.dropzone-active {
  border-color: var(--color-accent1);
  background-color: var(--color-surface);
}

.dropzone-icon {
  font-size: 20px;
  color: var(--color-text-faint);
  margin-bottom: 4px;
}

.dropzone-text {
  font-size: var(--font-size-sm);
  color: var(--color-text-dim);
}

.dropzone-hint {
  font-size: var(--font-size-xs);
  color: var(--color-text-faint);
}

// L'input file vero resta nel DOM (serve per aprire il dialogo nativo di
// Esplora risorse) ma invisibile: tutti i click passano dalla dropzone
// sopra, che lo attiva a sua volta via triggerFileInput() — altrimenti
// un click diretto sull'input aprirebbe il dialogo due volte.
.dropzone-input {
  display: none;
}

.progress-loading {
  margin-top: 4px;
}

// Barra indeterminata: non conosciamo la percentuale reale di
// avanzamento (il server risponde in un unico blocco, non a step), quindi
// invece di una barra "finta" con una percentuale inventata, una striscia
// che scorre avanti e indietro — comunica "sto lavorando" senza mentire
// su quanto manca.
.progress-bar {
  width: 100%;
  height: 6px;
  border-radius: var(--radius-pill);
  background-color: var(--color-surface2);
  overflow: hidden;
}

.progress-bar-fill {
  width: 40%;
  height: 100%;
  border-radius: var(--radius-pill);
  background-color: var(--color-accent1);
  animation: progress-indeterminate 1.2s ease-in-out infinite;
}

@keyframes progress-indeterminate {
  0% {
    transform: translateX(-100%);
  }
  100% {
    transform: translateX(250%);
  }
}

.progress-loading-text {
  font-size: var(--font-size-xs);
  color: var(--color-text-faint);
  margin-top: 6px;
}
</style>

<script lang="ts">
import 'vue-awesome/icons/trash';
import 'vue-awesome/icons/download';
import 'vue-awesome/icons/folder-open';

import _ from 'lodash';
import Papa from 'papaparse';
import moment from 'moment';
import { invoke } from '@tauri-apps/api/core';

import { useBucketsStore } from '~/stores/buckets';
import { useSettingsStore } from '~/stores/settings';
import { downloadFile } from '~/util/export';

// Stesso schema di src/views/settings/DeveloperSettings.vue — solo
// VoiSpeed scrive sotto un client diverso dal proprio nome modulo (gira
// in-process, non è un sidecar coi suoi argv, vedi voispeed.rs).
const CLIENT_FOR_MODULE: Record<string, string> = {
  'aw-watcher-voispeed': 'trackflow-voispeed',
};

// Bucket della funzione "Progetti" (cronometro), vedi projectTimerMixin.ts —
// non è un modulo di ALL_MODULES (nessun processo da avviare/fermare), ma
// per richiesta esplicita va mostrato forzatamente dentro "Sorgenti dati"
// invece che tra i watcher personalizzati, perché è un bucket di sistema
// dell'app e non qualcosa creato dall'utente col wizard.
const STOPWATCH_BUCKET_ID = 'aw-stopwatch';

// Watcher esclusi dagli export — richiesta esplicita: i dati di
// aw-watcher-screenshot sono solo nomi di file immagine, inutili fuori
// dall'app (le immagini vere non sono nel bucket, restano su disco).
// Per aggiungerne un altro/toglierne uno basta modificare questo Set,
// nessun'altra modifica necessaria.
const WATCHER_ESCLUSI_DA_EXPORT = new Set(['aw-watcher-screenshot']);

interface WatcherStatusDto {
  name: string;
  label: string;
  enabled_in_config: boolean;
  running: boolean;
  pid: number | null;
  has_process: boolean;
}

interface WatcherPersonalizzatoInfo {
  id: string;
  name: string;
  mode: string;
  timeline_lane: boolean;
  running: boolean;
  bucket_id: string | null;
}

// Ordine di importanza per la tabella Sorgenti dati (richiesta esplicita:
// tutti i watcher in ordine di importanza, non quello di dichiarazione/
// alfabetico di ALL_MODULES in lib.rs) — "Finestra attiva" per primo
// perché senza quello l'app non funziona davvero. Un watcher assente da
// questo elenco finisce in fondo, dopo tutti quelli elencati.
const PRIORITY_ORDER = [
  'aw-watcher-window', // senza questo l'app non funziona: Timeline, categorizzazione e quasi tutti i moduli Home dipendono dai suoi dati
  'aw-watcher-afk', // usato per filtrare l'inattività quasi ovunque, insieme a Finestra attiva
  'aw-watcher-app-icons', // icone corrette per tutte le app in Home/Timeline
  'aw-watcher-vscode',
  'aw-watcher-excel',
  'aw-watcher-claude-code',
  'aw-watcher-vpn',
  'aw-watcher-voispeed',
  'aw-watcher-screenshot', // funzionalità accessoria, non tracciamento
  'ai-categorization', // assistente, non un vero watcher
  'aw-watcher-tray', // ancora in sviluppo, vedi CONFERME_TOGGLE_RISCHIOSO sotto
];

// TEMPORANEO — un avviso di conferma prima di accendere/spegnere un
// watcher "rischioso". Rimuovere la riga del watcher qui sotto (o
// l'intera costante) quando il problema descritto è risolto: il popup
// sparisce da solo, nessun'altra modifica necessaria altrove in questo
// file. `direzione: 'off'` avvisa quando lo si spegne, `'on'` quando lo
// si accende.
const CONFERME_TOGGLE_RISCHIOSO: Record<
  string,
  { direzione: 'off' | 'on'; titleKey: string; bodyKey: string; confirmKey: string }
> = {
  'aw-watcher-window': {
    direzione: 'off',
    titleKey: 'buckets.confirmDisableWindowTitle',
    bodyKey: 'buckets.confirmDisableWindowBody',
    confirmKey: 'buckets.confirmDisableWindowConfirm',
  },
  'aw-watcher-tray': {
    direzione: 'on',
    titleKey: 'buckets.confirmEnableTrayTitle',
    bodyKey: 'buckets.confirmEnableTrayBody',
    confirmKey: 'buckets.confirmEnableTrayConfirm',
  },
};

export default {
  name: 'Buckets',
  components: {
    'confirm-modal': () => import('~/components/ConfirmModal.vue'),
    'custom-watcher-wizard': () => import('~/components/CustomWatcherWizard.vue'),
  },
  data() {
    return {
      bucketsStore: useBucketsStore(),
      settingsStore: useSettingsStore(),

      import_file: null,
      import_error: null,
      isDraggingFile: false,
      // Esportazione di tutti i bucket in due passi (richiesta esplicita):
      // 'idle' -> click "Esporta" -> 'loading' (raccolta dati, può durare
      // minuti con uno storico lungo) -> 'ready' (dati pronti in memoria,
      // il pulsante diventa "Scarica file di esportazione") -> click ->
      // dialogo nativo "Salva con nome" -> torna a 'idle'.
      exportState: 'idle' as 'idle' | 'loading' | 'ready',
      exportedJsonData: null as string | null,
      exportError: null as string | null,
      delete_bucket_selected: null,
      delete_watcher_selected: null,
      delete_display_label: null,
      showWatcherWizard: false,
      watcherStatuses: [] as WatcherStatusDto[],
      customWatchers: [] as WatcherPersonalizzatoInfo[],
      // Aggiorna in automatico bucket/watcher mentre la pagina resta
      // aperta — senza questo, un watcher appena creato restava bloccato
      // su "in attesa di dati" (nessun pulsante "Apri") per sempre: il
      // refresh dopo la creazione scatta subito alla chiusura del wizard,
      // troppo presto perché lo script abbia già prodotto il primo dato,
      // e nient'altro richiamava più `loadBuckets()` finché l'utente non
      // ricaricava la pagina a mano (bug reale segnalato da un utente).
      bucketsPollInterval: null as ReturnType<typeof setInterval> | null,
      // Toggle di un watcher "rischioso" (vedi CONFERME_TOGGLE_RISCHIOSO)
      // in attesa di conferma — null quando nessun popup è aperto.
      pendingToggle: null as null | {
        row: any;
        nuovoStato: boolean;
        titleKey: string;
        bodyKey: string;
        confirmKey: string;
      },
    };
  },
  computed: {
    // Una riga per ogni watcher integrato (stessa lista del menu Moduli
    // della tray, vedi ALL_MODULES in lib.rs) — sempre presente anche
    // senza ancora un bucket con dati reali, così si vede subito cosa
    // esiste davvero e non solo cosa ha già prodotto qualcosa. "Finestra
    // attiva" sempre per prima: è l'unico senza cui l'app smette di
    // funzionare, merita di essere il primo che si vede.
    watcherRows(this: any) {
      const posizione = (nome: string) => {
        const i = PRIORITY_ORDER.indexOf(nome);
        return i === -1 ? PRIORITY_ORDER.length : i;
      };
      const ordinati = [...this.watcherStatuses].sort(
        (a: WatcherStatusDto, b: WatcherStatusDto) => posizione(a.name) - posizione(b.name)
      );
      const righe = ordinati.map((w: WatcherStatusDto) => {
        const client = CLIENT_FOR_MODULE[w.name] || w.name;
        const bucket = this.bucketsStore.buckets.find((b: any) => b.client === client);
        return {
          key: w.name,
          displayName: this.$t('firstRunSetup.modules.' + w.name),
          hasBucket: !!bucket,
          bucketId: bucket ? bucket.id : null,
          lastUpdated: bucket ? bucket.last_updated : null,
          isBuiltin: true,
          enabled: w.enabled_in_config,
          csvDisabilitato: WATCHER_ESCLUSI_DA_EXPORT.has(w.name),
          noToggle: false,
        };
      });

      // Riga forzata per il bucket del cronometro progetti — non è un
      // processo avviabile/arrestabile, quindi niente toggle.
      const stopwatchBucket = this.bucketsStore.buckets.find(
        (b: any) => b.id === STOPWATCH_BUCKET_ID
      );
      righe.push({
        key: STOPWATCH_BUCKET_ID,
        displayName: this.$t('buckets.stopwatchSource'),
        hasBucket: !!stopwatchBucket,
        bucketId: STOPWATCH_BUCKET_ID,
        lastUpdated: stopwatchBucket ? stopwatchBucket.last_updated : null,
        isBuiltin: true,
        enabled: true,
        csvDisabilitato: false,
        noToggle: true,
      });

      return righe;
    },
    // Watcher personalizzati creati dall'utente — visibili qui SUBITO al
    // momento della creazione (da elenca_watcher_personalizzati, non dal
    // solo elenco bucket), anche prima che producano il primo dato.
    // Dopo, ogni altro bucket non spiegato né da un modulo integrato né
    // da un watcher personalizzato conosciuto (bucket legacy/orfani, o
    // creati fuori da TrackFlow) — così nessun dato resta invisibile.
    customSectionRows(this: any) {
      const knownClients = new Set(
        this.watcherStatuses.map((w: WatcherStatusDto) => CLIENT_FOR_MODULE[w.name] || w.name)
      );
      const bucketIdsDaWatcher = new Set(
        this.customWatchers.map((w: WatcherPersonalizzatoInfo) => w.bucket_id).filter(Boolean)
      );

      const daWatcher = this.customWatchers.map((w: WatcherPersonalizzatoInfo) => {
        const bucket = w.bucket_id
          ? this.bucketsStore.buckets.find((b: any) => b.id === w.bucket_id)
          : null;
        return {
          key: 'watcher-' + w.id,
          displayName: w.name,
          hasBucket: !!bucket,
          bucketId: bucket ? bucket.id : null,
          lastUpdated: bucket ? bucket.last_updated : null,
          // Cartella su disco del watcher — permette a "Elimina" di
          // cancellare anche processo+cartella, non solo il bucket (senza
          // questo, il processo ancora attivo ricreerebbe da solo il
          // bucket appena cancellato al giro di heartbeat successivo).
          watcherId: w.id,
        };
      });

      const altri = _.orderBy(
        this.bucketsStore.buckets.filter(
          (b: any) =>
            b.id !== STOPWATCH_BUCKET_ID &&
            !bucketIdsDaWatcher.has(b.id) &&
            (!b.client || !knownClients.has(b.client))
        ),
        ['id'],
        ['asc']
      ).map((b: any) => ({
        key: b.id,
        displayName: b.id,
        hasBucket: true,
        bucketId: b.id,
        lastUpdated: b.last_updated,
      }));

      return [...daWatcher, ...altri];
    },
  },
  mounted: async function () {
    await this.bucketsStore.loadBuckets();
    try {
      this.watcherStatuses = await invoke<WatcherStatusDto[]>('stato_watcher');
    } catch {
      // Fuori da Tauri (dev server puro nel browser) invoke() non esiste —
      // stesso pattern già usato altrove (es. CategorizationSettings.vue).
      this.watcherStatuses = [];
    }
    try {
      this.customWatchers = await invoke<WatcherPersonalizzatoInfo[]>('elenca_watcher_personalizzati');
    } catch {
      this.customWatchers = [];
    }
    this.bucketsPollInterval = setInterval(this.refreshWatchersAndBuckets, 3000);
  },
  beforeDestroy: function () {
    if (this.bucketsPollInterval) {
      clearInterval(this.bucketsPollInterval);
    }
  },
  methods: {
    // Accende/spegne un watcher integrato — se il watcher è in
    // CONFERME_TOGGLE_RISCHIOSO e la direzione richiesta è quella
    // segnalata come rischiosa, chiede prima conferma con un popup
    // (vedi applyToggle/confirmPendingToggle sotto); altrimenti applica
    // subito.
    async toggleWatcher(row: { key: string; enabled: boolean }) {
      const nuovoStato = !row.enabled;
      const regola = CONFERME_TOGGLE_RISCHIOSO[row.key];
      const direzioneRischiosa =
        regola && ((regola.direzione === 'off' && !nuovoStato) || (regola.direzione === 'on' && nuovoStato));
      if (direzioneRischiosa) {
        this.pendingToggle = { row, nuovoStato, ...regola };
        return;
      }
      await this.applyToggle(row, nuovoStato);
    },
    // Applica per davvero il cambio — stessa funzione Rust (imposta_modulo)
    // usata dal menu Moduli della tray: persiste su modules-config.json
    // (letto ad ogni avvio dell'app), aggiorna anche la spunta nel menu
    // della tray, avvia/ferma il processo. Non una "modalità solo
    // sessione" come avvia/ferma_watcher_sessione.
    async applyToggle(row: { key: string }, nuovoStato: boolean) {
      try {
        await invoke('imposta_modulo_watcher', { nome: row.key, attivo: nuovoStato });
        const w = this.watcherStatuses.find((w: WatcherStatusDto) => w.name === row.key);
        if (w) w.enabled_in_config = nuovoStato;
      } catch {
        // Fuori da Tauri: nessuna azione possibile, lo stato resta quello reale.
      }
    },
    confirmPendingToggle() {
      if (!this.pendingToggle) return;
      const { row, nuovoStato } = this.pendingToggle;
      this.pendingToggle = null;
      this.applyToggle(row, nuovoStato);
    },
    cancelPendingToggle() {
      this.pendingToggle = null;
    },
    isRecent: function (date) {
      return moment().diff(date) / 1000 < 120;
    },
    // Ricarica bucket + elenco watcher personalizzati da disco — usato sia
    // quando un nuovo watcher viene creato dal wizard (così compare subito
    // come riga, senza aspettare i suoi primi dati né un riavvio dell'app),
    // sia dopo un'eliminazione (per far sparire la riga cancellata).
    refreshWatchersAndBuckets: async function () {
      await this.bucketsStore.loadBuckets();
      try {
        this.customWatchers = await invoke<WatcherPersonalizzatoInfo[]>('elenca_watcher_personalizzati');
      } catch {
        // Fuori da Tauri: l'elenco resta quello di prima.
      }
    },
    // Punto d'ingresso condiviso tra l'input file nativo (scelto tramite
    // Esplora risorse) e il trascinamento diretto nella dropzone — stessa
    // identica logica in entrambi i casi, solo la provenienza del file cambia.
    processImportFile: async function (file: File) {
      this.import_file = file;
      try {
        if (file.name.toLowerCase().endsWith('.csv')) {
          await this.importCsvFile(file);
        } else {
          await this.importBuckets(file);
        }
        this.import_error = null;
      } catch (err: any) {
        this.import_error = err?.message || 'Import failed, see aw-server logs for more info';
      }
      await this.bucketsStore.loadBuckets();
      this.import_file = null;
    },
    onImportFileChange: async function (evt: Event) {
      const target = evt.target as HTMLInputElement;
      const file = target.files && target.files[0];
      if (file) await this.processImportFile(file);
      target.value = '';
    },
    // Apre il dialogo nativo di Esplora risorse cliccando sulla dropzone —
    // l'input file vero resta nascosto (vedi .dropzone-input nello style),
    // così un click ci passa sempre da qui, mai due dialoghi aperti insieme.
    triggerFileInput: function () {
      if (this.import_file) return;
      const input = this.$refs.fileInput as HTMLInputElement | undefined;
      if (input) input.click();
    },
    onFileDrop: function (evt: DragEvent) {
      this.isDraggingFile = false;
      if (this.import_file) return;
      const file = evt.dataTransfer && evt.dataTransfer.files && evt.dataTransfer.files[0];
      if (!file) return;
      const nome = file.name.toLowerCase();
      if (!nome.endsWith('.json') && !nome.endsWith('.csv')) {
        this.import_error = this.$t('buckets.dropzoneUnsupported') as string;
        return;
      }
      this.processImportFile(file);
    },
    openDeleteBucketModal: function (row: { bucketId: string | null; watcherId?: string | null; displayName: string }) {
      this.delete_bucket_selected = row.bucketId || null;
      this.delete_watcher_selected = row.watcherId || null;
      this.delete_display_label = row.bucketId || row.displayName;
    },
    cancelDeleteBucket: function () {
      this.delete_bucket_selected = null;
      this.delete_watcher_selected = null;
      this.delete_display_label = null;
    },
    // Elimina anche processo+cartella del watcher (non solo il bucket) —
    // senza questo, un watcher ancora attivo ricreerebbe da solo il
    // bucket appena cancellato al giro di heartbeat successivo, lasciando
    // in tabella una riga "fantasma" senza più alcun controllo (bug
    // segnalato dall'utente).
    deleteBucket: async function () {
      try {
        if (this.delete_watcher_selected) {
          await invoke('elimina_watcher_personalizzato', { id: this.delete_watcher_selected });
        }
        if (this.delete_bucket_selected) {
          await this.bucketsStore.deleteBucket({ bucketId: this.delete_bucket_selected });
        }
      } finally {
        this.cancelDeleteBucket();
        await this.refreshWatchersAndBuckets();
      }
    },
    importBuckets: async function (importFile) {
      const formData = new FormData();
      formData.append('buckets.json', importFile);
      const headers = { 'Content-Type': 'multipart/form-data' };
      // timeout: 0 = nessun limite — con uno storico molto lungo
      // l'importazione (dedup evento per evento lato server) può
      // richiedere minuti, ben oltre i 30s di default del client.
      return this.$aw.req.post('/0/import', formData, { headers, timeout: 0 });
    },
    // Riconosce un CSV esportato da "Esporta come CSV" (vedi export_csv:
    // bucket_id/bucket_type/client/hostname ripetuti su ogni riga) e lo
    // traduce nello stesso formato {"buckets": {...}} che l'endpoint di
    // import si aspetta già — riusa l'identica logica di merge/dedup
    // lato server, nessuna modifica al backend necessaria. Un CSV con
    // righe di bucket diversi (es. incollate a mano da più export) viene
    // comunque smistato correttamente, riga per riga.
    importCsvFile: async function (importFile: File) {
      const text = await importFile.text();
      const parsed = Papa.parse(text, { header: true, skipEmptyLines: true, dynamicTyping: true });
      const rows = parsed.data as Record<string, any>[];
      if (rows.length === 0) {
        throw new Error('CSV vuoto o senza intestazione riconoscibile');
      }
      // Richiesta esplicita: nessun valore "indovinato" quando una
      // colonna manca — un CSV modificato a mano senza tutte le colonne
      // richieste viene rifiutato per intero, invece di importare dati
      // con metadati inventati.
      const colonneRichieste = ['bucket_id', 'bucket_type', 'client', 'hostname', 'timestamp', 'duration'];
      const mancanti = colonneRichieste.filter(c => !(c in rows[0]));
      if (mancanti.length > 0) {
        throw new Error(
          `CSV non riconosciuto: mancano le colonne ${mancanti.join(', ')} (esportalo da questa pagina, non modificarlo a mano)`
        );
      }

      const buckets: Record<string, any> = {};
      for (const row of rows) {
        const bucketId = String(row.bucket_id ?? '').trim();
        const bucketType = String(row.bucket_type ?? '').trim();
        const client = String(row.client ?? '').trim();
        const hostname = String(row.hostname ?? '').trim();
        if (!bucketId || !bucketType || !client || !hostname) {
          throw new Error('CSV non valido: una riga ha bucket_id/bucket_type/client/hostname vuoti');
        }
        if (!buckets[bucketId]) {
          buckets[bucketId] = {
            id: bucketId,
            type: bucketType,
            client,
            hostname,
            events: [] as any[],
          };
        }
        const { bucket_id, bucket_type, client: _client, hostname: _hostname, timestamp, duration, ...data } =
          row;
        buckets[bucketId].events.push({
          timestamp,
          duration: Number(duration) || 0,
          data,
        });
      }

      return this.$aw.req.post(
        '/0/import',
        { buckets },
        { headers: { 'Content-Type': 'application/json' }, timeout: 0 }
      );
    },

    // Passo 1: raccoglie i dati di TUTTI i bucket (integrati e
    // personalizzati — il server non li distingue, sono tutti bucket
    // nel datastore, vedi export.rs) in memoria. Nessun limite di tempo
    // sulla richiesta: con uno storico molto lungo può richiedere
    // minuti. Notifica di sistema al termine, per chi nel frattempo ha
    // cambiato pagina o messo l'app in background.
    async startExportAllBuckets() {
      this.exportState = 'loading';
      this.exportError = null;
      try {
        const response = await this.$aw.req.get('/0/export', { timeout: 0 });
        const data = response.data;
        // Esclude i bucket dei watcher in WATCHER_ESCLUSI_DA_EXPORT (es.
        // screenshot: solo nomi di file, inutili fuori dall'app) — le
        // immagini vere restano su disco in ogni caso, non erano mai
        // incluse nel bucket.
        if (data && data.buckets) {
          data.buckets = Object.fromEntries(
            Object.entries(data.buckets).filter(
              ([, bucket]: [string, any]) => !WATCHER_ESCLUSI_DA_EXPORT.has(bucket.client)
            )
          );
        }
        this.exportedJsonData = JSON.stringify(data, null, 2);
        this.exportState = 'ready';
        try {
          await invoke('invia_notifica_generica', {
            titolo: this.$t('buckets.exportNotificationTitle') as string,
            corpo: this.$t('buckets.exportNotificationBody') as string,
          });
        } catch {
          // Non bloccante: i dati sono comunque pronti da scaricare.
        }
      } catch (err) {
        this.exportState = 'idle';
        this.exportError = 'Export failed, see aw-server logs for more info';
      }
    },
    // Passo 2: i dati sono già in memoria (passo 1) — qui si apre solo
    // il dialogo nativo "Salva con nome", nessuna nuova richiesta di rete.
    async downloadExportedJson() {
      if (!this.exportedJsonData) return;
      await downloadFile('aw-bucket-export.json', this.exportedJsonData, 'application/json');
      this.exportState = 'idle';
      this.exportedJsonData = null;
    },

    async export_csv(bucketId: string) {
      const bucket = await this.bucketsStore.getBucketWithEvents({ id: bucketId });
      const events = bucket.events;
      // Unione delle chiavi di TUTTI gli eventi, non solo del primo —
      // eventi con campi diversi (es. un watcher personalizzato che ha
      // cambiato formato nel tempo) altrimenti perdevano colonne in
      // silenzio, dato che Papa.unparse esporta solo le colonne
      // esplicitamente elencate.
      const datakeys = _.uniq(events.flatMap((e: any) => Object.keys(e.data)));
      // bucket_id/bucket_type/client/hostname ripetuti su ogni riga
      // (invece che in un'intestazione a parte) — richiesta esplicita:
      // ricaricando questo stesso CSV da "Importa bucket", l'app deve
      // riconoscere da solo a quale bucket appartiene. Ripetuti su ogni
      // riga invece che in un preambolo separato perché sopravvivono
      // anche se il file viene aperto e risalvato con Excel — un
      // preambolo a parte rischierebbe di essere alterato.
      const columns = ['bucket_id', 'bucket_type', 'client', 'hostname', 'timestamp', 'duration'].concat(
        datakeys
      );
      const data = events.map((e: any) => {
        return Object.assign(
          {
            bucket_id: bucket.id,
            bucket_type: bucket.type,
            client: bucket.client || '',
            hostname: bucket.hostname || '',
            // e.timestamp è un vero oggetto Date (vedi aw-client) — senza
            // toISOString() Papa.unparse lo stringifica con .toString(),
            // che produce un formato lungo dipendente dal fuso/locale
            // della macchina invece di un timestamp pulito e ordinabile.
            timestamp: e.timestamp.toISOString(),
            duration: e.duration,
          },
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

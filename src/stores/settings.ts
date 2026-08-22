import { defineStore } from 'pinia';
import { getClient } from '~/util/awclient';
import type { AppCategory } from '~/stores/appCategories';
import { SavedQuery } from '~/util/savedQueries';
import { View, defaultViews } from '~/stores/views';
import { Project } from '~/stores/projects';
import type { PrivacyFilterRule } from '~/util/privacyFilters';
import type { NotifyRule } from '~/util/notifyRules';
import { isEqual } from 'lodash';
import { AppLocale, i18n, isAppLocale, setAppLocale } from '~/i18n';

function jsonEq(a: any, b: any) {
  const jsonA = JSON.parse(JSON.stringify(a));
  const jsonB = JSON.parse(JSON.stringify(b));
  return isEqual(jsonA, jsonB);
}

let settingsLoadPromise: Promise<void> | null = null;

interface State {
  startOfDay: string;
  startOfWeek: string;
  // Purely decorative band drawn on the Home Timeline (the "lunch
  // break" hatch) — "HH:mm" strings, same format as startOfDay. Not
  // exposed in a settings UI yet (TODO, see BLUEPRINT.md section 8),
  // but lives here already so the Timeline reads a real setting
  // instead of a hardcoded 13:00–14:00.
  lunchBreakStart: string;
  lunchBreakEnd: string;
  // Ore di lavoro attese in una giornata — l'unico input dell'utente per
  // colorare il modulo Home "Attività (calendario)"
  // (visualizations/ActivityHeatmap.vue): ogni giorno viene colorato in
  // base a quanto il tempo non-AFK di quel giorno si avvicina a questo
  // obiettivo, non più relativo al giorno più attivo del semestre.
  dailyWorkHoursBudget: number;
  // Ogni quanti secondi aw-watcher-screenshot cattura il desktop — letta
  // direttamente dal watcher Python tramite le impostazioni di
  // aw-server (client.get_setting), non solo dalla webui. Nessuna UI in
  // Impostazioni ancora (TODO, vedi BLUEPRINT.md sezione 8), stesso
  // stato di lunchBreakStart/End qui sopra.
  screenshotIntervalSeconds: number;
  // Dopo quanti giorni aw-watcher-screenshot elimina da solo i file più
  // vecchi (stessa lettura diretta della impostazione, nessun bisogno
  // di riavvio — vedi ScreenshotSettings.vue).
  screenshotRetentionDays: number;
  theme: 'light' | 'dark' | 'auto';
  locale: string;

  always_active_apps: string[];
  privacy_filters: PrivacyFilterRule[];
  // Nuovo sistema semplice app→categoria (sostituisce del tutto il
  // vecchio sistema a regole regex ereditato da ActivityWatch upstream,
  // rimosso su richiesta esplicita dell'utente 2026-08-12 — vedi
  // BLUEPRINT.md sezione 3). Stessa chiave già scritta dalla
  // categorizzazione automatica AI lato Rust (`src-tauri/src/
  // categorization.rs`, chiave impostazioni "appCategories") — questo
  // campo è la stessa identica fonte, letta/scritta sia da lì che da
  // stores/appCategories.ts.
  appCategories: AppCategory[];
  views: View[];
  saved_queries: SavedQuery[];
  projects: Project[];
  // Regole di notifica personalizzate (Impostazioni → Notifiche) — vedi
  // util/notifyRules.ts per la forma, notifyRulesEngine.ts per come
  // vengono valutate. La regola "vpn" (cliente sconosciuto) è letta
  // anche lato Rust (vpn_notify.rs, tramite AppServer::get_setting) per
  // decidere se mandare quella notifica specifica.
  notifyRules: NotifyRule[];

  requestTimeout: number;

  // Interruttore principale della sezione Sviluppatore (Impostazioni →
  // Sviluppatore) — spento di default, richiede conferma esplicita per
  // essere acceso (vedi DeveloperSettings.vue). Spegnerlo riporta anche
  // tutte le singole opzioni sotto (es. devtoolsEnabled) al loro
  // default sicuro, così non restano abilitate "in silenzio" dopo che
  // la sezione è stata richiusa.
  developerModeEnabled: boolean;
  // DevTools (Ispeziona/F12) e tasto destro nella finestra dell'app.
  // Guardato lato JS in main.js (contextmenu/keydown) — vedi
  // util/devtoolsGuard.ts per il motivo per cui è lì e non solo qui.
  devtoolsEnabled: boolean;
  // Pulsante "Apri" su ogni riga della tabella Sorgenti dati (Watchers) —
  // spento di default: è un pannello tecnico (timeline grezza + lista
  // eventi) non pensato per l'uso quotidiano, va acceso solo per
  // diagnosticare un problema. Vedi views/Buckets.vue.
  rawDataDiagnosticsEnabled: boolean;

  // Whether to hide visualizations that lack required data (default: off)
  hideUnsupportedVisualizations: boolean;

  // Nascondere i moduli della Home (Top File Excel/VPN/VoiSpeed/Editor)
  // quando non hanno dati per il periodo mostrato, invece di lasciarli
  // visibili col loro stato vuoto — vedi SelectableVisualization.vue's
  // hasNoDataForPeriod(). Default true (comportamento già in uso prima
  // che diventasse un'impostazione). L'utente ha segnalato che entrando
  // in "Modifica moduli" le card nascoste ricompaiono (necessario, per
  // poterle gestire), causando uno spostamento percepibile delle altre
  // — questa impostazione gli dà la scelta di disattivare del tutto il
  // comportamento invece di doverlo subire.
  hideEmptyModules: boolean;
  // Stessa idea, per le corsie della Timeline (VPN/Claude/VSCode/Excel/
  // VoiSpeed/Browser) — vedi HomeTimelineSection.vue's rebuildLanes().
  // Default true, stessa ragione.
  hideEmptyTimelineLanes: boolean;

  // Se un aggiornamento trovato va scaricato subito in background
  // (popup "Download in corso" → "Riavvia per aggiornare") oppure solo
  // segnalato ("Update disponibile, clicca per aggiornare") lasciando
  // all'utente la scelta di quando scaricarlo — vedi UpdatePopup.vue,
  // stores/updater.ts. Richiesta esplicita: l'aggiornamento silenzioso
  // è "un po' borderline" per un progetto open source, quindi resta
  // disattivabile. Default true (comportamento stile Claude Desktop).
  autoUpdateEnabled: boolean;

  // Vero il valore reale è nel registro di Windows (vedi autostart.rs,
  // HKCU\...\Run), non qui — questo campo serve solo a ricordare "il
  // default (acceso) è già stato applicato una volta", così un utente
  // che lo spegne da Impostazioni non se lo ritrova riacceso al
  // prossimo avvio (vedi App.vue). Richiesta esplicita: acceso di
  // default, ma resta una scelta dell'utente da lì in poi, non forzata
  // ad ogni avvio.
  autostartDefaultApplied: boolean;

  // Set to true if settings loaded
  _loaded: boolean;
}

export const useSettingsStore = defineStore('settings', {
  state: (): State => ({
    startOfDay: '04:00',
    startOfWeek: 'Monday',
    lunchBreakStart: '13:00',
    lunchBreakEnd: '14:00',
    dailyWorkHoursBudget: 8,
    screenshotIntervalSeconds: 30,
    screenshotRetentionDays: 14,

    // Richiesta esplicita: la primissima apertura (nessuna impostazione
    // salvata ancora) deve mostrare subito il tema scuro, non "auto"
    // (che su questa macchina risolveva a chiaro). L'utente resta
    // comunque libero di cambiarlo in qualunque momento, dal toggle in
    // sidebar o dalle Impostazioni — questo è solo il punto di partenza.
    theme: 'dark',
    // Italiano come lingua di partenza — l'app è pensata prima di
    // tutto per un utente italiano (vedi BLUEPRINT.md), coerente col
    // resto dell'interfaccia già scritta in italiano.
    locale: 'it',

    always_active_apps: [],
    privacy_filters: [],
    appCategories: [],
    views: defaultViews,
    saved_queries: [],
    projects: [],
    notifyRules: [],

    requestTimeout: 30,
    developerModeEnabled: false,
    devtoolsEnabled: false,
    rawDataDiagnosticsEnabled: false,
    hideUnsupportedVisualizations: false,
    hideEmptyModules: false,
    hideEmptyTimelineLanes: false,
    autoUpdateEnabled: true,
    autostartDefaultApplied: false,

    _loaded: false,
  }),

  getters: {
    loaded(state: State) {
      return state._loaded;
    },
  },

  actions: {
    async ensureLoaded() {
      if (this.loaded) {
        return;
      }

      if (!settingsLoadPromise) {
        settingsLoadPromise = this.load().finally(() => {
          settingsLoadPromise = null;
        });
      }

      await settingsLoadPromise;
    },
    async load({ save }: { save?: boolean } = {}) {
      if (typeof localStorage === 'undefined') {
        console.error('localStorage is not supported');
        return;
      }
      const client = getClient();

      // Fetch from server, fall back to localStorage
      const server_settings = await client.get_settings();

      // Build a unified map: server value wins, localStorage is fallback.
      // Skip keys that are missing from BOTH sources — otherwise `null` from
      // localStorage.getItem overrides the defaults defined in `state()`.
      const storage: Record<string, unknown> = {};
      const used = new Set<string>();

      // 1. Server settings take priority
      for (const key of Object.keys(server_settings)) {
        if (key.startsWith('_')) continue;
        if (key === 'locale' && !isAppLocale(server_settings[key])) {
          console.warn('Ignoring invalid locale from server:', server_settings[key]);
          continue;
        }
        storage[key] = server_settings[key];
        used.add(key);
      }

      // 2. localStorage fills in gaps, but skip missing keys (null)
      for (const key of Object.keys(localStorage)) {
        if (key.startsWith('_') || used.has(key)) continue;
        const raw = localStorage.getItem(key);
        if (raw === null || raw === 'null') continue; // key absent or stored as null → keep state() default

        // Keys ending with 'Data' are JSON-serialized objects in localStorage
        const isJsonKey =
          key.endsWith('Data') ||
          key == 'views' ||
          key == 'appCategories' ||
          key == 'saved_queries' ||
          key == 'projects';
        try {
          if (isJsonKey) {
            const parsed = JSON.parse(raw);
            storage[key] = parsed;
          } else if (raw === 'true' || raw === 'false') {
            storage[key] = raw === 'true';
          } else if (key === 'locale') {
            if (isAppLocale(raw)) {
              storage[key] = raw;
            } else {
              console.warn('Ignoring invalid locale from storage:', raw);
            }
          } else {
            storage[key] = raw;
          }
        } catch (e) {
          console.error('failed to parse', key, raw, e);
        }
      }
      this.$patch({ ...storage, _loaded: true });

      const localeFromServer = 'locale' in server_settings;
      const localeFromLocalStorage = localStorage.getItem('locale') != null;

      if ((localeFromServer || localeFromLocalStorage) && isAppLocale(this.locale)) {
        setAppLocale(this.locale);
      } else if (isAppLocale(i18n.locale)) {
        this.$patch({ locale: i18n.locale as AppLocale });
      }

      // Since `requestTimeout` is used to initialize the client, we need to set it again
      // https://github.com/ActivityWatch/activitywatch/issues/979
      client.req.defaults.timeout = this.requestTimeout * 1000;

      if (save) {
        await this.save();
      }
    },
    async save() {
      // Important check, to avoid saving settings before they are loaded (potentially overwriting them with defaults)
      if (!this.loaded) {
        console.error('Settings not loaded, not saving');
        return;
      }
      // We want to avoid saving to localStorage to not accidentally mess up pre-migration data
      // For example, if the user is using several browsers, and opened in their non-main browser on first run after upgrade.
      const saveToLocalStorage = false;

      // Save to localStorage and backend
      // NOTE: localStorage deprecated, will be removed in future
      const client = getClient();

      // Fetch current settings from server
      const server_settings = await client.get_settings();

      // Save settings
      for (const key of Object.keys(this.$state)) {
        // Skip keys starting with underscore, as they are local to the vuex store.
        if (key.startsWith('_')) {
          continue;
        }

        const value = this.$state[key];

        // Save to localStorage
        // NOTE: we always save the theme to localStorage, since it is used before the settings are loaded
        if (saveToLocalStorage || key == 'theme') {
          if (typeof value === 'object') {
            localStorage.setItem(key, JSON.stringify(value));
          } else {
            localStorage.setItem(key, value);
          }
        }

        // Save changed settings to backend
        if (server_settings[key] === undefined || !jsonEq(server_settings[key], value)) {
          if (server_settings[key] === undefined && value === false) {
            // Skip saving settings that are set to false and not already saved on the server
            continue;
          }
          console.log('Saving', { [key]: value });
          //console.log('Was:', server_settings[key]);
          //console.log('Now:', value);
          await client.req.post('/0/settings/' + key, value, {
            headers: {
              'Content-Type': 'application/json',
            },
          });
        }
      }

      // After save, reload
      await this.load({ save: false });
    },
    async update(new_state: Record<string, any>) {
      console.log('Updating state', new_state);
      await this.ensureLoaded();
      this.$patch(new_state);
      await this.save();
    },
  },
});

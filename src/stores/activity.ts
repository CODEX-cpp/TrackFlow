import { defineStore } from 'pinia';
import moment from 'moment';
import * as _ from 'lodash';
import { IEvent } from '~/util/interfaces';

import queries from '~/queries';
import { get_day_start_with_offset } from '~/util/time';
import {
  TimePeriod,
  dateToTimeperiod,
  timeperiodToStr,
  timeperiodsAroundTimeperiod,
} from '~/util/timeperiod';

import { useSettingsStore } from '~/stores/settings';
import { useBucketsStore } from '~/stores/buckets';

import { getClient } from '~/util/awclient';

function timeperiodStrsAroundTimeperiod(timeperiod: TimePeriod): string[] {
  return timeperiodsAroundTimeperiod(timeperiod).map(timeperiodToStr);
}

// Compares the fields that actually define "what to query" — everything
// except the force/background control flags, which say HOW to (re)query
// the same context, not what the context is. Used by ensure_loaded() to
// decide whether this is a genuine new context (different day/host/
// filters) or just a repeat call for the one already loaded — a plain
// `!==` there always failed because every caller builds a brand new
// options object literal per call, so the "already loaded" cache never
// actually hit (bug report: every 30s auto-refresh, and every unrelated
// re-render, silently redid the full query chain).
function sameQueryContext(a: QueryOptions | null | undefined, b: QueryOptions): boolean {
  if (!a) return false;
  return (
    a.host === b.host &&
    a.date === b.date &&
    timeperiodToStr(a.timeperiod as TimePeriod) === timeperiodToStr(b.timeperiod as TimePeriod) &&
    a.filter_afk === b.filter_afk &&
    a.include_audible === b.include_audible &&
    a.include_stopwatch === b.include_stopwatch &&
    _.isEqual(a.always_active_apps, b.always_active_apps)
  );
}

export interface QueryOptions {
  host: string;
  date?: string;
  timeperiod?: TimePeriod;
  filter_afk?: boolean;
  include_audible?: boolean;
  include_stopwatch?: boolean;
  force?: boolean;
  always_active_apps?: string[];
  // Set by a periodic auto-refresh re-querying the SAME context (same
  // host/day), not a genuine new one (date/host change). Skips the
  // null-out in start_loading() so the modules keep showing their last
  // values (bars, rankings) until the new ones are ready, instead of
  // flashing empty/"Loading..." on every tick — see ensure_loaded().
  background?: boolean;
}

interface State {
  loaded: boolean;

  window: {
    available: boolean;
    top_apps: IEvent[];
    top_titles: IEvent[];
  };

  browser: {
    available: boolean;
    duration: number;
    top_urls: IEvent[];
    top_domains: IEvent[];
    top_titles: IEvent[];
  };

  editor: {
    available: boolean;
    duration: number;
    top_files: IEvent[];
    top_projects: IEvent[];
    top_languages: IEvent[];
  };

  excel: {
    available: boolean;
    duration: number;
    top_files: IEvent[];
  };

  voispeed: {
    available: boolean;
    duration: number;
    top_contacts: IEvent[];
  };

  active: {
    available: boolean;
    duration: number;
    // non-afk events (no detail data) for the current period
    events: IEvent[];
    // Aggregated events for current and past periods
    history: Record<any, IEvent[]>;
  };

  android: {
    available: boolean;
  };

  ios: {
    available: boolean;
  };

  stopwatch: {
    available: boolean;
    top_stopwatches: IEvent[];
  };

  query_options?: QueryOptions;

  // Can't this be handled in bucketStore?
  buckets: {
    loaded: boolean;
    afk: string[];
    window: string[];
    editor: string[];
    excel: string[];
    voispeed: string[];
    browser: string[];
    android: string[];
    stopwatch: string[];
  };
}

export const useActivityStore = defineStore('activity', {
  // initial state
  state: (): State => ({
    // set to true once loading has started
    loaded: false,

    window: {
      available: false,
      top_apps: [],
      top_titles: [],
    },

    browser: {
      available: false,
      duration: 0,
      top_domains: [],
      top_urls: [],
      top_titles: [],
    },

    editor: {
      available: false,
      duration: 0,
      top_files: [],
      top_languages: [],
      top_projects: [],
    },

    excel: {
      available: false,
      duration: 0,
      top_files: [],
    },

    voispeed: {
      available: false,
      duration: 0,
      top_contacts: [],
    },

    active: {
      available: false,
      duration: 0,
      // non-afk events (no detail data) for the current period
      events: [],
      // Aggregated events for current and past periods
      history: {},
    },

    android: {
      available: false,
    },

    ios: {
      available: false,
    },

    stopwatch: {
      available: false,
      top_stopwatches: [],
    },

    query_options: null,

    buckets: {
      loaded: false,
      afk: [],
      window: [],
      editor: [],
      excel: [],
      voispeed: [],
      browser: [],
      android: [],
      stopwatch: [],
    },
  }),

  actions: {
    async ensure_loaded(query_options: QueryOptions) {
      const settingsStore = useSettingsStore();
      await settingsStore.ensureLoaded();

      const bucketsStore = useBucketsStore();

      console.info('Query options: ', query_options);
      if (this.loaded) {
        getClient().abort();
      }
      // Resolved up front (not after the same-context check below) so
      // both sides of that comparison always have a real timeperiod to
      // compare, whether the caller passed one directly or only a date.
      if (!query_options.timeperiod) {
        query_options.timeperiod = dateToTimeperiod(query_options.date, settingsStore.startOfDay);
      }
      if (
        !this.loaded ||
        !sameQueryContext(this.query_options, query_options) ||
        query_options.force
      ) {
        if (query_options.background) {
          // Same context, just re-querying for fresh data — keep
          // showing the last values instead of nulling everything out
          // first. Each query_*_completed action below overwrites the
          // relevant fields in place once its new data arrives, so
          // module lists/bars update smoothly (and — for anything keyed
          // by name rather than array index — animate) instead of
          // flashing to an empty state on every auto-refresh tick.
          this.loaded = true;
          this.query_options = query_options;
        } else {
          this.start_loading(query_options);
        }

        await bucketsStore.ensureLoaded();
        await this.get_buckets(query_options);

        // TODO: These queries can actually run in parallel, but since server won't process them in parallel anyway we won't.
        this.set_available();

        if (this.window.available) {
          await this.query_desktop_full(query_options);
        } else if (this.android.available) {
          await this.query_android(query_options);
        } else {
          console.log(
            'Cannot query windows as we are missing either an afk/window bucket pair or an android bucket'
          );
          this.query_window_completed();
        }

        if (this.active.available) {
          await this.query_active_history(query_options);
        } else if (this.android.available) {
          await this.query_active_history_android(query_options);
        } else {
          console.log('Cannot call query_active_history as we do not have an afk bucket');
          await this.query_active_history_completed();
        }

        if (this.editor.available) {
          await this.query_editor(query_options);
        } else {
          console.log('Cannot call query_editor as we do not have any editor buckets');
          await this.query_editor_completed();
        }

        if (this.excel.available) {
          await this.query_excel(query_options);
        } else {
          console.log('Cannot call query_excel as we do not have any excel buckets');
          await this.query_excel_completed();
        }

        if (this.voispeed.available) {
          await this.query_voispeed(query_options);
        } else {
          console.log('Cannot call query_voispeed as we do not have any voispeed buckets');
          await this.query_voispeed_completed();
        }
      } else {
        console.warn(
          'ensure_loaded called twice with same query_options but without query_options.force = true, skipping...'
        );
      }
    },

    async query_android({ timeperiod }: QueryOptions) {
      const periods = [timeperiodToStr(timeperiod)];

      // Prefer the ScreenTime bucket when both Android watcher and ScreenTime buckets
      // exist for the same host (hybrid case). Without this, android[0] is the Android
      // watcher bucket, isIos is false, and ScreenTime data is never processed.
      const iosBucket = this.buckets.android.find((id: string) =>
        id.startsWith('aw-import-screentime')
      );
      const selectedBucket = iosBucket || this.buckets.android[0];

      const q = queries.appQuery(selectedBucket);
      const data = await getClient().query(periods, q).catch(this.errorHandler);

      // Post-process for iOS compatibility (swap app <-> title)
      const isIos = !!iosBucket;

      if (isIos && data && data[0] && data[0].title_events) {
        // Build bundle ID → human name lookup from title_events before modifying them.
        // title_events has 'app' = bundle ID and 'title' = human-readable name.
        // For ScreenTime imports each bundle ID maps to exactly one human-readable name,
        // so the server's top-100 title groups and top-100 app groups rank identically —
        // this lookup covers every app_events entry. Apps with no title fall back to the
        // raw bundle ID via the `|| bundleId` guard below.
        const bundleIdToName: Record<string, string> = {};
        data[0].title_events.forEach((e: IEvent) => {
          if (e.data.title && !bundleIdToName[e.data.app]) {
            bundleIdToName[e.data.app] = e.data.title;
          }
        });

        // Remap title_events: swap bundle ID → human name, store bundle ID as classname.
        data[0].title_events.forEach((e: IEvent) => {
          const originalApp = e.data.app;
          e.data.classname = originalApp; // Bundle ID (e.g. com.google.ios.youtube)
          e.data.app = e.data.title || originalApp; // Human name (e.g. YouTube), or bundle ID if absent
        });

        // Remap app_events directly using the lookup, preserving the server's complete aggregation.
        // Re-aggregating from title_events would corrupt totals when there are >100 distinct apps,
        // because title_events is capped at 100 entries by the query.
        if (data[0].app_events) {
          data[0].app_events.forEach((e: IEvent) => {
            const bundleId = e.data.app;
            e.data.classname = bundleId;
            e.data.app = bundleIdToName[bundleId] || bundleId;
          });
        }
      }

      this.query_window_completed(data[0]);
    },

    async reset() {
      getClient().abort();
      this.query_window_completed({});
      this.query_browser_completed({});
      this.query_editor_completed({});
      this.query_excel_completed({});
      this.query_voispeed_completed({});
    },

    async query_desktop_full({
      timeperiod,
      filter_afk,
      include_audible,
      include_stopwatch,
      always_active_apps,
    }: QueryOptions) {
      const periods = [timeperiodToStr(timeperiod)];

      const q = queries.fullDesktopQuery({
        bid_window: this.buckets.window[0],
        bid_afk: this.buckets.afk[0],
        bid_browsers: this.buckets.browser,
        bid_stopwatch:
          include_stopwatch && this.buckets.stopwatch.length > 0
            ? this.buckets.stopwatch[0]
            : undefined,
        filter_afk,
        include_audible,
        always_active_apps,
      });
      const data = await getClient().query(periods, q, {
        name: 'fullDesktopQuery',
        verbose: true,
      });
      this.query_window_completed(data[0].window);
      this.query_browser_completed(data[0].browser);
      if (include_stopwatch) {
        this.query_stopwatch_completed(data[0].stopwatch);
      }
    },

    async query_editor({ timeperiod }) {
      const periods = [timeperiodToStr(timeperiod)];
      const q = queries.editorActivityQuery(this.buckets.editor);
      const data = await getClient().query(periods, q, {
        name: 'editorActivityQuery',
        verbose: true,
      });
      this.query_editor_completed(data[0]);
    },

    async query_excel({ timeperiod }) {
      const periods = [timeperiodToStr(timeperiod)];
      const q = queries.excelActivityQuery(this.buckets.excel);
      const data = await getClient().query(periods, q, {
        name: 'excelActivityQuery',
        verbose: true,
      });
      this.query_excel_completed(data[0]);
    },

    async query_voispeed({ timeperiod }) {
      const periods = [timeperiodToStr(timeperiod)];
      const q = queries.voispeedActivityQuery(this.buckets.voispeed);
      const data = await getClient().query(periods, q, {
        name: 'voispeedActivityQuery',
        verbose: true,
      });
      this.query_voispeed_completed(data[0]);
    },

    async query_active_history({ timeperiod }: QueryOptions) {
      // Filter out periods that are already in the history, and that are in the future
      const periods = timeperiodStrsAroundTimeperiod(timeperiod).filter(tp_str => {
        return (
          !_.includes(this.active.history, tp_str) && new Date(tp_str.split('/')[0]) < new Date()
        );
      });
      const afk_buckets: string[] = [this.buckets.afk[0]];
      const query = queries.activityQuery(afk_buckets);
      const data = await getClient().query(periods, query, {
        name: 'activityQuery',
        verbose: true,
      });
      const active_history = _.zipObject(
        periods,
        _.map(data, pair => _.filter(pair, e => e.data.status == 'not-afk'))
      );
      this.query_active_history_completed({ active_history });
    },

    async query_active_history_android({ timeperiod }: QueryOptions) {
      const periods = timeperiodStrsAroundTimeperiod(timeperiod).filter(tp_str => {
        return !_.includes(this.active.history, tp_str);
      });
      // Prefer ScreenTime bucket over Android watcher for consistency with query_android
      const iosOrAndroidBucket =
        this.buckets.android.find((id: string) => id.startsWith('aw-import-screentime')) ||
        this.buckets.android[0];
      const data = await getClient().query(
        periods,
        queries.activityQueryAndroid(iosOrAndroidBucket)
      );
      const active_history = _.zipObject(periods, data);
      const active_history_events = _.mapValues(
        active_history,
        (duration: number, key): [IEvent] => {
          return [{ timestamp: key.split('/')[0], duration, data: { status: 'not-afk' } }];
        }
      );
      this.query_active_history_completed({ active_history: active_history_events });
    },

    set_available(this: State) {
      // TODO: Move to bucketStore on a per-host basis?
      this.window.available = this.buckets.afk.length > 0 && this.buckets.window.length > 0;
      this.browser.available =
        this.buckets.afk.length > 0 &&
        this.buckets.window.length > 0 &&
        this.buckets.browser.length > 0;
      this.active.available = this.buckets.afk.length > 0;
      this.editor.available = this.buckets.editor.length > 0;
      this.excel.available = this.buckets.excel.length > 0;
      this.voispeed.available = this.buckets.voispeed.length > 0;
      this.android.available = this.buckets.android.length > 0;
      this.ios.available = this.buckets.android.some(id => id.startsWith('aw-import-screentime'));
      this.stopwatch.available = this.buckets.stopwatch.length > 0;
    },

    async get_buckets(this: State, { host }) {
      // TODO: Move to bucketStore on a per-host basis?
      const bucketsStore = useBucketsStore();
      this.buckets.afk = bucketsStore.bucketsAFK(host);
      this.buckets.window = bucketsStore.bucketsWindow(host);
      this.buckets.android = bucketsStore.bucketsAndroid(host);
      this.buckets.browser = bucketsStore.bucketsBrowser(host);
      this.buckets.editor = bucketsStore.bucketsEditor(host);
      this.buckets.excel = bucketsStore.bucketsExcel(host);
      this.buckets.voispeed = bucketsStore.bucketsVoispeed(host);
      this.buckets.stopwatch = bucketsStore.bucketsStopwatch(host);

      console.log('Available buckets: ', this.buckets);
      this.buckets.loaded = true;
    },

    // mutations
    start_loading(this: State, query_options: QueryOptions) {
      this.loaded = true;
      this.query_options = query_options;

      // Resets the store state while waiting for new query to finish
      this.window.top_apps = null;
      this.window.top_titles = null;

      this.browser.duration = 0;
      this.browser.top_domains = null;
      this.browser.top_urls = null;
      this.browser.top_titles = null;

      this.editor.duration = 0;
      this.editor.top_files = null;
      this.editor.top_languages = null;
      this.editor.top_projects = null;

      this.excel.duration = 0;
      this.excel.top_files = null;

      this.voispeed.duration = 0;
      this.voispeed.top_contacts = null;

      this.active.duration = null;

      // Ensures that active history isn't being fully reloaded on every date change
      // (see caching done in query_active_history and query_active_history_android)
      // FIXME: Better detection of when to actually clear (such as on force reload, hostname change)
      if (Object.keys(this.active.history).length === 0) {
        this.active.history = {};
      }
    },

    query_window_completed(
      this: State,
      data = { app_events: [], title_events: [], active_events: [], duration: 0 }
    ) {
      this.window.top_apps = data.app_events;
      this.window.top_titles = data.title_events;
      this.active.duration = data.duration;
      this.active.events = data.active_events;
    },

    query_browser_completed(
      this: State,
      data = { domains: [], urls: [], titles: [], duration: 0 }
    ) {
      this.browser.top_domains = data.domains;
      this.browser.top_urls = data.urls;
      this.browser.top_titles = data.titles;
      this.browser.duration = data.duration;
    },

    query_stopwatch_completed(this: State, data = { stopwatch_events: [] }) {
      this.stopwatch.top_stopwatches = data.stopwatch_events;
    },

    query_editor_completed(
      this: State,
      data = { duration: 0, files: [], languages: [], projects: [] }
    ) {
      this.editor.duration = data.duration;
      this.editor.top_files = data.files;
      this.editor.top_languages = data.languages;
      this.editor.top_projects = data.projects;
    },

    query_excel_completed(this: State, data = { duration: 0, files: [] }) {
      this.excel.duration = data.duration;
      this.excel.top_files = data.files;
    },

    query_voispeed_completed(this: State, data = { duration: 0, contacts: [] }) {
      this.voispeed.duration = data.duration;
      this.voispeed.top_contacts = data.contacts;
    },

    query_active_history_completed(this: State, { active_history } = { active_history: {} }) {
      this.active.history = {
        ...this.active.history,
        ...active_history,
      };
    },
  },
});

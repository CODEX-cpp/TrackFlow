import 'core-js/stable';
import 'regenerator-runtime/runtime';

import Vue from 'vue';

// Load the Bootstrap CSS
import BootstrapVue from 'bootstrap-vue';
import 'bootstrap/dist/css/bootstrap.css';
import 'bootstrap-vue/dist/bootstrap-vue.css';
Vue.use(BootstrapVue);

import { Datetime } from 'vue-datetime';
import 'vue-datetime/dist/vue-datetime.css';
Vue.component('datetime', Datetime);

// Load the Varela Round font
import 'typeface-varela-round';

// Load the main style
import './style/style.scss';
import './style/theme.css';

// Loads all the filters
import './util/filters.js';

// Internationalization
import { i18n } from './i18n';

// Sets up the routing and the base app (using vue-router)
import router from './route.js';

// Locale di moment (incluso l'italiano) registrata e impostata in
// ./i18n/index.ts, importato più sopra — la copia che viveva qui era
// ridondante (e per giunta bacata: hardcodava sempre 'it' ignorando la
// lingua salvata) e chiamava `import 'moment/locale/it'` nella forma
// side-effect-only che si è rivelata inaffidabile nella build di
// produzione (vedi il commento in i18n/index.ts per i dettagli).

// Sets up the pinia store
import pinia from './stores';

// Register Font Awesome icon component
Vue.component('icon', () => import('vue-awesome/components/Icon.vue'));

// General components
Vue.component('error-boundary', () => import('./components/ErrorBoundary.vue'));
Vue.component('input-timeinterval', () => import('./components/InputTimeInterval.vue'));
Vue.component('aw-sidebar', () => import('./components/Sidebar.vue'));
Vue.component('aw-topbar', () => import('./components/Topbar.vue'));
Vue.component('aw-selectable-vis', () => import('./components/SelectableVisualization.vue'));
Vue.component('aw-query-options', () => import('./components/QueryOptions.vue'));
Vue.component('aw-bucket-timeline', () => import('./components/BucketTimeline.vue'));

// Visualization components
Vue.component('aw-summary', () => import('./visualizations/Summary.vue'));
Vue.component('aw-top-summary', () => import('./visualizations/TopSummary.vue'));
Vue.component('aw-eventlist', () => import('./visualizations/EventList.vue'));
Vue.component('aw-category-bar', () => import('./visualizations/CategoryBar.vue'));
Vue.component('aw-timeline-barchart', () => import('./visualizations/TimelineBarChart.vue'));
Vue.component('aw-custom-vis', () => import('./visualizations/CustomVisualization.vue'));
Vue.component('aw-custom-watcher-view', () => import('./visualizations/CustomWatcherView.vue'));

// A mixin to make async method errors propagate
import asyncErrorCapturedMixin from './mixins/asyncErrorCaptured.js';
Vue.mixin(asyncErrorCapturedMixin);

// Set the PRODUCTION constant
// FIXME: Thould follow Vue convention and start with a $.
Vue.prototype.PRODUCTION = PRODUCTION;
Vue.prototype.COMMIT_HASH = COMMIT_HASH;

// Set the $isAndroid constant
Vue.prototype.$isAndroid = process.env.VUE_APP_ON_ANDROID;

// Create an instance of AWClient as this.$aw
// NOTE: needs to be created before the Vue app is created,
//       since stores rely on it having been run.
import { createClient, getClient, configureClient } from './util/awclient';
createClient();

// Setup Vue app
import App from './App.vue';
new Vue({
  el: '#app',
  router: router,
  i18n,
  render: h => h(App),
  pinia,
});

// Set the $aw global
Vue.prototype.$aw = getClient();

// Must be run after vue init since it relies on the settings store
configureClient();

// Pulls in whatever app icons/colors/names aw-watcher-app-icons has
// discovered since this build was made (see appNames.ts's header for
// why this can't just be baked into the bundle). Best-effort: if it
// fails (server not serving /pages/app-data/ yet), the baseline
// bundled at build time stays in place.
import { refreshDynamicAppData } from './util/appNames';
refreshDynamicAppData();

// Stato acceso/spento dei watcher (sottomenu "Moduli" della tray) —
// serve alla Home per nascondere le corsie della Timeline di feature
// non usate, vedi util/modulesConfig.ts.
import { refreshModulesConfig } from './util/modulesConfig';
refreshModulesConfig();

// Blocco tasto destro/F12 quando l'opzione DevTools (Impostazioni →
// Sviluppatore) è spenta — installato una sola volta qui, reattivo ai
// cambi dell'impostazione da solo (legge lo store ad ogni evento).
import { installDevtoolsGuard } from './util/devtoolsGuard';
installDevtoolsGuard();

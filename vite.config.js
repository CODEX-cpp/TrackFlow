import path from 'node:path';
import child_process from 'node:child_process';
import { defineConfig, loadEnv } from 'vite';
import vue from '@vitejs/plugin-vue2';

// Nessun repository git nel progetto (rimosso deliberatamente, vedi
// BLUEPRINT.md) — prima andava in errore duro e bloccava ogni build.
// "unknown" come fallback finché non verrà inizializzato un repository
// vero in una fase successiva.
let COMMIT_HASH = 'unknown';
try {
  COMMIT_HASH = child_process.execSync('git rev-parse --short HEAD').toString().trim();
} catch {
  // nessun repository git — resta "unknown"
}

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), '');
  const PRODUCTION = mode === 'production';

  // Auto-injects /src/main.js into index.html on a new line after the one which has VITE_AUTOINJECT
  const autoInject = () => {
    return {
      name: 'html-transform',
      transformIndexHtml: {
        order: 'pre',
        handler(html) {
          const pattern = /<!--.*VITE_AUTOINJECT.*-->/;
          // check if the pattern exists in the html, if not, throw error
          if (!pattern.test(html)) {
            throw new Error(`Could not find pattern ${pattern} in the html file`);
          }
          return html.replace(
            pattern,
            '<!-- Vite injected! --><script type="module" src="/src/main.js"></script>'
          );
        },
      },
    };
  };

  // Return the configuration
  return {
    build: {
      // Temporary compat knob for #867/#869: the forced esbuild 0.28.x security
      // upgrade cannot lower one destructuring pattern to the old default targets.
      // Drop this once Vite can consume a patched esbuild line without overrides.
      target: 'es2022',
      rollupOptions: {
        output: {
          // Bug reale trovato dall'utente: la topbar mostrava i giorni
          // della settimana in inglese ("Wed 5 Aug") anche con l'app in
          // italiano. Causa: senza questo, Rollup inlinea una copia
          // PRIVATA di 'moment' dentro ogni chunk lazy-loaded diverso
          // (Topbar.vue, CalendarPicker.vue, ecc. sono tutti
          // `() => import(...)`) invece di farli puntare tutti alla
          // stessa istanza — moment tiene la locale attiva come stato
          // globale sull'istanza del modulo, quindi `moment.locale('it')`
          // chiamato all'avvio (main.js/i18n/index.ts) non aveva alcun
          // effetto sulla copia privata di moment usata da quei chunk
          // (confermato: `moment.locales()` lì dentro conteneva solo
          // `["en"]`, la locale 'it' non era mai stata registrata su
          // quella copia). Forzare 'moment' in un chunk manuale dedicato
          // garantisce che ogni importatore, eager o lazy, condivida la
          // stessa istanza — e quindi la stessa locale.
          // Forma a funzione (non l'oggetto { moment: ['moment'] } provato
          // prima, insufficiente da solo): serve intercettare OGNI file
          // sotto node_modules/moment, incluse le singole locale
          // (moment/locale/it.js) — altrimenti Rollup avvolge ciascuna
          // con la propria interop CommonJS separata, ricreando di fatto
          // una seconda copia privata dell'intera libreria moment con un
          // registro locale tutto suo (mai raggiunta da moment.locale('it')
          // chiamato altrove) anche quando la copia "principale" finisce
          // nello stesso chunk dichiarato sopra.
          manualChunks(id) {
            if (id.includes('/node_modules/moment/')) {
              return 'moment';
            }
          },
        },
      },
    },
    optimizeDeps: {
      // The dep pre-bundler uses its own esbuild target, separate from `build.target`
      // above. Without this it defaults to an older baseline that the same esbuild
      // version can't lower some deps' (pinia, luxon) destructuring patterns to.
      esbuildOptions: { target: 'es2022' },
    },
    plugins: [
      autoInject(),
      vue(),
    ],
    server: {
      port: 27180,
      // TODO: Fix this.
      // Breaks a bunch of style-related stuff etc.
      // We'd need to move in the entire CSP config in here (not just the default-src) if we want to use this.
      //headers: {
      //  'Content-Security-Policy': PRODUCTION ? "default-src 'self'" : "default-src 'self' *:5666",
      //},
      watch: {
        // aw-watcher-app-icons rewrites these two files on its own 30s
        // poll cycle (it's meant to — that's how newly-seen apps' icons/
        // colors get picked up live). Both live under src/ so Vite's
        // dev-server watcher treats every rewrite as a real source
        // change and hot-reloads every module that imports them
        // (appNames.ts, and transitively HomeTimelineSection.vue,
        // SelectableVisualization.vue, TimelineBlockDetailModal.vue) —
        // which for a .vue SFC's <script> block means a full component
        // remount, not an in-place patch. Explicit bug report: this
        // silently closed the Timeline's block-detail popup every 30s
        // while dev-testing, since the remounted component starts over
        // with no selected block. Only affects `npx vite` dev mode —
        // the production build has no live file-watching at all.
        ignored: ['**/src/util/appIconColors.json', '**/src/util/appAutoNames.json'],
      },
    },
    publicDir: './static',
    resolve: {
      alias: { '~': path.resolve(__dirname, 'src') },
    },
    define: {
      PRODUCTION,
      // esbuild's define does raw text substitution: values must be valid JS
      // source (i.e. JSON.stringify'd strings), not bare/undefined env values,
      // or the replacement corrupts every file esbuild transforms.
      AW_SERVER_URL: JSON.stringify(env.AW_SERVER_URL || ''),
      COMMIT_HASH: JSON.stringify(COMMIT_HASH),
      'process.env.VUE_APP_ON_ANDROID': JSON.stringify(env.VUE_APP_ON_ANDROID === 'android'),
    },
  };
});

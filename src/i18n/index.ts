import Vue from 'vue';
import VueI18n from 'vue-i18n';
import moment from 'moment';

import it from './locales/it';
import en from './locales/en';

Vue.use(VueI18n);

// Registrata a mano invece di `import 'moment/locale/it'` (usato prima) —
// bug reale trovato dall'utente: la topbar mostrava i giorni della
// settimana in inglese ("Wed 5 Aug") nonostante l'app fosse in italiano.
// Causa: nella build di produzione (Vite/Rollup), quell'import — un side
// effect puro su un modulo CommonJS/UMD, senza alcun binding usato — non
// arrivava mai ad eseguirsi: `moment.locales()` restava `["en"]` anche
// subito dopo questa stessa riga, in qualunque punto del bundle lo si
// controllasse (confermato con log diretti). Non un problema di chunk
// duplicati (già escluso forzando 'moment' in un chunk manuale dedicato,
// vedi vite.config.js — non bastava da solo): l'interop CommonJS di
// Rollup semplicemente non generava mai la chiamata che esegue il
// modulo. Chiamare `defineLocale` direttamente qui, da codice ESM
// nostro, elimina il problema alla radice — nessuna chiamata può essere
// "dimenticata" da un bundler quando è una riga di funzione scritta a
// mano, non il side effect di un import. Configurazione copiata
// verbatim da `node_modules/moment/locale/it.js` (moment stesso, MIT).
moment.defineLocale('it', {
  months: 'gennaio_febbraio_marzo_aprile_maggio_giugno_luglio_agosto_settembre_ottobre_novembre_dicembre'.split(
    '_'
  ),
  monthsShort: 'gen_feb_mar_apr_mag_giu_lug_ago_set_ott_nov_dic'.split('_'),
  weekdays: 'domenica_lunedì_martedì_mercoledì_giovedì_venerdì_sabato'.split('_'),
  weekdaysShort: 'dom_lun_mar_mer_gio_ven_sab'.split('_'),
  weekdaysMin: 'do_lu_ma_me_gi_ve_sa'.split('_'),
  longDateFormat: {
    LT: 'HH:mm',
    LTS: 'HH:mm:ss',
    L: 'DD/MM/YYYY',
    LL: 'D MMMM YYYY',
    LLL: 'D MMMM YYYY HH:mm',
    LLLL: 'dddd D MMMM YYYY HH:mm',
  },
  calendar: {
    sameDay: function (this: moment.Moment) {
      return '[Oggi a' + (this.hours() > 1 ? 'lle ' : this.hours() === 0 ? ' ' : "ll'") + ']LT';
    },
    nextDay: function (this: moment.Moment) {
      return '[Domani a' + (this.hours() > 1 ? 'lle ' : this.hours() === 0 ? ' ' : "ll'") + ']LT';
    },
    nextWeek: function (this: moment.Moment) {
      return 'dddd [a' + (this.hours() > 1 ? 'lle ' : this.hours() === 0 ? ' ' : "ll'") + ']LT';
    },
    lastDay: function (this: moment.Moment) {
      return '[Ieri a' + (this.hours() > 1 ? 'lle ' : this.hours() === 0 ? ' ' : "ll'") + ']LT';
    },
    lastWeek: function (this: moment.Moment) {
      switch (this.day()) {
        case 0:
          return (
            '[La scorsa] dddd [a' + (this.hours() > 1 ? 'lle ' : this.hours() === 0 ? ' ' : "ll'") + ']LT'
          );
        default:
          return (
            '[Lo scorso] dddd [a' + (this.hours() > 1 ? 'lle ' : this.hours() === 0 ? ' ' : "ll'") + ']LT'
          );
      }
    },
    sameElse: 'L',
  },
  relativeTime: {
    future: 'tra %s',
    past: '%s fa',
    s: 'alcuni secondi',
    ss: '%d secondi',
    m: 'un minuto',
    mm: '%d minuti',
    h: "un'ora",
    hh: '%d ore',
    d: 'un giorno',
    dd: '%d giorni',
    w: 'una settimana',
    ww: '%d settimane',
    M: 'un mese',
    MM: '%d mesi',
    y: 'un anno',
    yy: '%d anni',
  },
  dayOfMonthOrdinalParse: /\d{1,2}º/,
  ordinal: '%dº',
  week: {
    dow: 1, // Monday is the first day of the week.
    doy: 4, // The week that contains Jan 4th is the first week of the year.
  },
});

// Solo italiano + inglese — richiesta esplicita (2026-08-12): l'app è
// pensata prima di tutto per un utente italiano (il grosso
// dell'interfaccia, Home/Progetti/Sidebar/Topbar, è scritto in
// italiano fin dall'inizio), l'inglese resta come seconda lingua di
// riferimento. Le altre lingue che esistevano qui (UK/DE/RU/zh-CN)
// traducevano solo le pagine vecchie ereditate da ActivityWatch
// upstream e non venivano mai usate — rimosse.
export type AppLocale = 'it' | 'en';

const SUPPORTED: AppLocale[] = ['it', 'en'];

export function isAppLocale(value: string | null | undefined): value is AppLocale {
  return SUPPORTED.includes(value as AppLocale);
}

const HTML_LANG: Record<AppLocale, string> = {
  it: 'it',
  en: 'en',
};

function detectBrowserLocale(): AppLocale | null {
  if (typeof navigator === 'undefined') {
    return null;
  }
  const lang = (navigator.language || '').toLowerCase();
  if (lang.startsWith('it')) return 'it';
  if (lang.startsWith('en')) return 'en';
  return null;
}

export function getInitialLocale(): AppLocale {
  try {
    const saved = localStorage.getItem('locale');
    if (saved !== null && isAppLocale(saved)) {
      return saved;
    }
  } catch {
    /* ignore */
  }
  // Inglese di default se il sistema è in una lingua diversa da italiano
  // o inglese — richiesta esplicita: solo un sistema davvero in italiano
  // deve aprire l'app in italiano, qualunque altra lingua (non solo
  // l'inglese) apre in inglese.
  return detectBrowserLocale() ?? 'en';
}

const MOMENT_LOCALE: Record<AppLocale, string> = {
  it: 'it',
  en: 'en',
};

const initialLocale = getInitialLocale();

export const i18n = new VueI18n({
  locale: initialLocale,
  fallbackLocale: 'it',
  messages: { it, en },
  silentTranslationWarn: process.env.NODE_ENV === 'production',
});

moment.locale(MOMENT_LOCALE[initialLocale]);

export function setAppLocale(locale: string): void {
  const next = isAppLocale(locale) ? locale : 'it';
  i18n.locale = next;
  moment.locale(MOMENT_LOCALE[next]);
  document.documentElement.lang = HTML_LANG[next];
  try {
    localStorage.setItem('locale', next);
  } catch {
    /* ignore */
  }
}

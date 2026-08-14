// Motore di valutazione delle regole di notifica personalizzate
// (Impostazioni → Notifiche, vedi util/notifyRules.ts per la forma).
// Chiamato periodicamente da App.vue (ogni 60s, stessa idea del
// controllo sforamento progetti in projectTimerMixin.ts) — non dipende
// dallo stato "corrente" di activityStore (che potrebbe mostrare un
// giorno diverso da oggi se l'utente sta guardando la Timeline di ieri):
// interroga i bucket direttamente, sempre per "oggi".
//
// La regola di tipo "vpn" NON viene valutata qui — quella gira lato
// Rust (vpn_notify.rs), che legge lo stesso settingsStore.notifyRules
// tramite AppServer::get_setting per sapere se è abilitata. Qui viene
// solo ignorata.
import { invoke } from '@tauri-apps/api/core';
import moment from 'moment';
import { useSettingsStore } from '~/stores/settings';
import { useBucketsStore } from '~/stores/buckets';
import { useAppCategoriesStore } from '~/stores/appCategories';
import { useProjectsStore } from '~/stores/projects';
import { getHomeClient } from '~/util/awclient';
import { get_day_start_with_offset, get_today_with_offset } from '~/util/time';
import { NotifyRule } from '~/util/notifyRules';

// Dedup in memoria, si resetta ad ogni riavvio dell'app — stessa idea di
// progettiGiaNotificati in projectTimerMixin.ts e di gia_notificati in
// vpn_notify.rs: non rimandare lo stesso avviso ad ogni giro finché la
// condizione resta vera, ma nemmeno persistere nulla su disco (l'utente
// la rivede comunque se la condizione torna vera in una sessione futura).
const giaNotificate = new Set<string>();

// L'AFK watcher manda un ping ogni 5s ma un evento "not-afk" dura poco
// per volta (vedi HomeTimelineSection.vue) — un margine di 3 minuti
// dall'ultimo evento noto è più che sufficiente per considerare lo stato
// ancora valido senza rischiare falsi negativi per un giro di poll perso.
const AFK_FRESHNESS_MINUTES = 3;

async function invia(nome: string, corpo: string) {
  try {
    await invoke('invia_notifica_generica', { titolo: nome, corpo });
  } catch (e) {
    // Fuori da Tauri (dev server puro nel browser) — stesso pattern
    // già usato ovunque nell'app.
  }
}

function oggiRange() {
  const settingsStore = useSettingsStore();
  const start = moment(get_day_start_with_offset(get_today_with_offset(settingsStore.startOfDay), settingsStore.startOfDay));
  const end = start.clone().add(1, 'day');
  return { start, end };
}

async function minutiOggiPerApp(host: string): Promise<Map<string, number>> {
  const { start, end } = oggiRange();
  const mappa = new Map<string, number>();
  if (!host) return mappa;
  try {
    const eventi = await getHomeClient().getEvents(`aw-watcher-window_${host}`, {
      start: start.toDate(),
      end: end.toDate(),
      limit: -1,
    });
    for (const e of eventi as any[]) {
      const app = (e.data?.app || '').toLowerCase();
      if (!app) continue;
      mappa.set(app, (mappa.get(app) || 0) + (e.duration || 0));
    }
  } catch {
    // Bucket assente (host non ancora noto, watcher mai partito) — nessun dato, non un errore.
  }
  return mappa;
}

function minutiPerCategoria(minutiApp: Map<string, number>): Map<string, number> {
  const appCategoriesStore = useAppCategoriesStore();
  const mappa = new Map<string, number>();
  for (const [app, secondi] of minutiApp) {
    const categoria = appCategoriesStore.categoryForApp(app);
    if (!categoria) continue;
    mappa.set(categoria, (mappa.get(categoria) || 0) + secondi);
  }
  return mappa;
}

async function isAfkAdesso(host: string): Promise<boolean> {
  if (!host) return false;
  try {
    const eventi = await getHomeClient().getEvents(`aw-watcher-afk_${host}`, { limit: 1 });
    const ultimo = eventi[0] as any;
    if (!ultimo) return false;
    const fine = moment(ultimo.timestamp).add(ultimo.duration || 0, 'seconds');
    const fresco = moment().diff(fine, 'minutes') <= AFK_FRESHNESS_MINUTES;
    return fresco && ultimo.data?.status === 'afk';
  } catch {
    return false;
  }
}

async function progettoInCorso(): Promise<boolean> {
  try {
    const blocchi = await getHomeClient().getEvents('aw-stopwatch', { limit: -1 });
    return (blocchi as any[]).some(b => {
      const segmenti = b.data?.segments || [];
      const ultimo = segmenti[segmenti.length - 1];
      return !b.data?.sealed && ultimo && ultimo.end === null;
    });
  } catch {
    return false;
  }
}

async function minutiTotaliProgetto(projectId: string): Promise<number> {
  try {
    const blocchi = await getHomeClient().getEvents('aw-stopwatch', { limit: -1 });
    let secondi = 0;
    for (const b of blocchi as any[]) {
      if (b.data?.projectId !== projectId) continue;
      for (const s of b.data.segments || []) {
        const fine = s.end ? moment(s.end) : moment();
        secondi += fine.diff(moment(s.start), 'seconds');
      }
    }
    return secondi / 60;
  } catch {
    return 0;
  }
}

function soglaRaggiunta(rule: NotifyRule, minutiAttuali: number, minutiRiferimentoPct: number | null): boolean {
  if (rule.cond === 'minutes') {
    return minutiAttuali >= (rule.amount || 0);
  }
  if (rule.cond === 'pct') {
    const riferimento = minutiRiferimentoPct;
    if (!riferimento || riferimento <= 0) return false;
    const percentualeAttuale = (minutiAttuali / riferimento) * 100;
    return percentualeAttuale >= (rule.amount || 0);
  }
  return false;
}

function chiaveDedup(rule: NotifyRule, extra = ''): string {
  // La data di oggi entra nella chiave: la stessa regola può scattare di
  // nuovo il giorno dopo (i minuti ripartono da zero), non resta "già
  // notificata" per sempre dopo il primo giorno in cui è scattata.
  return `${rule.id}|${get_today_with_offset(useSettingsStore().startOfDay)}|${extra}`;
}

export async function valutaRegoleNotifica(): Promise<void> {
  const settingsStore = useSettingsStore();
  const regole = settingsStore.notifyRules.filter(r => r.enabled && r.type !== 'vpn');
  if (regole.length === 0) return;

  const host = useBucketsStore().host;
  const haCategoriaOApp = regole.some(r => r.type === 'category' || r.type === 'app');
  const haAfk = regole.some(r => r.type === 'afk' || r.type === 'afkProject');
  const haAfkProgetto = regole.some(r => r.type === 'afkProject');
  const haProgetto = regole.some(r => r.type === 'project');
  const haNonCategorizzato = regole.some(r => r.type === 'uncategorized');

  const [minutiApp, afkAdesso, progettoAttivo] = await Promise.all([
    haCategoriaOApp || haNonCategorizzato ? minutiOggiPerApp(host) : Promise.resolve(new Map<string, number>()),
    haAfk ? isAfkAdesso(host) : Promise.resolve(false),
    haAfkProgetto ? progettoInCorso() : Promise.resolve(false),
  ]);
  const minutiCategoria = haCategoriaOApp ? minutiPerCategoria(minutiApp) : new Map<string, number>();

  for (const rule of regole) {
    const nomeRegola = rule.name || rule.value || rule.type;

    if (rule.type === 'category' || rule.type === 'app') {
      if (!rule.value) continue;
      const minutiAttuali =
        (rule.type === 'category' ? minutiCategoria.get(rule.value) : minutiApp.get(rule.value.toLowerCase())) || 0;
      const minutiAttualiInMinuti = minutiAttuali / 60;
      const riferimentoPct = rule.sogliaMinuti;
      if (soglaRaggiunta(rule, minutiAttualiInMinuti, riferimentoPct)) {
        const chiave = chiaveDedup(rule);
        if (!giaNotificate.has(chiave)) {
          giaNotificate.add(chiave);
          await invia(nomeRegola, `${rule.value}: ${Math.round(minutiAttualiInMinuti)} min oggi.`);
        }
      }
      continue;
    }

    if (rule.type === 'project') {
      if (!rule.value) continue;
      if (!haProgetto) continue;
      const progetto = useProjectsStore().projects.find(p => p.id === rule.value);
      if (!progetto || progetto.closed) continue;
      const minutiAttuali = await minutiTotaliProgetto(rule.value);
      const riferimentoPct = progetto.budgetHours ? progetto.budgetHours * 60 : null;
      if (soglaRaggiunta(rule, minutiAttuali, riferimentoPct)) {
        const chiave = chiaveDedup(rule);
        if (!giaNotificate.has(chiave)) {
          giaNotificate.add(chiave);
          await invia(nomeRegola, `${progetto.name}: ${Math.round(minutiAttuali)} min totali.`);
        }
      }
      continue;
    }

    if (rule.type === 'afk' || rule.type === 'afkProject') {
      if (rule.type === 'afkProject' && !progettoAttivo) continue;
      if (!afkAdesso) continue;
      if (rule.cond === 'minutes') {
        // Serve sapere DA QUANTO tempo dura l'inattività, non solo che è
        // in corso — l'ultimo evento AFK letto da isAfkAdesso() copre
        // già questo: se è ancora fresco ma la sua durata supera la
        // soglia, l'inattività dura almeno quanto la soglia.
        const eventi = await getHomeClient().getEvents(`aw-watcher-afk_${host}`, { limit: 1 });
        const ultimo = eventi[0] as any;
        const minutiInattivo = ultimo ? (ultimo.duration || 0) / 60 : 0;
        if (minutiInattivo < (rule.amount || 0)) continue;
      }
      const chiave = chiaveDedup(rule);
      if (!giaNotificate.has(chiave)) {
        giaNotificate.add(chiave);
        await invia(
          nomeRegola,
          rule.type === 'afkProject' ? 'Sei inattivo con un progetto ancora in corso.' : 'Sei inattivo.'
        );
      }
      continue;
    }

    if (rule.type === 'uncategorized') {
      const appCategoriesStore = useAppCategoriesStore();
      for (const app of minutiApp.keys()) {
        if (appCategoriesStore.categoryForApp(app)) continue;
        const chiave = chiaveDedup(rule, app);
        if (!giaNotificate.has(chiave)) {
          giaNotificate.add(chiave);
          await invia(nomeRegola, `«${app}» non ha ancora una categoria.`);
        }
      }
    }
  }
}

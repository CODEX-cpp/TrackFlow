// Tipi e testo descrittivo condivisi per le regole di notifica
// personalizzate (Impostazioni → Notifiche) — la UI di editing
// (NotificationRulesSettings.vue) e il motore di valutazione periodico
// (notifyRulesEngine.ts) leggono/scrivono la stessa identica forma,
// salvata in settingsStore.notifyRules.
export type NotifyRuleType =
  | 'category'
  | 'app'
  | 'project'
  | 'vpn'
  | 'afk'
  | 'afkProject'
  | 'uncategorized';

export type NotifyRuleCond = 'minutes' | 'pct' | 'event';

export interface NotifyRule {
  id: string;
  name: string;
  enabled: boolean;
  type: NotifyRuleType;
  // Categoria/app/id progetto scelto — null per vpn/afk/afkProject/
  // uncategorized (non hanno un "valore" da scegliere, sono globali).
  value: string | null;
  // null per vpn/uncategorized (eventi puri, nessuna condizione da
  // configurare).
  cond: NotifyRuleCond | null;
  // Minuti soglia (cond = 'minutes'), oppure percentuale 1-100
  // (cond = 'pct') — null per cond = 'event' o quando cond è null.
  amount: number | null;
  // SOLO per category/app con cond = 'pct': quanti minuti valgono il
  // 100% (non esiste un budget implicito per categorie/app come invece
  // per i progetti, che usano project.budgetHours). Ignorato per tutti
  // gli altri casi.
  sogliaMinuti: number | null;
}

// Tipi che non hanno un dropdown "Valore" — sono eventi/condizioni
// globali, non riferiti a una categoria/app/progetto specifico.
export const TYPES_WITHOUT_VALUE: NotifyRuleType[] = [
  'vpn',
  'afk',
  'afkProject',
  'uncategorized',
];

// Tipi per cui l'utente può scegliere tra "supera N minuti" e
// "raggiunge soglia %".
export const TYPES_WITH_MINUTES_OR_PCT: NotifyRuleType[] = ['category', 'app', 'project'];

// Tipi per cui l'utente può scegliere tra "appena rilevata" (evento) e
// "supera N minuti".
export const TYPES_WITH_EVENT_OR_MINUTES: NotifyRuleType[] = ['afk', 'afkProject'];

// Tipi puramente ad evento, nessuna condizione configurabile.
export const TYPES_EVENT_ONLY: NotifyRuleType[] = ['vpn', 'uncategorized'];

export function newRuleId(): string {
  return `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
}

export function defaultRule(): NotifyRule {
  return {
    id: newRuleId(),
    name: '',
    enabled: true,
    type: 'category',
    value: null,
    cond: 'minutes',
    amount: 120,
    sogliaMinuti: null,
  };
}

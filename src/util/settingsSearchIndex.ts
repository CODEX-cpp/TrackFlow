// Elenco a mano di ogni riga/impostazione cercabile dalla ricerca in
// Topbar quando si è sulla pagina Impostazioni — titolo + testo di
// aiuto, con la chiave i18n del gruppo a cui appartiene (per lo scroll
// alla sezione giusta, vedi Settings.vue::scrollToSection). Curato a
// mano invece che introspezionando i componenti/le chiavi i18n a
// runtime: più semplice, verificabile a colpo d'occhio, e coerente con
// altri elenchi curati già nel progetto (es. APP_INFO in
// util/appNames.ts) — va solo tenuto aggiornato quando si aggiunge una
// nuova riga di impostazioni altrove.
export interface SettingsSearchEntryDef {
  groupId: string;
  titleKey: string;
  helpKey?: string;
}

export const SETTINGS_SEARCH_INDEX: SettingsSearchEntryDef[] = [
  // Generale
  { groupId: 'general', titleKey: 'settings.language.title', helpKey: 'settings.language.help' },
  {
    groupId: 'general',
    titleKey: 'settings.daystart.startOfDay',
    helpKey: 'settings.daystart.startOfDayHelp',
  },
  {
    groupId: 'general',
    titleKey: 'settings.daystart.startOfWeek',
    helpKey: 'settings.daystart.startOfWeekHelp',
  },
  // Home
  { groupId: 'home', titleKey: 'settings.lunchBreak.title', helpKey: 'settings.lunchBreak.help' },
  {
    groupId: 'home',
    titleKey: 'settings.screenshot.intervalTitle',
    helpKey: 'settings.screenshot.intervalHelp',
  },
  {
    groupId: 'home',
    titleKey: 'settings.screenshot.retentionTitle',
    helpKey: 'settings.screenshot.retentionHelp',
  },
  // Aspetto
  { groupId: 'appearance', titleKey: 'settings.theme.title', helpKey: 'settings.theme.help' },
  // Categorizzazione
  {
    groupId: 'categorization',
    titleKey: 'settings.categorization.title',
    helpKey: 'settings.categorization.help',
  },
  { groupId: 'categorization', titleKey: 'settings.categorization.appsTitle' },
  { groupId: 'categorization', titleKey: 'settings.activePattern.title' },
  // Notifiche
  { groupId: 'notifications', titleKey: 'settings.notifications.title', helpKey: 'settings.notifications.help' },
  // Integrazioni
  {
    groupId: 'integrations',
    titleKey: 'settings.vpnMapping.title',
    helpKey: 'settings.vpnMapping.help',
  },
  {
    groupId: 'integrations',
    titleKey: 'settings.voispeed.title',
    helpKey: 'settings.voispeed.help',
  },
  { groupId: 'integrations', titleKey: 'settings.aiAgent.title', helpKey: 'settings.aiAgent.help' },
  // Privacy
  { groupId: 'privacy', titleKey: 'settings.privacyFilters.title' },
  // Sviluppatore
  {
    groupId: 'developer',
    titleKey: 'settings.developer.masterToggle',
    helpKey: 'settings.developer.masterToggleHelp',
  },
  {
    groupId: 'developer',
    titleKey: 'settings.developer.devtoolsTitle',
    helpKey: 'settings.developer.devtoolsHelp',
  },
  {
    groupId: 'developer',
    titleKey: 'settings.developer.watcherStatus.title',
    helpKey: 'settings.developer.watcherStatus.help',
  },
  {
    groupId: 'developer',
    titleKey: 'settings.developer.logPanel.title',
    helpKey: 'settings.developer.logPanel.help',
  },
  {
    groupId: 'developer',
    titleKey: 'settings.developer.aqlConsole.title',
    helpKey: 'settings.developer.aqlConsole.help',
  },
  {
    groupId: 'developer',
    titleKey: 'settings.developer.folderShortcuts.title',
    helpKey: 'settings.developer.folderShortcuts.help',
  },
  // Nomi dei gruppi stessi — cercare "Aspetto" deve trovare il gruppo
  // anche se nessuna riga al suo interno contiene letteralmente quella
  // parola.
  { groupId: 'general', titleKey: 'settings.groups.general' },
  { groupId: 'home', titleKey: 'settings.groups.home' },
  { groupId: 'appearance', titleKey: 'settings.groups.appearance' },
  { groupId: 'categorization', titleKey: 'settings.groups.categorization' },
  { groupId: 'notifications', titleKey: 'settings.notifications.title' },
  { groupId: 'integrations', titleKey: 'settings.groups.integrations' },
  { groupId: 'privacy', titleKey: 'settings.groups.privacy' },
  { groupId: 'developer', titleKey: 'settings.groups.developer' },
];

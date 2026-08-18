<template lang="pug">
div
  div.settings-row
    div
      div.settings-row-title {{ $t('settings.notifications.title') }}
      div.settings-row-help {{ $t('settings.notifications.help') }}

  div.nr-list(v-if="rules.length > 0")
    div.nr-row(v-for="rule in rules" :key="rule.id")
      span.nr-row-name {{ rule.name || describeRule(rule) }}
      span.nr-row-desc(v-if="rule.name") {{ describeRule(rule) }}
      div.settings-toggle(:class="{ 'settings-toggle-on': rule.enabled }" @click="toggleRule(rule)")
        div.settings-toggle-thumb
      div.pill-btn-ghost.nr-remove-btn(@click="removeRule(rule)" :title="$t('settings.notifications.remove')")
        icon(name="trash")
  div.settings-row-help(v-else) {{ $t('settings.notifications.empty') }}

  div.nr-builder
    div.settings-row-title.nr-builder-title {{ $t('settings.notifications.newRule') }}

    div.nr-field
      label.settings-field-label {{ $t('settings.notifications.fieldName') }}
      input.settings-field(v-model="draft.name" :placeholder="$t('settings.notifications.fieldNamePlaceholder')")

    div.nr-field
      label.settings-field-label {{ $t('settings.notifications.fieldType') }}
      select.settings-field(v-model="draft.type" @change="onTypeChange")
        option(value="category") {{ $t('settings.notifications.typeCategory') }}
        option(value="app") {{ $t('settings.notifications.typeApp') }}
        option(value="project") {{ $t('settings.notifications.typeProject') }}
        option(value="vpn") {{ $t('settings.notifications.typeVpn') }}
        option(value="afk") {{ $t('settings.notifications.typeAfk') }}
        option(value="afkProject") {{ $t('settings.notifications.typeAfkProject') }}
        option(value="uncategorized") {{ $t('settings.notifications.typeUncategorized') }}

    div.nr-field(v-if="draft.type === 'category'")
      label.settings-field-label {{ $t('settings.notifications.fieldValue') }}
      select.settings-field(v-model="draft.value")
        option(v-if="categoryNames.length === 0" value="" disabled) {{ $t('settings.notifications.noCategories') }}
        option(v-for="c in categoryNames" :key="c" :value="c") {{ c }}

    div.nr-field(v-if="draft.type === 'app'")
      label.settings-field-label {{ $t('settings.notifications.fieldValue') }}
      select.settings-field(v-model="draft.value")
        option(v-if="appKeys.length === 0" value="" disabled) {{ $t('settings.notifications.noApps') }}
        option(v-for="a in appKeys" :key="a" :value="a") {{ appDisplayName(a) }}

    div.nr-field(v-if="draft.type === 'project'")
      label.settings-field-label {{ $t('settings.notifications.fieldValue') }}
      select.settings-field(v-model="draft.value")
        option(v-if="activeProjects.length === 0" value="" disabled) {{ $t('settings.notifications.noProjects') }}
        option(v-for="p in activeProjects" :key="p.id" :value="p.id") {{ p.name }}

    div.nr-field(v-if="hasCondition")
      label.settings-field-label {{ $t('settings.notifications.fieldCondition') }}
      div.nr-cond-row
        select.settings-field.nr-cond-select(v-model="draft.cond" @change="onCondChange")
          option(v-for="c in condOptions" :key="c.value" :value="c.value") {{ c.label }}
        template(v-if="draft.cond !== 'event'")
          input.settings-field.nr-cond-amount(type="number" min="1" v-model.number="draft.amount")
          span.nr-cond-unit {{ draft.cond === 'pct' ? '%' : $t('settings.notifications.minutesUnit') }}

    div.nr-field(v-if="draft.cond === 'pct' && (draft.type === 'category' || draft.type === 'app')")
      label.settings-field-label {{ $t('settings.notifications.fieldThresholdMinutes') }}
      input.settings-field(type="number" min="1" v-model.number="draft.sogliaMinuti")
      div.settings-row-help {{ $t('settings.notifications.thresholdMinutesHelp') }}

    div.nr-field(v-if="draft.cond === 'pct' && draft.type === 'project'")
      div.settings-row-help {{ $t('settings.notifications.projectPctHelp') }}

    div.nr-preview
      icon(name="bell")
      span {{ previewText }}

    div.settings-alert.settings-alert-danger(v-if="error") {{ error }}

    div.pill-btn(@click="addRule") {{ $t('settings.notifications.addRule') }}
</template>

<script lang="ts">
import 'vue-awesome/icons/trash';
import 'vue-awesome/icons/bell';
import { useSettingsStore } from '~/stores/settings';
import { useAppCategoriesStore } from '~/stores/appCategories';
import { useProjectsStore } from '~/stores/projects';
import {
  NotifyRule,
  NotifyRuleType,
  TYPES_WITHOUT_VALUE,
  TYPES_WITH_MINUTES_OR_PCT,
  TYPES_WITH_EVENT_OR_MINUTES,
  defaultRule,
} from '~/util/notifyRules';
import { invoke } from '@tauri-apps/api/core';
import { displayNameForApp, refreshDynamicAppData } from '~/util/appNames';

interface AppConosciuta {
  app: string;
  nome_leggibile: string | null;
}

export default {
  name: 'NotificationRulesSettings',
  data(this: any) {
    return {
      settingsStore: useSettingsStore(),
      appCategoriesStore: useAppCategoriesStore(),
      projectsStore: useProjectsStore(),
      draft: defaultRule() as NotifyRule,
      error: '',
      appKeys: [] as string[],
    };
  },
  async mounted(this: any) {
    // Entrambe queste liste potrebbero non essere ancora state caricate
    // se si arriva direttamente su Impostazioni senza prima passare da
    // Home/Progetti in questa sessione — projectsStore.load() è
    // idempotente (vedi projectTimerMixin.ts), refreshDynamicAppData()
    // aggiorna nomi/colori (solo presentazione, vedi appDisplayName) con
    // quanto scoperto dal watcher da quando è stata compilata questa
    // build in poi.
    await Promise.all([this.projectsStore.load(), refreshDynamicAppData()]);
    // Elenco app selezionabili: le app DAVVERO apparse in Timeline
    // (stesso comando Tauri usato da CategorizationSettings.vue) — non
    // più knownAppKeys() (icone/nomi scoperti dal watcher icone, fonte
    // sbagliata: include anche processi di sistema mai mostrati come
    // finestra attiva, es. svchost.exe, conhost.exe — bug reale
    // segnalato dall'utente). knownAppKeys() resta usata sotto solo per
    // le funzioni di presentazione (nome/icona), non per decidere quali
    // app offrire nel menu.
    try {
      const appConosciute = await invoke<AppConosciuta[]>('elenca_app_conosciute');
      this.appKeys = appConosciute.map((a: AppConosciuta) => a.app).sort();
    } catch (e) {
      // L'app potrebbe girare fuori da Tauri durante lo sviluppo web puro
      // (npx vite senza il guscio nativo) — invoke() non esiste in quel
      // caso, non è un errore da mostrare all'utente. Stesso pattern già
      // usato in AiAgentSettings.vue/CategorizationSettings.vue.
    }
  },
  computed: {
    rules(this: any): NotifyRule[] {
      return this.settingsStore.notifyRules;
    },
    categoryNames(this: any): string[] {
      return this.appCategoriesStore.categories.map((c: any) => c.name);
    },
    activeProjects(this: any) {
      return this.projectsStore.activeProjects;
    },
    hasCondition(this: any): boolean {
      return TYPES_WITH_MINUTES_OR_PCT.includes(this.draft.type) || TYPES_WITH_EVENT_OR_MINUTES.includes(this.draft.type);
    },
    condOptions(this: any) {
      if (TYPES_WITH_MINUTES_OR_PCT.includes(this.draft.type as NotifyRuleType)) {
        return [
          { value: 'minutes', label: this.$t('settings.notifications.condOver') },
          { value: 'pct', label: this.$t('settings.notifications.condThreshold') },
        ];
      }
      if (TYPES_WITH_EVENT_OR_MINUTES.includes(this.draft.type as NotifyRuleType)) {
        return [
          { value: 'event', label: this.$t('settings.notifications.condEvent') },
          { value: 'minutes', label: this.$t('settings.notifications.condOver') },
        ];
      }
      return [];
    },
    previewText(this: any): string {
      return this.describeRule(this.draft);
    },
  },
  methods: {
    describeRule(rule: NotifyRule): string {
      const t = (key: string, params?: any) => this.$t(key, params) as string;
      if (rule.type === 'vpn') return t('settings.notifications.descVpn');
      if (rule.type === 'uncategorized') return t('settings.notifications.descUncategorized');
      if (rule.type === 'afk' || rule.type === 'afkProject') {
        const base = rule.cond === 'event' ? t('settings.notifications.condEvent') : `${t('settings.notifications.condOver')} ${rule.amount || 0} ${t('settings.notifications.minutesUnit')}`;
        return rule.type === 'afkProject'
          ? t('settings.notifications.descAfkProject', { cond: base })
          : t('settings.notifications.descAfk', { cond: base });
      }
      const valueLabel = rule.value || '…';
      const condStr =
        rule.cond === 'pct'
          ? `${t('settings.notifications.condThreshold')} ${rule.amount || 0}%`
          : `${t('settings.notifications.condOver')} ${rule.amount || 0} ${t('settings.notifications.minutesUnitToday')}`;
      const typeLabel =
        rule.type === 'category'
          ? t('settings.notifications.typeCategory')
          : rule.type === 'app'
          ? t('settings.notifications.typeApp')
          : t('settings.notifications.typeProject');
      const displayValue =
        rule.type === 'project'
          ? this.projectsStore.projects.find((p: any) => p.id === rule.value)?.name || valueLabel
          : rule.type === 'app'
          ? this.appDisplayName(rule.value || '')
          : valueLabel;
      return `${typeLabel} «${displayValue}» ${condStr}`;
    },
    appDisplayName(key: string): string {
      return displayNameForApp(key) || key;
    },
    onTypeChange(this: any) {
      this.draft.value = null;
      if (TYPES_WITH_MINUTES_OR_PCT.includes(this.draft.type)) {
        this.draft.cond = 'minutes';
        this.draft.amount = 120;
      } else if (TYPES_WITH_EVENT_OR_MINUTES.includes(this.draft.type)) {
        this.draft.cond = 'event';
        this.draft.amount = null;
      } else {
        this.draft.cond = null;
        this.draft.amount = null;
      }
      this.draft.sogliaMinuti = null;
    },
    onCondChange(this: any) {
      if (this.draft.cond === 'event') {
        this.draft.amount = null;
      } else if (this.draft.cond === 'pct') {
        this.draft.amount = 80;
      } else {
        this.draft.amount = 120;
      }
    },
    toggleRule(this: any, rule: NotifyRule) {
      const updated = this.rules.map((r: NotifyRule) => (r.id === rule.id ? { ...r, enabled: !r.enabled } : r));
      this.settingsStore.update({ notifyRules: updated });
    },
    removeRule(this: any, rule: NotifyRule) {
      const updated = this.rules.filter((r: NotifyRule) => r.id !== rule.id);
      this.settingsStore.update({ notifyRules: updated });
    },
    async addRule(this: any) {
      this.error = '';
      const needsValue = !TYPES_WITHOUT_VALUE.includes(this.draft.type);
      if (needsValue && !this.draft.value) {
        this.error = this.$t('settings.notifications.valueRequiredError') as string;
        return;
      }
      if (this.draft.cond === 'pct' && (this.draft.type === 'category' || this.draft.type === 'app') && !this.draft.sogliaMinuti) {
        this.error = this.$t('settings.notifications.thresholdRequiredError') as string;
        return;
      }
      const nuova: NotifyRule = { ...this.draft, id: `${Date.now()}-${Math.random().toString(36).slice(2, 8)}` };
      await this.settingsStore.update({ notifyRules: [...this.rules, nuova] });
      this.draft = defaultRule();
    },
  },
};
</script>

<style scoped>
.nr-list {
  margin-top: 12px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  overflow: hidden;
}

.nr-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border-bottom: 1px solid var(--color-border);
  background-color: var(--color-surface);
  font-size: var(--font-size-sm);
}

.nr-row:last-child {
  border-bottom: none;
}

.nr-row-name {
  color: var(--color-text);
  font-weight: var(--font-weight-semibold);
}

.nr-row-desc {
  flex: 1;
  color: var(--color-text-faint);
  font-size: var(--font-size-xs);
}

.nr-remove-btn {
  flex-shrink: 0;
  padding: 6px 10px;
}

.nr-builder {
  margin-top: 20px;
  border-top: 1px solid var(--color-border);
  padding-top: 16px;
}

.nr-builder-title {
  margin-bottom: 12px;
}

.nr-field {
  margin-bottom: 10px;
}

.nr-cond-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.nr-cond-select {
  flex-shrink: 0;
  width: auto;
}

.nr-cond-amount {
  width: 90px;
}

.nr-cond-unit {
  font-size: var(--font-size-sm);
  color: var(--color-text-faint);
}

.nr-preview {
  display: flex;
  align-items: center;
  gap: 8px;
  background-color: var(--color-surface2);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  padding: 10px 12px;
  font-size: var(--font-size-sm);
  color: var(--color-text-dim);
  margin: 12px 0;
}

.settings-alert {
  margin-bottom: 12px;
  padding: 10px 14px;
  border-radius: var(--radius-md);
  border: 1px solid var(--color-border);
  font-size: var(--font-size-sm);
}

.settings-alert-danger {
  background-color: rgba(217, 83, 79, 0.15);
  color: var(--color-text);
}
</style>

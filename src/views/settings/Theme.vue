<template lang="pug">
div.settings-row
  div
    div.settings-row-title {{ $t('settings.theme.title') }}
    div.settings-row-help {{ $t('settings.theme.help') }}
  div.settings-tabs(v-if="_loaded")
    div.settings-tab(
      v-for="opt in themeOptions"
      :key="opt.value"
      :class="{ 'settings-tab-active': theme === opt.value }"
      @click="theme = opt.value"
    )
      icon(:name="opt.icon" style="width: 12px; height: 12px;")
      | {{ $t(opt.labelKey) }}
  span(v-else)
    | {{ $t('common.loading') }}
</template>

<script lang="ts">
import 'vue-awesome/icons/desktop';
import 'vue-awesome/icons/sun';
import 'vue-awesome/icons/moon';
import { mapState } from 'pinia';
import { useSettingsStore } from '~/stores/settings';
import { detectPreferredTheme } from '~/util/theme';

export default {
  name: 'Theme',
  data() {
    return {
      themeOptions: [
        { value: 'auto', labelKey: 'settings.theme.auto', icon: 'desktop' },
        { value: 'light', labelKey: 'settings.theme.light', icon: 'sun' },
        { value: 'dark', labelKey: 'settings.theme.dark', icon: 'moon' },
      ],
    };
  },
  computed: {
    ...mapState(useSettingsStore, ['_loaded']),
    theme: {
      get() {
        const settingsStore = useSettingsStore();
        return settingsStore.theme;
      },
      set(value) {
        const settingsStore = useSettingsStore();
        settingsStore.update({
          theme: value,
        });

        const detectedTheme = value === 'auto' ? detectPreferredTheme() : value;
        document.body.classList.toggle('theme-dark', detectedTheme === 'dark');
        document.body.classList.toggle('theme-light', detectedTheme !== 'dark');
      },
    },
  },
};
</script>

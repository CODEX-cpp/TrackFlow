<template lang="pug">
div.settings-row
  div
    div.settings-row-title {{ $t('settings.language.title') }}
    div.settings-row-help {{ $t('settings.language.help') }}
  select.settings-field(v-if="_loaded" :value="locale" @change="locale = $event.target.value")
    option(value="it") {{ $t('common.languageIt') }}
    option(value="en") {{ $t('common.languageEn') }}
  span(v-else)
    | {{ $t('common.loading') }}
</template>

<script lang="ts">
import { mapState } from 'pinia';
import { useSettingsStore } from '~/stores/settings';
import { isAppLocale, setAppLocale } from '~/i18n';

export default {
  name: 'LanguageSettings',
  computed: {
    ...mapState(useSettingsStore, ['_loaded']),
    locale: {
      get() {
        const { locale } = useSettingsStore();
        return isAppLocale(locale) ? locale : 'it';
      },
      set(value: string) {
        if (!isAppLocale(value)) {
          return;
        }
        useSettingsStore().update({ locale: value });
        setAppLocale(value);
      },
    },
  },
};
</script>

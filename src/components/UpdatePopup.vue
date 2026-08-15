
<template lang="pug">
div.update-popup(v-if="visible" :class="{ 'update-popup-clickable': clickable }" @click="onClick")
  img.update-popup-logo(:src="logoUrl" alt="TrackFlow")
  div.update-popup-text
    div.update-popup-title {{ title }}
    div.update-popup-version {{ $t('updatePopup.versionPrefix') }} {{ updaterStore.version }}
  div.update-popup-action
    div.update-popup-spinner(v-if="updaterStore.status === 'downloading'")
    icon(v-else name="arrow-right")
</template>

<script lang="ts">
import 'vue-awesome/icons/arrow-right';
import { useUpdaterStore } from '~/stores/updater';
import logoUrl from '~/assets/logo.png';

// Tre stati possibili (vedi stores/updater.ts) — il popup resta
// nascosto del tutto in idle, così compare solo mentre sta scaricando
// o quando c'è davvero qualcosa da fare, non come elemento fisso della
// sidebar.
export default {
  name: 'UpdatePopup',
  data() {
    return {
      updaterStore: useUpdaterStore(),
      logoUrl,
    };
  },
  computed: {
    visible(this: any): boolean {
      // 'error' resta silenzioso (loggato in console dallo store) —
      // niente popup di errore previsto per ora, un fallimento di
      // download/installazione non deve bloccare l'uso dell'app.
      return this.updaterStore.status === 'available'
        || this.updaterStore.status === 'downloading'
        || this.updaterStore.status === 'ready';
    },
    clickable(this: any): boolean {
      return this.updaterStore.status === 'available' || this.updaterStore.status === 'ready';
    },
    title(this: any): string {
      switch (this.updaterStore.status) {
        case 'downloading':
          return this.$t('updatePopup.downloading') as string;
        case 'ready':
          return this.$t('updatePopup.restartToUpdate') as string;
        case 'available':
          return this.$t('updatePopup.available') as string;
        default:
          return '';
      }
    },
  },
  methods: {
    onClick(this: any) {
      if (this.updaterStore.status === 'available') {
        this.updaterStore.startDownload();
      } else if (this.updaterStore.status === 'ready') {
        this.updaterStore.installaERiavvia();
      }
    },
  },
};
</script>

<style lang="scss" scoped>
@import '../style/theme.css';

.update-popup {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px;
  margin-bottom: 8px;
  border-radius: var(--radius-md);
  background-color: var(--color-surface2);
}

.update-popup-clickable {
  cursor: pointer;

  &:hover {
    filter: brightness(1.08);
  }
}

.update-popup-logo {
  width: 20px;
  height: 20px;
  flex-shrink: 0;
  object-fit: contain;
}

.update-popup-text {
  flex: 1;
  min-width: 0;
}

.update-popup-title {
  font-size: var(--font-size-xs);
  line-height: 1.25;
  color: var(--color-text);
  font-weight: var(--font-weight-semibold);
  // Testi come "Update disponibile, clicca per aggiornare" non ci
  // stanno su una riga negli 220px della sidebar nemmeno al font più
  // piccolo disponibile — vanno a capo invece di essere troncati con
  // ellissi (segnalato dall'utente: il testo veniva tagliato).
  white-space: normal;
  word-break: break-word;
}

.update-popup-version {
  font-size: 10px;
  color: var(--color-text-faint);
  margin-top: 2px;
}

.update-popup-action {
  flex-shrink: 0;
  width: 16px;
  height: 16px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--color-text-dim);
}

.update-popup-spinner {
  width: 14px;
  height: 14px;
  border-radius: 50%;
  border: 2px solid var(--color-border);
  border-top-color: var(--color-accent1);
  animation: update-popup-spin 0.8s linear infinite;
}

@keyframes update-popup-spin {
  to {
    transform: rotate(360deg);
  }
}
</style>

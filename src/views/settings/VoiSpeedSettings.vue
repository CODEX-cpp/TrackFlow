<template lang="pug">
div
  div.settings-row
    div
      div.settings-row-title {{ $t('settings.voispeed.title') }}
      div.settings-row-help {{ $t('settings.voispeed.help') }}
    div.pill-btn(
      v-if="!status.connected"
      @click="!status.connecting && connect()"
      :class="{ 'pill-btn-disabled': status.connecting }"
    )
      | {{ status.connecting ? $t('settings.voispeed.connecting') : $t('settings.voispeed.connect') }}
    div.voispeed-connected-actions(v-else)
      button.voispeed-refresh-btn(
        type="button"
        @click="!refreshing && forceCheck()"
        :disabled="refreshing"
        :title="$t('settings.voispeed.refreshTitle')"
      )
        icon(name="sync" :class="{ 'voispeed-refresh-spin': refreshing }")
        | {{ refreshing ? $t('settings.voispeed.refreshing') : $t('settings.voispeed.refresh') }}
      div.pill-btn-ghost(@click="disconnect()")
        | {{ $t('settings.voispeed.disconnect') }}

  div.settings-row-help.voispeed-hint(v-if="!status.connected && !status.connecting")
    | {{ $t('settings.voispeed.loginHint') }}

  div.settings-alert.settings-alert-danger(v-if="status.last_error") {{ status.last_error }}

  div.voispeed-status
    span.voispeed-status-dot(:class="{ 'voispeed-status-dot-on': status.connected }")
    span {{ status.connected ? $t('settings.voispeed.statusConnected') : $t('settings.voispeed.statusNotConnected') }}
    span.voispeed-status-sync(v-if="status.connected")
      | — {{ $t('settings.voispeed.lastSync') }}: {{ lastSyncDisplay }}
</template>

<script lang="ts">
import { invoke } from '@tauri-apps/api/core';
import 'vue-awesome/icons/sync';

interface VoiSpeedStatus {
  connected: boolean;
  connecting: boolean;
  last_error: string | null;
  last_sync: string | null;
}

function emptyStatus(): VoiSpeedStatus {
  return { connected: false, connecting: false, last_error: null, last_sync: null };
}

export default {
  name: 'VoiSpeedSettings',
  data() {
    return {
      status: emptyStatus() as VoiSpeedStatus,
      pollHandle: null as ReturnType<typeof setInterval> | null,
      refreshing: false,
    };
  },
  computed: {
    lastSyncDisplay(): string {
      if (!this.status.last_sync) return this.$t('settings.voispeed.never') as string;
      return new Date(this.status.last_sync).toLocaleTimeString(this.$i18n.locale, {
        hour: '2-digit',
        minute: '2-digit',
      });
    },
  },
  async mounted() {
    await this.refreshStatus();
    // Il collegamento/la sincronizzazione avvengono lato Rust, fuori dal
    // controllo diretto di questo componente — un poll leggero tiene lo
    // stato mostrato aggiornato (es. dopo il login nella finestra VoiSpeed,
    // o quando arriva un nuovo giro di sincronizzazione in background).
    this.pollHandle = setInterval(() => this.refreshStatus(), 5000);
  },
  beforeDestroy() {
    if (this.pollHandle) clearInterval(this.pollHandle);
  },
  methods: {
    async refreshStatus() {
      try {
        this.status = await invoke<VoiSpeedStatus>('voispeed_status');
      } catch (e) {
        // L'app potrebbe girare fuori da Tauri durante lo sviluppo web
        // puro (npx vite senza il guscio nativo) — in quel caso invoke()
        // non esiste, non è un errore da mostrare all'utente.
      }
    },
    async connect() {
      this.status.connecting = true;
      try {
        await invoke('voispeed_connect');
      } catch (e: any) {
        this.status.last_error = e?.toString?.() ?? String(e);
      } finally {
        await this.refreshStatus();
      }
    },
    async disconnect() {
      try {
        await invoke('voispeed_disconnect');
      } finally {
        await this.refreshStatus();
      }
    },
    // Forza subito il giro di verifica (rinnovo token + controllo login)
    // invece di aspettare il prossimo giro automatico — richiesta
    // esplicita dell'utente. L'errore, se presente, arriva già nella
    // risposta del comando stesso (non serve aspettare il prossimo poll
    // di refreshStatus per vederlo).
    async forceCheck() {
      this.refreshing = true;
      try {
        await invoke('voispeed_refresh_check');
      } catch (e: any) {
        this.status.last_error = e?.toString?.() ?? String(e);
      } finally {
        this.refreshing = false;
        await this.refreshStatus();
      }
    },
  },
};
</script>

<style scoped>
.voispeed-hint {
  margin-top: 4px;
}

.voispeed-connected-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.voispeed-refresh-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  border-radius: var(--radius-md);
  border: 1px solid var(--color-border);
  background-color: transparent;
  color: var(--color-text-dim);
  font-size: var(--font-size-sm);
  font-family: inherit;
  cursor: pointer;
}

.voispeed-refresh-btn:hover:not(:disabled) {
  color: var(--color-text);
  border-color: var(--color-text-dim);
}

.voispeed-refresh-btn:disabled {
  cursor: default;
  opacity: 0.7;
}

.voispeed-refresh-spin {
  animation: voispeed-spin 1s linear infinite;
}

@keyframes voispeed-spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}

.voispeed-status {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 12px;
  font-size: var(--font-size-sm);
  color: var(--color-text-dim);
}

.voispeed-status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background-color: var(--color-text-faint);
  flex-shrink: 0;
}

.voispeed-status-dot-on {
  background-color: #7c9a5c;
}

.voispeed-status-sync {
  color: var(--color-text-faint);
}
</style>

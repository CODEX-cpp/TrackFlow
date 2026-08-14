<template lang="pug">
div
  div.settings-row
    div
      div.settings-row-title {{ $t('settings.vpnMapping.title') }}
      div.settings-row-help {{ $t('settings.vpnMapping.help') }}
    div.settings-actions
      div.pill-btn-ghost(@click="!salvando && scarta()" :class="{ 'pill-btn-disabled': !modificato || salvando }") {{ $t('settings.vpnMapping.discard') }}
      div.pill-btn(@click="!salvando && salva()" :class="{ 'pill-btn-disabled': !modificato || salvando }")
        | {{ salvando ? $t('settings.vpnMapping.saving') : $t('settings.vpnMapping.save') }}

  div.settings-alert.settings-alert-danger(v-if="errore") {{ errore }}
  div.settings-alert.settings-alert-success(v-if="successo")
    | {{ $t('settings.vpnMapping.saved') }}
    span.settings-alert-close(@click="successo = false") ×

  div(v-if="caricando") {{ $t('settings.vpnMapping.loading') }}
  template(v-else)
    div.vpn-section
      div.settings-row-title.vpn-subtitle {{ $t('settings.vpnMapping.autoTitle') }}
      div.settings-row-help {{ $t('settings.vpnMapping.autoHelp') }}
      div.settings-row-help(v-if="vociAuto.length === 0") {{ $t('settings.vpnMapping.autoEmpty') }}
      div.vpn-list(v-else)
        div.vpn-row(v-for="v in vociAuto" :key="v.indirizzo")
          span.vpn-row-address {{ v.indirizzo }}
          span.vpn-row-arrow →
          span.vpn-row-client {{ v.cliente }}

    div.vpn-section
      div.settings-row-title.vpn-subtitle {{ $t('settings.vpnMapping.manualTitle') }}
      div.settings-row-help {{ $t('settings.vpnMapping.manualHelp') }}

      div.vpn-manual-row(v-for="(riga, i) in righeManuali" :key="riga.id")
        input.settings-field.vpn-input-address(v-model="riga.indirizzo" :placeholder="$t('settings.vpnMapping.addressPlaceholder')")
        input.settings-field.vpn-input-client(v-model="riga.cliente" :placeholder="$t('settings.vpnMapping.clientPlaceholder')")
        div.pill-btn-ghost.vpn-remove-btn(@click="rimuoviRiga(i)" :title="$t('settings.vpnMapping.removeRow')")
          icon(name="trash")

      div.pill-btn-ghost.vpn-add-btn(@click="aggiungiRiga")
        icon(name="plus")
        | {{ $t('settings.vpnMapping.addRow') }}
</template>

<script lang="ts">
import 'vue-awesome/icons/plus';
import 'vue-awesome/icons/trash';
import { invoke } from '@tauri-apps/api/core';

interface VoceMappingVpn {
  indirizzo: string;
  cliente: string;
  origine: 'openvpn' | 'manuale';
}

interface RigaManuale {
  id: number;
  indirizzo: string;
  cliente: string;
}

let contatoreId = 0;

export default {
  name: 'VpnMappingSettings',
  data() {
    return {
      caricando: true,
      salvando: false,
      errore: '',
      successo: false,
      vociAuto: [] as VoceMappingVpn[],
      righeManuali: [] as RigaManuale[],
      righeManualiSalvate: '',
    };
  },
  computed: {
    modificato(): boolean {
      return JSON.stringify(this.righeManuali.map(r => [r.indirizzo, r.cliente])) !== this.righeManualiSalvate;
    },
  },
  async mounted() {
    await this.carica();
  },
  methods: {
    async carica() {
      this.caricando = true;
      this.errore = '';
      try {
        const voci = await invoke<VoceMappingVpn[]>('leggi_mapping_vpn');
        this.vociAuto = voci.filter(v => v.origine === 'openvpn');
        this.righeManuali = voci
          .filter(v => v.origine === 'manuale')
          .map(v => ({ id: contatoreId++, indirizzo: v.indirizzo, cliente: v.cliente }));
        this.sincronizzaSalvate();
      } catch (e: any) {
        // Fuori da Tauri (dev server puro nel browser) invoke() non
        // esiste — stesso pattern già usato in CategorizationSettings.vue.
      } finally {
        this.caricando = false;
      }
    },
    sincronizzaSalvate() {
      this.righeManualiSalvate = JSON.stringify(this.righeManuali.map(r => [r.indirizzo, r.cliente]));
    },
    aggiungiRiga() {
      this.righeManuali.push({ id: contatoreId++, indirizzo: '', cliente: '' });
    },
    rimuoviRiga(i: number) {
      this.righeManuali.splice(i, 1);
    },
    scarta() {
      this.carica();
    },
    async salva() {
      this.errore = '';
      this.successo = false;

      const righeValide = this.righeManuali
        .map(r => ({ indirizzo: r.indirizzo.trim(), cliente: r.cliente.trim() }))
        .filter(r => r.indirizzo !== '' && r.cliente !== '');

      const indirizziVisti = new Set<string>();
      for (const riga of righeValide) {
        if (indirizziVisti.has(riga.indirizzo.toLowerCase())) {
          this.errore = this.$t('settings.vpnMapping.duplicateAddressError', {
            indirizzo: riga.indirizzo,
          }) as string;
          return;
        }
        indirizziVisti.add(riga.indirizzo.toLowerCase());
      }

      this.salvando = true;
      try {
        await invoke('salva_mapping_vpn_manuale', { voci: righeValide });
        this.successo = true;
        await this.carica();
      } catch (e: any) {
        this.errore = `${this.$t('settings.vpnMapping.saveError')} ${e?.message ?? e}`;
      } finally {
        this.salvando = false;
      }
    },
  },
};
</script>

<style scoped>
.settings-actions {
  display: flex;
  gap: 8px;
  flex-shrink: 0;
}

.settings-alert {
  margin-top: 12px;
  padding: 10px 14px;
  border-radius: var(--radius-md);
  border: 1px solid var(--color-border);
  font-size: var(--font-size-sm);
}

.settings-alert-danger {
  background-color: rgba(217, 83, 79, 0.15);
  color: var(--color-text);
}

.settings-alert-success {
  background-color: rgba(90, 176, 110, 0.15);
  color: var(--color-text);
  display: flex;
  justify-content: space-between;
}

.settings-alert-close {
  cursor: pointer;
  color: var(--color-text-faint);
}

.vpn-section {
  margin-top: 20px;
}

.vpn-subtitle {
  margin-bottom: 2px;
}

.vpn-list {
  margin-top: 10px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  overflow: hidden;
}

.vpn-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  border-bottom: 1px solid var(--color-border);
  background-color: var(--color-surface);
  font-size: var(--font-size-sm);
}

.vpn-row:last-child {
  border-bottom: none;
}

.vpn-row-address {
  color: var(--color-text);
  min-width: 180px;
}

.vpn-row-arrow {
  color: var(--color-text-faint);
}

.vpn-row-client {
  color: var(--color-text-dim);
}

.vpn-manual-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 10px;
}

.vpn-input-address {
  flex: 1 1 45%;
}

.vpn-input-client {
  flex: 1 1 35%;
}

.vpn-remove-btn {
  flex-shrink: 0;
  padding: 6px 10px;
}

.vpn-add-btn {
  margin-top: 12px;
  display: inline-flex;
  align-items: center;
  gap: 6px;
}
</style>

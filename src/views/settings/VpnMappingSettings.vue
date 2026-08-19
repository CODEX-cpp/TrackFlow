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
    div.settings-row-help {{ $t('settings.vpnMapping.tableHelp') }}
    div.settings-row-help(v-if="righe.length === 0") {{ $t('settings.vpnMapping.allEmpty') }}
    div.vpn-list(v-else)
      div.vpn-row(v-for="riga in righe" :key="riga.id")
        input.settings-field.vpn-input-client(
          v-model="riga.cliente"
          @input="riga.sovrascritta = true"
          :placeholder="$t('settings.vpnMapping.clientPlaceholder')"
        )
        input.settings-field.vpn-input-address(
          v-model="riga.indirizzo"
          :disabled="riga.origineAuto"
          :placeholder="$t('settings.vpnMapping.addressPlaceholder')"
        )
        span.vpn-row-origin(:class="{ 'vpn-row-origin-manual': !riga.origineAuto }")
          | {{ riga.origineAuto ? $t('settings.vpnMapping.originAuto') : $t('settings.vpnMapping.originManual') }}
        div.pill-btn-ghost.vpn-remove-btn(
          v-if="!riga.origineAuto"
          @click="rimuoviRiga(riga.id)"
          :title="$t('settings.vpnMapping.removeRow')"
        )
          icon(name="trash")
        div.vpn-remove-btn-spacer(v-else)

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

interface RigaTabella {
  id: number;
  indirizzo: string;
  cliente: string;
  // true = indirizzo noto da un profilo OpenVPN Connect reale — il campo
  // indirizzo resta non modificabile (non è TrackFlow a deciderlo), ma il
  // nome sì: digitarne uno diverso e salvare crea una sovrascrittura
  // "solo per TrackFlow", senza toccare il profilo OpenVPN vero — da quel
  // momento in poi questo indirizzo diventa a tutti gli effetti una riga
  // manuale (modificabile, rimovibile). false = riga già manuale (nuova
  // o già una sovrascrittura), sia indirizzo che nome modificabili e
  // rimovibile.
  origineAuto: boolean;
  // true solo dopo che l'utente ha effettivamente digitato nel campo
  // nome di una riga origineAuto — decide se salvare una sovrascrittura;
  // niente valore originale da confrontare, il nome digitato sostituisce
  // e basta.
  sovrascritta: boolean;
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
      righe: [] as RigaTabella[],
      righeSalvate: '',
    };
  },
  computed: {
    modificato(): boolean {
      return JSON.stringify(this.righe.map(r => [r.indirizzo, r.cliente])) !== this.righeSalvate;
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
        this.righe = voci
          .map(v => ({
            id: contatoreId++,
            indirizzo: v.indirizzo,
            cliente: v.cliente,
            origineAuto: v.origine === 'openvpn',
            sovrascritta: false,
          }))
          // Richiesta esplicita: ordine alfabetico per nome cliente, non
          // per indirizzo — calcolato solo qui (al caricamento), non come
          // computed reattivo: riordinare dal vivo mentre l'utente digita
          // un nome farebbe "saltare" la riga sotto al cursore.
          .sort((a, b) => a.cliente.localeCompare(b.cliente));
        this.sincronizzaSalvate();
      } catch (e: any) {
        // Fuori da Tauri (dev server puro nel browser) invoke() non
        // esiste — stesso pattern già usato in CategorizationSettings.vue.
      } finally {
        this.caricando = false;
      }
    },
    sincronizzaSalvate() {
      this.righeSalvate = JSON.stringify(this.righe.map(r => [r.indirizzo, r.cliente]));
    },
    aggiungiRiga() {
      this.righe.push({ id: contatoreId++, indirizzo: '', cliente: '', origineAuto: false, sovrascritta: false });
    },
    rimuoviRiga(id: number) {
      this.righe = this.righe.filter(r => r.id !== id);
    },
    scarta() {
      this.carica();
    },
    async salva() {
      this.errore = '';
      this.successo = false;

      const righeValide = this.righe
        .filter(r => {
          // Una riga "auto" genera una sovrascrittura solo se l'utente
          // ci ha davvero digitato dentro — altrimenti resta quella
          // letta dal profilo OpenVPN, senza scrivere nulla nel file.
          if (r.origineAuto) {
            return r.sovrascritta && r.cliente.trim() !== '';
          }
          return r.indirizzo.trim() !== '' && r.cliente.trim() !== '';
        })
        .map(r => ({ indirizzo: r.indirizzo.trim(), cliente: r.cliente.trim() }));

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

.vpn-list {
  margin-top: 12px;
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

.vpn-input-client,
.vpn-input-address {
  /* Larghezza fissa invece che percentuale: con `flex-grow` i due campi
     si dividevano lo spazio rimasto DOPO badge e pulsante rimuovi, che
     non hanno tutti la stessa larghezza ("auto" è più corto di
     "manuale", e le righe automatiche non hanno il pulsante rimuovi) —
     risultato, form di larghezza diversa da riga a riga. Segnalato
     dall'utente. */
  flex: 0 0 260px;
}

.vpn-input-address:disabled {
  color: var(--color-text-dim);
  background-color: transparent;
  border-color: transparent;
  cursor: default;
}

.vpn-row-origin {
  flex-shrink: 0;
  font-size: var(--font-size-xs);
  color: var(--color-text-faint);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  padding: 1px 8px;
  text-transform: uppercase;
  letter-spacing: 0.02em;
}

.vpn-row-origin-manual {
  color: var(--color-accent1);
  border-color: var(--color-accent1);
}

.vpn-remove-btn {
  flex-shrink: 0;
  padding: 6px 10px;
}

.vpn-remove-btn-spacer {
  flex-shrink: 0;
  width: 33px;
}

.vpn-add-btn {
  margin-top: 12px;
  display: inline-flex;
  align-items: center;
  gap: 6px;
}
</style>

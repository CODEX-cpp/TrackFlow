<template lang="pug">
div
  div.settings-row
    div
      div.settings-row-title {{ $t('settings.categorization.title') }}
      div.settings-row-help {{ $t('settings.categorization.help') }}

  div.cat-categories-row
    input.settings-field.cat-new-input(
      v-model="nuovaCategoria"
      type="text"
      :placeholder="$t('settings.categorization.newCategoryPlaceholder')"
      @keyup.enter="creaCategoria"
    )
    div.pill-btn(@click="creaCategoria" :class="{ 'pill-btn-disabled': !nuovaCategoria.trim() }") {{ $t('settings.categorization.addCategory') }}

  div.cat-chip-list(v-if="categorie.length > 0")
    div.cat-chip(
      v-for="c in categorie"
      :key="c.name"
      :class="{ 'cat-chip-selected': categoriaSelezionata === c.name }"
      @click="selezionaCategoria(c.name)"
    )
      span.cat-chip-name {{ c.name }}
      span.cat-chip-count {{ c.apps.length }}
      span.cat-chip-edit(@click.stop="apriModificaCategoria(c.name)")
        icon(name="pencil-alt")
      span.cat-chip-delete(@click.stop="categoriaDaEliminare = c.name") ×
  div.settings-row-help(v-else) {{ $t('settings.categorization.noCategories') }}

  div.cat-apps-section
    div.cat-apps-header
      h5.settings-row-title.cat-no-margin {{ $t('settings.categorization.appsTitle') }}
      div.cat-apps-controls
        input.settings-field.cat-search-input(
          v-model="filtroApp"
          type="text"
          :placeholder="$t('settings.categorization.searchPlaceholder')"
        )
        label.cat-filter-toggle
          input.custom-checkbox(type="checkbox" v-model="soloNonCategorizzate" @change="categoriaSelezionata = null")
          | {{ $t('settings.categorization.onlyUncategorized') }}

    div.cat-active-filter(v-if="categoriaSelezionata")
      | {{ $t('settings.categorization.filteredBy', { name: categoriaSelezionata }) }}
      span.cat-active-filter-clear(@click="categoriaSelezionata = null") × {{ $t('settings.categorization.clearFilter') }}

    div(v-if="caricandoApp") {{ $t('common.loading') }}
    div.settings-row-help(v-else-if="appConosciute.length === 0") {{ $t('settings.categorization.noApps') }}
    div.settings-row-help(v-else-if="appFiltrate.length === 0") {{ $t('settings.categorization.noAppsMatch') }}
    div.cat-app-list.themed-scroll(v-else)
      div.cat-app-row(v-for="a in appFiltrate" :key="a.app")
        img.cat-app-icon(
          v-if="!iconFalliti[a.app]"
          :src="iconUrlForApp(a.app)"
          @error="iconFalliti = { ...iconFalliti, [a.app]: true }"
          alt=""
        )
        span.cat-app-icon-fallback(v-else) {{ fallbackIconForApp(a.app) }}
        div.cat-app-names
          div.cat-app-name {{ a.nome_leggibile || a.app }}
          div.cat-app-raw-name(v-if="a.nome_leggibile") {{ a.app }}
        select.settings-field.cat-app-select(
          :value="categorieStore.categoryForApp(a.app) || ''"
          @change="assegnaApp(a.app, $event.target.value || null)"
        )
          option(value="") {{ $t('settings.categorization.uncategorized') }}
          option(v-for="c in categorie" :key="c.name" :value="c.name") {{ c.name }}

  confirm-modal(
    v-if="categoriaDaEliminare"
    :title="$t('settings.categorization.deleteConfirmTitle')"
    :confirm-label="$t('settings.categorization.deleteConfirm')"
    :cancel-label="$t('settings.categorization.deleteCancel')"
    @confirm="eliminaCategoriaConfermata"
    @cancel="categoriaDaEliminare = null"
  )
    div {{ $t('settings.categorization.deleteConfirmText', { name: categoriaDaEliminare }) }}

  confirm-modal(
    v-if="categoriaDaModificare"
    :title="$t('settings.categorization.editConfirmTitle', { name: categoriaDaModificare })"
    :confirm-label="$t('settings.categorization.editConfirm')"
    :cancel-label="$t('settings.categorization.editCancel')"
    confirm-variant="primary"
    @confirm="confermaModificaCategoria"
    @cancel="categoriaDaModificare = null"
  )
    div.cat-color-label {{ $t('settings.categorization.colorLabel') }}
    div.cat-color-picker
      div.cat-color-swatch(
        v-for="color in paletteColoriCategoria"
        :key="color"
        :class="{ 'cat-color-swatch-selected': colorSelezionato === color }"
        :style="{ backgroundColor: color }"
        @click="colorSelezionato = color"
      )
</template>

<script lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { useAppCategoriesStore } from '~/stores/appCategories';
import { iconUrlForApp, fallbackIconForApp } from '~/util/appNames';
import { CATEGORY_COLOR_PALETTE } from '~/util/hashColor';
import 'vue-awesome/icons/pencil-alt';

interface AppConosciuta {
  app: string;
  nome_leggibile: string | null;
}

export default {
  name: 'CategorizationSettings',
  components: {
    'confirm-modal': () => import('~/components/ConfirmModal.vue'),
  },
  data() {
    return {
      categorieStore: useAppCategoriesStore(),
      nuovaCategoria: '',
      categoriaDaEliminare: null as string | null,
      // Popup di modifica — per ora solo il colore (icona penna → 15
      // cerchi pastello, vedi CATEGORY_COLOR_PALETTE in util/hashColor.ts):
      // base su cui costruire altra modifica (rinomina, ecc.) in un
      // secondo momento.
      categoriaDaModificare: null as string | null,
      // Colore selezionato nel popup — inizializzato al colore GIÀ
      // salvato per la categoria (apriModificaCategoria sotto), non a
      // quello automatico calcolato dal nome: così l'anello giallo mostra
      // "nessuna scelta ancora fatta" per una categoria mai modificata,
      // invece di far sembrare selezionato un cerchio a caso.
      colorSelezionato: null as string | null,
      categoriaSelezionata: null as string | null,
      appConosciute: [] as AppConosciuta[],
      caricandoApp: true,
      filtroApp: '',
      soloNonCategorizzate: false,
      iconFalliti: {} as Record<string, boolean>,
    };
  },
  computed: {
    categorie() {
      return this.categorieStore.categories;
    },
    paletteColoriCategoria() {
      return CATEGORY_COLOR_PALETTE;
    },
    appFiltrate(): AppConosciuta[] {
      const query = this.filtroApp.trim().toLowerCase();
      return this.appConosciute.filter(a => {
        const categoriaApp = this.categorieStore.categoryForApp(a.app);
        if (this.categoriaSelezionata) {
          if (categoriaApp !== this.categoriaSelezionata) return false;
        } else if (this.soloNonCategorizzate && categoriaApp) {
          return false;
        }
        if (!query) return true;
        return (
          a.app.toLowerCase().includes(query) ||
          (a.nome_leggibile || '').toLowerCase().includes(query)
        );
      });
    },
  },
  async mounted() {
    try {
      this.appConosciute = await invoke<AppConosciuta[]>('elenca_app_conosciute');
    } catch (e) {
      // L'app potrebbe girare fuori da Tauri durante lo sviluppo web puro
      // (npx vite senza il guscio nativo) — invoke() non esiste in quel
      // caso, non è un errore da mostrare all'utente. Stesso pattern già
      // usato in AiAgentSettings.vue.
    } finally {
      this.caricandoApp = false;
    }
  },
  methods: {
    iconUrlForApp,
    fallbackIconForApp,
    async creaCategoria() {
      const nome = this.nuovaCategoria.trim();
      if (!nome) return;
      await this.categorieStore.createCategory(nome);
      this.nuovaCategoria = '';
    },
    selezionaCategoria(nome: string) {
      // Click di nuovo sulla stessa categoria già selezionata = deseleziona
      // (mostra di nuovo tutte le app) — un solo click per attivare/disattivare
      // il filtro, niente pulsante separato per "annulla" oltre a quello
      // esplicito già presente sotto la barra di ricerca.
      this.categoriaSelezionata = this.categoriaSelezionata === nome ? null : nome;
      // Le due modalità di filtro non hanno senso insieme (categoria X
      // + "solo non categorizzate" darebbe sempre lista vuota).
      this.soloNonCategorizzate = false;
    },
    async eliminaCategoriaConfermata() {
      if (!this.categoriaDaEliminare) return;
      await this.categorieStore.deleteCategory(this.categoriaDaEliminare);
      // Se era anche il filtro attivo, toglierlo — restare filtrati su una
      // categoria appena cancellata mostrerebbe una lista vuota senza
      // nessuna spiegazione visibile.
      if (this.categoriaSelezionata === this.categoriaDaEliminare) {
        this.categoriaSelezionata = null;
      }
      this.categoriaDaEliminare = null;
    },
    apriModificaCategoria(nome: string) {
      const cat = this.categorie.find((c: any) => c.name === nome);
      this.colorSelezionato = (cat && cat.color) || null;
      this.categoriaDaModificare = nome;
    },
    async confermaModificaCategoria() {
      if (!this.categoriaDaModificare) return;
      await this.categorieStore.setCategoryColor(this.categoriaDaModificare, this.colorSelezionato);
      this.categoriaDaModificare = null;
    },
    async assegnaApp(app: string, categoria: string | null) {
      await this.categorieStore.assignApp(app, categoria);
    },
  },
};
</script>

<style scoped>
.cat-categories-row {
  display: flex;
  gap: 8px;
  align-items: center;
  margin-top: 4px;
}

.cat-new-input {
  flex: 0 1 260px;
}

.cat-chip-list {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 14px;
}

.cat-chip {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 10px;
  border-radius: var(--radius-md);
  background-color: var(--color-surface2);
  font-size: var(--font-size-sm);
  color: var(--color-text);
  cursor: pointer;
  border: 1px solid transparent;
  /* line-height:1 su tutto il chip — senza, nome/numero/icone (font-size
     diversi tra loro) prendevano ciascuno l'altezza di riga naturale del
     proprio font-size invece di una comune, facendoli sembrare storti
     l'uno rispetto all'altro anche con align-items:center sul flex. */
  line-height: 1;
}

.cat-chip:hover {
  filter: brightness(1.15);
}

.cat-chip-selected {
  border-color: var(--color-accent1);
  background-color: var(--color-accent1);
  color: #241a12;
}

.cat-chip-selected .cat-chip-count {
  color: #241a12;
  opacity: 0.7;
}

.cat-chip-count {
  color: var(--color-text-faint);
  font-size: var(--font-size-xs);
}

/* Stesso box fisso (16x16, contenuto centrato) per penna e ×, invece
   di due span "a misura di contenuto" con dimensioni/line-height
   diverse tra loro — era quello a farle sembrare posizionate a
   altezze/dimensioni diverse l'una dall'altra, segnalato dall'utente. */
.cat-chip-edit,
.cat-chip-delete {
  cursor: pointer;
  color: var(--color-text-faint);
  display: flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
  flex-shrink: 0;
  line-height: 1;
}

.cat-chip-edit:hover {
  color: var(--color-accent1);
}

.cat-chip-delete {
  font-size: 15px;
}

.cat-chip-delete:hover {
  color: var(--color-danger);
}

.cat-chip-edit svg {
  display: block;
  width: 10px;
  height: 10px;
  /* Piccolo aggiustamento a occhio verso il basso — richiesta esplicita
     dell'utente, il glifo della matita ha più "aria" visiva sopra che
     sotto rispetto alla × accanto, anche a box/dimensioni ormai
     identici tra i due. */
  position: relative;
  top: 1px;
}

.cat-color-label {
  font-size: var(--font-size-sm);
  color: var(--color-text-dim);
  margin-bottom: 10px;
}

.cat-color-picker {
  display: flex;
  flex-wrap: wrap;
  /* justify-content:center (non solo flex-wrap) — richiesta esplicita
     dell'utente: senza, ogni riga (compresa l'ultima, spesso non piena)
     si allinea a sinistra invece che al centro del popup. Dimensione
     cerchio/gap scelta apposta con CATEGORY_COLOR_PALETTE (21 colori,
     vedi util/hashColor.ts) perché entrino ESATTAMENTE 7 per riga nei
     292px interni del popup (340px - 24px di padding per lato) — 3
     righe piene, nessun cerchio isolato in fondo. */
  justify-content: center;
  gap: 8px;
}

.cat-color-swatch {
  width: 26px;
  height: 26px;
  border-radius: 50%;
  cursor: pointer;
  flex-shrink: 0;
}

.cat-color-swatch:hover {
  filter: brightness(1.1);
}

/* Anello giallo attorno al cerchio selezionato — richiesta esplicita
   dell'utente. box-shadow invece di border/outline: un anello via
   box-shadow segue sempre il border-radius:50% del cerchio in modo
   pulito, con un piccolo distacco (il primo box-shadow, colore sfondo
   del popup) tra lo swatch e l'anello vero e proprio. */
.cat-color-swatch-selected {
  box-shadow: 0 0 0 3px var(--color-bg-elev), 0 0 0 5px #f2c94c;
}

.cat-apps-section {
  margin-top: 26px;
}

.cat-apps-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: 10px;
  margin-bottom: 12px;
}

.cat-no-margin {
  margin: 0;
}

.cat-apps-controls {
  display: flex;
  align-items: center;
  gap: 12px;
}

.cat-search-input {
  width: 200px;
}

.cat-filter-toggle {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: var(--font-size-sm);
  color: var(--color-text-dim);
  cursor: pointer;
  white-space: nowrap;
}

.cat-active-filter {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: var(--font-size-sm);
  color: var(--color-text-dim);
  margin-bottom: 10px;
}

.cat-active-filter-clear {
  cursor: pointer;
  color: var(--color-text-faint);
}

.cat-active-filter-clear:hover {
  color: var(--color-text);
}

.cat-app-list {
  max-height: 420px;
  overflow-y: auto;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
}

.cat-app-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  border-bottom: 1px solid var(--color-border);
}

.cat-app-row:last-child {
  border-bottom: none;
}

.cat-app-icon,
.cat-app-icon-fallback {
  width: 24px;
  height: 24px;
  flex-shrink: 0;
  border-radius: var(--radius-sm);
  object-fit: contain;
}

.cat-app-icon-fallback {
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 15px;
}

.cat-app-names {
  flex: 1;
  min-width: 0;
}

.cat-app-name {
  font-size: var(--font-size-sm);
  color: var(--color-text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.cat-app-raw-name {
  font-size: var(--font-size-xs);
  color: var(--color-text-faint);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.cat-app-select {
  flex: 0 0 200px;
}
</style>

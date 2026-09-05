<template lang="pug">
div
  div.modal-backdrop(@click="$emit('close')")
  div.edit-modal.gallery-modal.themed-scroll
    div.gallery-header
      div.gallery-header-top
        div.gallery-eyebrow {{ $t('home.screenshotGallery.eyebrow') }}
        div.gallery-header-right
          div.gallery-tabs
            button.gallery-tab(
              type="button"
              :class="{ 'gallery-tab-active': activeTab === 'cronologica' }"
              @click="activeTab = 'cronologica'"
            ) {{ $t('home.screenshotGallery.tabChronological') }}
            button.gallery-tab(
              type="button"
              :class="{ 'gallery-tab-active': activeTab === 'perFinestra' }"
              @click="activeTab = 'perFinestra'"
            ) {{ $t('home.screenshotGallery.tabByWindow') }}
          button.gallery-close-btn(type="button" @click="$emit('close')" :title="$t('home.screenshotGallery.close')")
            icon(name="times")
      div.gallery-title {{ displayName }}
      div.gallery-meta(v-if="flatScreenshots.length") {{ metaText }}

    div.gallery-empty(v-if="!flatScreenshots.length") {{ $t('home.screenshotGallery.empty') }}

    template(v-else-if="activeTab === 'cronologica'")
      // Un chip per titolo — richiesta esplicita: cliccandolo filtra la
      // griglia sotto a quel solo titolo (un secondo click sullo stesso
      // chip toglie il filtro). Il colore del pallino è lo stesso bordo
      // che ogni miniatura di quel titolo porta in cima, così i due si
      // leggono come lo stesso codice colore.
      div.gallery-chips(v-if="groups.length > 1")
        button.gallery-chip(
          v-for="g in groups"
          :key="g.title"
          type="button"
          :class="{ 'gallery-chip-active': filtroTitolo === g.title }"
          :style="{ '--chip-color': coloreForTitle(g.title) }"
          @click="toggleFiltro(g.title)"
        )
          span.gallery-chip-dot
          span.gallery-chip-label {{ g.title }}
          span.gallery-chip-count {{ g.shots.length }}

      div.gallery-grid
        div.gallery-thumb(
          v-for="s in screenshotsVisibili"
          v-show="!brokenFilenames[s.filename]"
          :key="s.filename"
          :title="s.title"
          :style="{ '--thumb-color': coloreForTitle(s.title) }"
          @click="openLightbox(s)"
        )
          div.gallery-thumb-img-wrap
            img.gallery-thumb-img(
              :src="'/pages/app-data/screenshots/' + s.filename"
              @error="$set(brokenFilenames, s.filename, true)"
            )
          div.gallery-thumb-label
            div.gallery-thumb-time {{ s.time }}
            div.gallery-thumb-title {{ s.title }}

    template(v-else)
      div.gallery-body-window
        div.gallery-sidebar
          div.gallery-sidebar-item(
            v-for="g in groups"
            :key="g.title"
            :class="{ 'gallery-sidebar-item-active': finestraSelezionata === g.title }"
            :style="{ '--item-color': coloreForTitle(g.title) }"
            @click="finestraSelezionata = g.title"
          )
            span.gallery-sidebar-dot
            div.gallery-sidebar-text
              div.gallery-sidebar-title {{ g.title }}
              div.gallery-sidebar-sub {{ rangeForGroup(g) }} · {{ $t('home.screenshotGallery.photoCount', { count: g.shots.length }) }}
        div.gallery-window-panel
          template(v-if="gruppoSelezionato")
            div.gallery-window-head
              span.gallery-window-name {{ gruppoSelezionato.title }}
              span.gallery-window-range {{ rangeForGroup(gruppoSelezionato) }} · {{ $t('home.screenshotGallery.photoCount', { count: gruppoSelezionato.shots.length }) }}
            div.gallery-grid
              div.gallery-thumb(
                v-for="s in gruppoSelezionato.shots"
                v-show="!brokenFilenames[s.filename]"
                :key="s.filename"
                :title="s.title"
                :style="{ '--thumb-color': coloreForTitle(s.title) }"
                @click="openLightbox(s)"
              )
                div.gallery-thumb-img-wrap
                  img.gallery-thumb-img(
                    :src="'/pages/app-data/screenshots/' + s.filename"
                    @error="$set(brokenFilenames, s.filename, true)"
                  )
                div.gallery-thumb-label
                  div.gallery-thumb-time {{ s.time }}
                  div.gallery-thumb-title {{ s.title }}

    div.gallery-footer(v-if="flatScreenshots.length")
      span.gallery-footer-status {{ $t('home.screenshotGallery.footerStatus', { count: flatScreenshots.length }) }}
      div.pill-btn-ghost(@click="$emit('close')") {{ $t('home.screenshotGallery.close') }}

  div.modal-backdrop.gallery-lightbox-backdrop(v-if="lightboxIndex !== null" @click="closeLightbox")
  div.gallery-lightbox(v-if="lightboxIndex !== null")
    div.lightbox-topbar(@click.stop)
      div.lightbox-info
        div.lightbox-index
          span.lightbox-index-current {{ lightboxIndex + 1 }}
          |  / {{ flatScreenshots.length }}
        div.lightbox-title {{ currentShot.title }}
        div.lightbox-time {{ currentShot.time }}
      div.lightbox-actions
        span.lightbox-zoom-level {{ imageZoom.toFixed(1) }}×
        div.pill-btn-ghost(@click="resetZoom") {{ $t('home.screenshotGallery.resetZoom') }}
        div.pill-btn-ghost.lightbox-close-btn(@click="closeLightbox")
          | {{ $t('home.screenshotGallery.close') }}
          icon(name="times")

    div.lightbox-stage
      div.gallery-lightbox-nav.gallery-lightbox-prev(
        v-if="lightboxIndex > 0"
        @click.stop="prev"
      )
        icon(name="angle-left")
      img.gallery-lightbox-img(
        :class="{ 'gallery-lightbox-img-zoomed': imageZoom > 1, 'gallery-lightbox-img-dragging': isDragging }"
        :src="'/pages/app-data/screenshots/' + currentShot.filename"
        :style="{ transform: 'translate(' + panX + 'px, ' + panY + 'px) scale(' + imageZoom + ')' }"
        draggable="false"
        @click.stop
        @wheel="onImageWheel"
        @mousedown.stop="onImageMouseDown"
      )
      div.gallery-lightbox-nav.gallery-lightbox-next(
        v-if="lightboxIndex < flatScreenshots.length - 1"
        @click.stop="next"
      )
        icon(name="angle-right")

    // Barra cronologica dell'intero blocco (non solo delle foto
    // attualmente filtrate/visibili in griglia) — richiesta esplicita:
    // trascinando l'indicatore ambra ci si sposta nel tempo, la
    // rotellina sopra la barra avanza/arretra di una foto alla volta.
    // I segmenti colorati usano lo stesso codice colore dei chip/della
    // griglia, quello davvero "attivo" (contenente la foto aperta) ha un
    // bordo evidenziato.
    div.lightbox-scrubber(@click.stop)
      div.lightbox-ticks
        span.lightbox-tick(
          v-for="(t, i) in timelineTicks"
          :key="i"
          :style="{ left: t.pct + '%' }"
        ) {{ t.label }}
      div.lightbox-track-frame
        div.lightbox-track(
          ref="scrubTrack"
          @mousedown="onScrubMouseDown"
          @wheel="onScrubWheel"
        )
          div.lightbox-segment(
            v-for="(seg, i) in timelineSegments"
            :key="i"
            :class="{ 'lightbox-segment-active': i === currentSegmentIndex }"
            :style="{ left: seg.leftPct + '%', width: seg.widthPct + '%', '--seg-color': seg.color }"
          )
            span.lightbox-segment-label {{ seg.title }}
          // Stesso genitore (.lightbox-track) del pallino subito sopra —
          // devono condividere ESATTAMENTE lo stesso riferimento
          // percentuale, altrimenti finiscono disallineati (bug reale
          // segnalato dall'utente, causato da padding/bordi diversi tra
          // i due contenitori quando erano separati).
          div.lightbox-handle(:style="{ left: currentPct + '%' }")
          div.lightbox-handle-time(:style="{ left: currentPct + '%' }") {{ currentTimeBreve }}
      // Tacche minori FUORI dalla cornice — richiesta esplicita: dentro,
      // lo spazio vuoto sotto la barra colorata la faceva percepire
      // spostata verso l'alto invece che centrata nella cornice.
      div.lightbox-minor-ticks
        span.lightbox-minor-tick(
          v-for="(pct, i) in timelineMinorTicks"
          :key="i"
          :style="{ left: pct + '%' }"
        )
      div.lightbox-hint {{ $t('home.screenshotGallery.scrubHint') }}
</template>

<style lang="scss" scoped>
@import '../style/theme.css';
@import '../style/modals.css';

// Deliberately much wider than the standard .edit-modal (340px) — this
// holds grouped thumbnail grids, not a form. Altezza FISSA (non solo
// max-height) — bug reale segnalato dall'utente: passando da un titolo
// all'altro nella vista "Per finestra" il numero di foto cambia, e con
// una semplice max-height il popup si ridimensionava ad ogni click
// invece di restare fermo. Le sezioni interne (griglia, sidebar) sono
// quelle che scorrono, non il popup nel suo insieme.
.gallery-modal {
  width: 1200px;
  max-width: calc(100vw - 48px);
  height: 680px;
  max-height: calc(100vh - 48px);
  display: flex;
  flex-direction: column;
  padding: 0;
  overflow: hidden;
}

.gallery-header {
  padding: 20px 24px 16px;
  flex-shrink: 0;
}

.gallery-header-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.gallery-eyebrow {
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-bold);
  letter-spacing: var(--letter-spacing-wide);
  text-transform: uppercase;
  color: var(--color-accent1);
}

.gallery-header-right {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-shrink: 0;
}

.gallery-tabs {
  display: flex;
  background-color: var(--color-surface2);
  border-radius: var(--radius-pill);
  padding: 3px;
  gap: 2px;
}

.gallery-tab {
  border: none;
  background: transparent;
  color: var(--color-text-dim);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-semibold);
  padding: 6px 14px;
  border-radius: var(--radius-pill);
  cursor: pointer;
}

.gallery-tab:hover {
  color: var(--color-text);
}

.gallery-tab-active,
.gallery-tab-active:hover {
  background-color: var(--color-accent1);
  color: #241a12;
}

.gallery-close-btn {
  flex-shrink: 0;
  width: 30px;
  height: 30px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  background: transparent;
  border-radius: var(--radius-sm);
  color: var(--color-text-faint);
  cursor: pointer;
  font-size: 14px;
}

.gallery-close-btn:hover {
  background-color: var(--color-surface2);
  color: var(--color-text);
}

.gallery-title {
  font-size: var(--font-size-xl);
  font-weight: var(--font-weight-bold);
  color: var(--color-text);
  margin-top: 6px;
}

.gallery-meta {
  font-size: var(--font-size-sm);
  color: var(--color-text-faint);
  margin-top: 4px;
  font-variant-numeric: tabular-nums;
}

.gallery-empty {
  color: var(--color-text-faint);
  font-size: var(--font-size-sm);
  padding: 0 24px 24px;
}

// Riga di chip filtro, uno per titolo — solo nella vista Cronologica e
// solo quando ha senso scegliere (più di un titolo nel blocco).
.gallery-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  padding: 0 24px 14px;
  flex-shrink: 0;
}

.gallery-chip {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  border: 1px solid var(--color-border);
  background-color: var(--color-surface2);
  color: var(--color-text-dim);
  border-radius: var(--radius-pill);
  padding: 5px 12px;
  font-size: var(--font-size-xs);
  cursor: pointer;
  max-width: 260px;
}

.gallery-chip:hover {
  border-color: var(--thumb-color, var(--color-accent1));
}

.gallery-chip-active {
  background-color: color-mix(in srgb, var(--chip-color) 20%, var(--color-surface2));
  border-color: var(--chip-color);
  color: var(--color-text);
}

.gallery-chip-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background-color: var(--chip-color);
  flex-shrink: 0;
}

.gallery-chip-label {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.gallery-chip-count {
  color: var(--color-text-faint);
  font-variant-numeric: tabular-nums;
}

.gallery-chip-active .gallery-chip-count {
  color: var(--color-text-dim);
}

// Griglia di miniature grandi — sostituisce le vecchie righe compatte
// da 90px raggruppate sotto un titolo di testo: qui il colore del bordo
// superiore basta a capire a quale titolo appartiene ciascuna, senza
// bisogno di intestazioni separate (tranne nella vista "Per finestra",
// dove è comunque una sola alla volta). Spaziatura più ampia (24px, non
// 12px) — richiesta esplicita: le foto risultavano troppo ravvicinate.
// Flexbox a capo automatico, non CSS Grid — bug reale trovato in
// verifica: con `display:grid` + colonne 1fr, l'altezza di ogni riga
// veniva calcolata considerando solo il riquadro con aspect-ratio
// (l'immagine), e l'etichetta orario/titolo sotto restava tagliata via
// dall'overflow:hidden, invisibile in ogni caso (anche con
// align-items:start, che non basta a risolverlo). Con flex-wrap ogni
// scheda si dimensiona per conto proprio sul contenuto reale (immagine
// + etichetta), nessuna "riga" condivisa che possa tagliarla.
.gallery-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 24px;
  padding: 4px 24px 20px;
  // Riempie lo spazio rimasto nel popup (altezza fissa, vedi
  // .gallery-modal) e scorre al suo interno — min-height:0 è necessario
  // perché un figlio flex non si restringe mai sotto il proprio
  // contenuto per default, che vanificherebbe l'altezza fissa del popup.
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  align-content: flex-start;
}

.gallery-thumb {
  cursor: pointer;
  // 4 per riga (non 5): richiesta esplicita, foto più larghe — 3 gap da
  // 24px sottratti dalla larghezza disponibile.
  flex: 0 0 calc((100% - 3 * 24px) / 4);
  width: calc((100% - 3 * 24px) / 4);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  overflow: hidden;
  background-color: var(--color-surface2);
  transition: transform 0.1s ease;
}

.gallery-thumb:hover {
  transform: translateY(-2px);
}

.gallery-thumb-img-wrap {
  aspect-ratio: 16 / 10;
  border-top: 3px solid var(--thumb-color, var(--color-border));
  margin-top: -1px; // copre il bordo esterno, la striscia colorata resta l'unico bordo visibile in cima
}

.gallery-thumb-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}

// Etichetta attaccata sotto la foto, dentro la stessa card — orario a
// sinistra in monospace (stesso trattamento di .bucket-id in
// Buckets.vue) nel colore del titolo, con il titolo stesso subito sotto.
.gallery-thumb-label {
  padding: 6px 8px;
  border-top: 1px solid var(--color-border);
  background-color: var(--color-bg-elev);
}

.gallery-thumb-time {
  font-family: ui-monospace, SFMono-Regular, 'SF Mono', Menlo, Consolas, monospace;
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-semibold);
  color: var(--thumb-color, var(--color-accent1));
}

.gallery-thumb-title {
  font-size: var(--font-size-xs);
  color: var(--color-text-faint);
  margin-top: 2px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

// Vista "Per finestra": sidebar a sinistra (elenco titoli) + pannello a
// destra con la sola griglia del titolo selezionato.
.gallery-body-window {
  display: flex;
  min-height: 0;
  flex: 1;
  overflow: hidden;
}

.gallery-sidebar {
  width: 260px;
  flex-shrink: 0;
  overflow-y: auto;
  padding: 0 12px 20px 24px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.gallery-sidebar-item {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 10px 12px;
  border-radius: var(--radius-md);
  border-left: 3px solid transparent;
  cursor: pointer;
}

.gallery-sidebar-item:hover {
  background-color: var(--color-surface2);
}

.gallery-sidebar-item-active {
  background-color: color-mix(in srgb, var(--item-color) 15%, transparent);
  border-left-color: var(--item-color);
}

.gallery-sidebar-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background-color: var(--item-color);
  flex-shrink: 0;
  margin-top: 5px;
}

.gallery-sidebar-text {
  min-width: 0;
}

.gallery-sidebar-title {
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-semibold);
  color: var(--color-text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.gallery-sidebar-sub {
  font-size: var(--font-size-xs);
  color: var(--color-text-faint);
  margin-top: 2px;
  font-variant-numeric: tabular-nums;
}

.gallery-window-panel {
  flex: 1;
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.gallery-window-head {
  display: flex;
  align-items: baseline;
  gap: 10px;
  padding: 0 24px 14px;
  flex-shrink: 0;
}

.gallery-window-name {
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
  color: var(--color-text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.gallery-window-range {
  font-size: var(--font-size-xs);
  color: var(--color-text-faint);
  white-space: nowrap;
  font-variant-numeric: tabular-nums;
}

.gallery-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 14px 24px;
  border-top: 1px solid var(--color-border);
  flex-shrink: 0;
}

.gallery-footer-status {
  font-size: var(--font-size-xs);
  color: var(--color-text-faint);
}

// Stacked above the gallery modal itself (z-index 50, see modals.css) —
// a click on the backdrop closes it, same "click outside to dismiss"
// convention as every other modal here.
.gallery-lightbox-backdrop {
  z-index: 60;
}

// Ridisegnata (Claude Design, richiesta esplicita): non più solo
// un'immagine centrata con una didascalia pill in basso, ma tre fasce
// impilate — barra info in alto, immagine al centro, barra cronologica
// scorrevole in basso — su tutta l'altezza della finestra.
.gallery-lightbox {
  position: fixed;
  inset: 0;
  z-index: 61;
  display: flex;
  flex-direction: column;
  pointer-events: none;
  // Sfondo pieno (non solo il backdrop semitrasparente sotto) — bug
  // reale trovato in verifica: la barra info in alto a sinistra finiva
  // per sovrapporsi visivamente al logo della sidebar, ancora visibile
  // in trasparenza attraverso i due soli backdrop semitrasparenti
  // impilati.
  background-color: #0a0b0d;
}

.lightbox-topbar {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  padding: 20px 28px 0;
  pointer-events: auto;
}

.lightbox-index {
  font-size: var(--font-size-xs);
  color: var(--color-text-faint);
}

.lightbox-index-current {
  color: var(--color-accent1);
  font-weight: var(--font-weight-bold);
}

.lightbox-title {
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
  color: var(--color-text);
  margin-top: 2px;
  max-width: 60vw;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.lightbox-time {
  font-size: var(--font-size-xs);
  color: var(--color-text-faint);
  margin-top: 2px;
  font-variant-numeric: tabular-nums;
}

.lightbox-actions {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-shrink: 0;
}

.lightbox-zoom-level {
  font-size: var(--font-size-xs);
  color: var(--color-text-faint);
  font-variant-numeric: tabular-nums;
  min-width: 28px;
  text-align: right;
}

.lightbox-close-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.lightbox-close-btn ::v-deep svg {
  width: 11px;
  height: 11px;
  fill: currentColor;
}

.lightbox-stage {
  flex: 1;
  min-height: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 16px 32px;
  position: relative;
  // Più scuro del resto della lightbox — richiesta esplicita, lo sfondo
  // dietro la foto deve risaltare meno della foto stessa.
  background-color: #000;
}

.gallery-lightbox-img {
  max-width: 100%;
  max-height: 100%;
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-elevated);
  pointer-events: auto;
  cursor: zoom-in;
  // scale()/translate() are relative to the image's own (already
  // max-width/height clamped) box, so a zoomed-in image can extend past
  // the stage's padding — expected, panning (see below) is exactly
  // what lets you reach the parts that go offscreen.
  transition: transform 0.08s ease-out;
  user-select: none;
}

// Explicit follow-up request: once zoomed in, the image needs to be
// draggable to reach the parts that scaled outside the visible area —
// grab/grabbing cursors signal that, and the transition above is
// dropped while actively dragging so the image tracks the cursor
// directly instead of easing behind it.
.gallery-lightbox-img-zoomed {
  cursor: grab;
}

.gallery-lightbox-img-dragging {
  cursor: grabbing;
  transition: none;
}

// Frecce minute e neutre (non più un cerchio pieno) — coerenti con lo
// stile più discreto del resto del redesign.
.gallery-lightbox-nav {
  position: absolute;
  top: 50%;
  transform: translateY(-50%);
  width: 40px;
  height: 40px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--color-text-faint);
  cursor: pointer;
  pointer-events: auto;
  user-select: none;
}

.gallery-lightbox-nav ::v-deep svg {
  width: 22px;
  height: 22px;
  fill: currentColor;
}

.gallery-lightbox-nav:hover {
  color: var(--color-text);
}

.gallery-lightbox-prev {
  left: 12px;
}

.gallery-lightbox-next {
  right: 12px;
}

// Barra cronologica in fondo — tick orari, segmenti colorati per
// titolo (stesso codice colore di chip/sidebar/miniature) e
// l'indicatore trascinabile della foto aperta.
.lightbox-scrubber {
  position: relative;
  flex-shrink: 0;
  padding: 8px 28px 16px;
  pointer-events: auto;
  // Rete di sicurezza oltre al preventDefault() sul mousedown — mai
  // selezionabile, non è testo con cui l'utente debba interagire così.
  user-select: none;
}

.lightbox-ticks {
  position: relative;
  height: 16px;
}

.lightbox-tick {
  position: absolute;
  top: 0;
  transform: translateX(-50%);
  font-size: 10px;
  color: var(--color-text-faint);
  white-space: nowrap;
  font-variant-numeric: tabular-nums;
}

// Cornice grigia attorno a segmenti + tacche minori — richiesta
// esplicita: la barra doveva risultare più visibile/definita.
.lightbox-track-frame {
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  // Padding simmetrico — bug reale segnalato dall'utente: con un
  // padding sbilanciato (più sotto che sopra) la barra colorata dentro
  // sembrava spostata verso l'alto invece che centrata nella cornice.
  padding: 5px 0;
  background-color: var(--color-bg-elev);
}

.lightbox-track {
  position: relative;
  height: 26px;
  cursor: pointer;
}

// Riga di tacche fitte (una ogni frazione dell'intervallo delle
// etichette sopra) — richiesta esplicita: rendere la barra più
// leggibile mostrando il passare del tempo anche tra un'etichetta
// oraria e l'altra, non solo ai punti etichettati.
.lightbox-minor-ticks {
  position: relative;
  height: 8px;
  margin-top: 6px;
}

// Più visibili — richiesta esplicita: col colore/altezza precedenti si
// vedevano a malapena.
.lightbox-minor-tick {
  position: absolute;
  top: 0;
  width: 1px;
  height: 8px;
  background-color: var(--color-text-faint);
  transform: translateX(-50%);
}

.lightbox-segment {
  position: absolute;
  top: 2px;
  height: 22px;
  min-width: 3px;
  border-radius: var(--radius-sm);
  border: 1px solid var(--seg-color);
  background-color: color-mix(in srgb, var(--seg-color) 18%, var(--color-bg-elev));
  overflow: hidden;
  display: flex;
  align-items: center;
  padding: 0 6px;
}

.lightbox-segment-active {
  border-width: 2px;
  background-color: color-mix(in srgb, var(--seg-color) 32%, var(--color-bg-elev));
}

.lightbox-segment-label {
  font-size: 10px;
  color: var(--color-text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

// Indicatore ambra trascinabile — sopra la fila dei segmenti, con la
// linea/tempo separati sotto (vedi .lightbox-handle-time).
.lightbox-handle {
  position: absolute;
  top: -3px;
  width: 12px;
  height: 12px;
  margin-left: -6px;
  border-radius: 50%;
  background-color: var(--color-accent1);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-accent1) 35%, transparent);
  cursor: grab;
  z-index: 1;
}

.lightbox-handle-time {
  position: absolute;
  top: 30px;
  transform: translateX(-50%);
  display: inline-block;
  background-color: var(--color-accent1);
  color: #241a12;
  font-size: 10px;
  font-weight: var(--font-weight-bold);
  border-radius: var(--radius-sm);
  padding: 2px 6px;
  font-variant-numeric: tabular-nums;
  pointer-events: none;
}

.lightbox-hint {
  margin-top: 24px;
  font-size: var(--font-size-xs);
  color: var(--color-text-faint);
  text-align: center;
}
</style>

<script lang="ts">
// Screenshot gallery for a Timeline block's detail popup — split out of
// TimelineBlockDetailModal.vue (explicit request): thumbnails used to
// sit inline there with no connection to *when* in the block they
// happened, so seeing "the screenshots of activity X" meant manually
// matching timestamps between the Top Window Titles breakdown and the
// screenshot strip by hand. This groups screenshots by whichever window
// title was actually active when each was taken (via titleSegments'
// real start/end times — the caller, TimelineBlockDetailModal.vue,
// passes only the segments/screenshots for whichever title's "N foto"
// link was clicked, already scoped to that title's own occurrences).
//
// Redesign (Claude Design, richiesta esplicita): due viste — Cronologica
// (griglia unica ordinata nel tempo, colore in cima a ogni miniatura +
// chip filtro per titolo) e Per finestra (sidebar + una sola griglia per
// titolo alla volta) — sostituiscono la vecchia unica vista con sezioni
// impilate verticalmente, stessa logica di raggruppamento sotto.
import moment from 'moment';
import 'vue-awesome/icons/angle-left';
import 'vue-awesome/icons/angle-right';
import 'vue-awesome/icons/times';
import { colorePerGiorno } from '~/util/dailyColorPalette';

interface Screenshot {
  filename: string;
  time: string;
  timestamp: moment.Moment;
}

interface TitleSegment {
  title: string;
  start: moment.Moment;
  end: moment.Moment;
}

interface FinestraReale {
  app: string;
  start: moment.Moment;
  end: moment.Moment;
}

type ScreenshotConTitolo = Screenshot & { title: string };

export default {
  name: 'ScreenshotGalleryModal',
  props: {
    screenshots: { type: Array as () => Screenshot[], default: () => [] },
    titleSegments: { type: Array as () => TitleSegment[], default: () => [] },
    // Eventi veri del bucket "Finestra attiva" per lo stesso intervallo
    // — usati SOLO per riempire i buchi della barra cronologica nella
    // vista a schermo intero (spiega cosa occupava un buco), MAI per
    // etichettare le miniature/i gruppi: quelli restano "Sconosciuto"
    // nei buchi, guardano solo titleSegments. Bug reale segnalato
    // dall'utente: prima finivano uniti anche lì, e un blocco "Zen
    // Browser" mostrava "Claude" come se fosse una scheda del browser.
    finestreReali: { type: Array as () => FinestraReale[], default: () => [] },
    displayName: { type: String, required: true },
  },
  data() {
    return {
      // Default "Per finestra" — richiesta esplicita dell'utente.
      activeTab: 'perFinestra' as 'cronologica' | 'perFinestra',
      // null = nessun filtro, mostra tutti i titoli — un click su un
      // chip già attivo lo toglie di nuovo (vedi toggleFiltro()).
      filtroTitolo: null as string | null,
      // Titolo mostrato nel pannello della vista "Per finestra" —
      // inizializzato al primo titolo del blocco quando la lista dei
      // gruppi cambia (vedi watch sotto), non deve mai restare vuoto se
      // ci sono foto.
      finestraSelezionata: '',
      // Indice dentro flatScreenshots (TUTTO il blocco, non solo la vista/
      // il filtro attivo al momento del click) — richiesta esplicita col
      // redesign della barra cronologica: quella barra mostra sempre
      // l'intero blocco, quindi Prec/Succ e lo scrubbing devono poter
      // raggiungere qualunque foto, non solo quelle attualmente filtrate.
      lightboxIndex: null as number | null,
      // True per tutta la durata di un trascinamento dell'indicatore
      // della barra cronologica — mousemove/up agganciati su `window`
      // stesso motivo del trascinamento dell'immagine zoomata sotto.
      isScrubbing: false,
      // Wheel-zoom on the open image (explicit request) — a plain CSS
      // scale, reset back to 1 on every navigation so zooming into one
      // screenshot doesn't carry over to the next.
      imageZoom: 1,
      // Pan offset while zoomed in (explicit follow-up request — once
      // zoomed, the interesting detail is often outside the visible
      // area, so it needs to be draggable). Raw pixels, reset alongside
      // imageZoom on every navigation.
      panX: 0,
      panY: 0,
      isDragging: false,
      dragStartMouse: { x: 0, y: 0 },
      dragStartPan: { x: 0, y: 0 },
      // Filename -> true per ogni miniatura il cui file non esiste più
      // (es. eliminato dalla pulizia automatica per anzianità) — nascosta
      // invece di mostrare l'icona di immagine rotta del browser.
      brokenFilenames: {} as Record<string, boolean>,
    };
  },
  computed: {
    // Every screenshot resolved to the title active at its own moment —
    // 'Sconosciuto' when there's no matching segment at all (e.g. the
    // block isn't from the Generale lane, so titleSegments is empty).
    flatScreenshots(): ScreenshotConTitolo[] {
      return this.screenshots.map(s => ({ ...s, title: this.titleForTimestamp(s.timestamp) }));
    },
    // Grouped in order of each title's first screenshot — not
    // alphabetical or by total count — so scanning top to bottom roughly
    // follows the order things happened in. Usato sia dai chip filtro
    // che dalla sidebar "Per finestra".
    groups(): { title: string; shots: ScreenshotConTitolo[] }[] {
      const order: string[] = [];
      const byTitle = new Map<string, ScreenshotConTitolo[]>();
      for (const s of this.flatScreenshots) {
        if (!byTitle.has(s.title)) {
          byTitle.set(s.title, []);
          order.push(s.title);
        }
        byTitle.get(s.title)!.push(s);
      }
      return order.map(title => ({ title, shots: byTitle.get(title)! }));
    },
    // Riga di riepilogo sotto il titolo: intervallo orario dell'intero
    // blocco, conteggio foto totale, conteggio finestre distinte.
    metaText(): string {
      const range = `${this.flatScreenshots[0].time} – ${this.flatScreenshots[this.flatScreenshots.length - 1].time}`;
      const chiave =
        this.groups.length === 1 ? 'home.screenshotGallery.metaWindow' : 'home.screenshotGallery.metaWindows';
      return this.$t(chiave, { range, count: this.flatScreenshots.length, windows: this.groups.length }) as string;
    },
    // Vista Cronologica: tutti gli scatti in ordine di tempo, filtrati al
    // solo titolo scelto tramite un chip (se nessuno è scelto, tutti).
    screenshotsVisibili(): ScreenshotConTitolo[] {
      if (!this.filtroTitolo) return this.flatScreenshots;
      return this.flatScreenshots.filter(s => s.title === this.filtroTitolo);
    },
    gruppoSelezionato(): { title: string; shots: ScreenshotConTitolo[] } | null {
      return this.groups.find(g => g.title === this.finestraSelezionata) || this.groups[0] || null;
    },
    currentShot(): ScreenshotConTitolo | null {
      return this.lightboxIndex !== null ? this.flatScreenshots[this.lightboxIndex] : null;
    },
    // Estremi temporali dell'intero blocco — base per posizionare tick,
    // segmenti e indicatore sulla barra cronologica della lightbox.
    // flatScreenshots è già in ordine cronologico (vedi loadScreenshots
    // in TimelineBlockDetailModal.vue), quindi primo/ultimo bastano.
    timelineStart(): moment.Moment | null {
      return this.flatScreenshots[0]?.timestamp ?? null;
    },
    timelineEnd(): moment.Moment | null {
      return this.flatScreenshots[this.flatScreenshots.length - 1]?.timestamp ?? null;
    },
    // Tacche orarie a intervalli "puliti" (1/2/5/10/15/30/60 minuti),
    // scelto in modo che ce ne siano circa 6-12 lungo tutta la barra —
    // né troppo fitte né troppo rade indipendentemente da quanto dura il
    // blocco.
    timelineTicks(): { label: string; pct: number }[] {
      if (!this.timelineStart || !this.timelineEnd) return [];
      const intervallo = this.intervalloTacche();
      const tacche: { label: string; pct: number }[] = [];
      let cursore = this.timelineStart.clone();
      while (!cursore.isAfter(this.timelineEnd)) {
        tacche.push({ label: cursore.format('HH:mm'), pct: this.pctPerMomento(cursore) });
        cursore = cursore.clone().add(intervallo, 'minutes');
      }
      return tacche;
    },
    // Tacche minori senza etichetta, un quarto dell'intervallo sopra —
    // richiesta esplicita: rendere la barra più leggibile anche tra
    // un'etichetta oraria e l'altra, non solo ai punti etichettati.
    timelineMinorTicks(): number[] {
      if (!this.timelineStart || !this.timelineEnd) return [];
      const intervalloMinore = Math.max(1, Math.round(this.intervalloTacche() / 4));
      const tacche: number[] = [];
      let cursore = this.timelineStart.clone();
      while (!cursore.isAfter(this.timelineEnd)) {
        tacche.push(this.pctPerMomento(cursore));
        cursore = cursore.clone().add(intervalloMinore, 'minutes');
      }
      return tacche;
    },
    // Un segmento colorato per ogni occorrenza reale di titolo (stesso
    // array titleSegments passato dal chiamante) — a differenza dei chip/
    // della sidebar (raggruppati per titolo), qui ogni occorrenza resta
    // separata: la barra deve rispecchiare fedelmente la cronologia vera,
    // non un riepilogo.
    // titleSegments copre solo le occorrenze REALI dell'app di questo
    // blocco — un buco tra due occorrenze (es. un'altra finestra passata
    // in primo piano nel mezzo, che sposta il blocco Timeline stesso in
    // due tronconi) restava uno spazio vuoto sulla barra, senza nessun
    // segmento. Richiesta esplicita dell'utente: riempire ogni buco con
    // un segmento "Sconosciuto" (stessa etichetta già usata per le
    // miniature senza un titolo corrispondente), così la barra copre
    // SEMPRE l'intero intervallo, mai un vuoto.
    segmentiCompleti(): TitleSegment[] {
      const start = this.timelineStart;
      const end = this.timelineEnd;
      if (!start || !end) return [];
      // Ritaglia (o scarta del tutto) ogni segmento fuori da
      // [timelineStart, timelineEnd] — bug reale trovato in verifica: un
      // segmento sintetico (finestra reale usata per riempire un buco,
      // vedi TimelineBlockDetailModal.vue) poteva estendersi oltre
      // l'ultimo screenshot vero del blocco, e finiva schiacciato a un
      // filo pressoché invisibile sul bordo destro della barra invece di
      // sparire del tutto — "tutto quello che viene dopo l'ultima
      // attività non deve essere mostrato", richiesta esplicita.
      const ritagliati = this.titleSegments
        .map(seg => ({ title: seg.title, start: moment.max(seg.start, start), end: moment.min(seg.end, end) }))
        .filter(seg => seg.start.isBefore(seg.end))
        .sort((a, b) => a.start.diff(b.start));

      const sconosciuto = this.$t('home.screenshotGallery.unknown') as string;
      const conBuchiRiempiti: TitleSegment[] = [];
      // Riempie un buco [da, a) con la vera finestra a fuoco in quel
      // momento (bucket "Finestra attiva", vedi finestreReali) — SOLO
      // per questa barra cronologica, mai per le miniature/i gruppi
      // (quelli restano "Sconosciuto" nei buchi). "Sconosciuto" resta
      // comunque per gli istanti in cui neanche quel watcher ha dati.
      const riempiBuco = (da: moment.Moment, a: moment.Moment) => {
        const reali = this.finestreReali
          .map(f => ({ title: f.app, start: moment.max(f.start, da), end: moment.min(f.end, a) }))
          .filter(f => f.start.isBefore(f.end))
          .sort((x, y) => x.start.diff(y.start));
        let cursoreBuco = da.clone();
        for (const f of reali) {
          if (f.start.isAfter(cursoreBuco)) {
            conBuchiRiempiti.push({ title: sconosciuto, start: cursoreBuco.clone(), end: f.start.clone() });
          }
          conBuchiRiempiti.push(f);
          if (f.end.isAfter(cursoreBuco)) cursoreBuco = f.end.clone();
        }
        if (cursoreBuco.isBefore(a)) {
          conBuchiRiempiti.push({ title: sconosciuto, start: cursoreBuco.clone(), end: a.clone() });
        }
      };
      let cursore = start.clone();
      for (const seg of ritagliati) {
        if (seg.start.isAfter(cursore)) {
          riempiBuco(cursore, seg.start);
        }
        conBuchiRiempiti.push(seg);
        if (seg.end.isAfter(cursore)) cursore = seg.end.clone();
      }
      if (cursore.isBefore(end)) {
        riempiBuco(cursore, end);
      }

      // Unisce nel segmento precedente ogni attività più breve dell'1%
      // della durata totale — richiesta esplicita: un'alternanza rapida
      // di finestre (pochi secondi ciascuna) produceva tante striscioline
      // illeggibili invece di un'unica barra leggibile.
      const SOGLIA_MINIMA = 0.01;
      const totale = end.diff(start) || 1;
      const uniti: TitleSegment[] = [];
      for (const seg of conBuchiRiempiti) {
        const durataRelativa = seg.end.diff(seg.start) / totale;
        if (durataRelativa < SOGLIA_MINIMA && uniti.length) {
          uniti[uniti.length - 1].end = seg.end;
        } else {
          uniti.push(seg);
        }
      }
      return uniti;
    },
    timelineSegments(): { title: string; leftPct: number; widthPct: number; color: string }[] {
      if (!this.timelineStart || !this.timelineEnd) return [];
      return this.segmentiCompleti.map(seg => {
        const left = this.pctPerMomento(seg.start);
        const right = this.pctPerMomento(seg.end);
        return {
          title: seg.title,
          leftPct: left,
          widthPct: Math.max(right - left, 0.4),
          color: this.coloreForTitle(seg.title),
        };
      });
    },
    // Indice (in titleSegments/timelineSegments) del segmento che
    // contiene davvero la foto aperta — evidenziato con un bordo più
    // marcato sulla barra.
    currentSegmentIndex(): number {
      if (!this.currentShot) return -1;
      const shot = this.currentShot;
      return this.segmentiCompleti.findIndex(
        seg => !shot.timestamp.isBefore(seg.start) && shot.timestamp.isBefore(seg.end)
      );
    },
    currentPct(): number {
      if (!this.currentShot) return 0;
      return this.pctPerMomento(this.currentShot.timestamp);
    },
    // Solo ore:minuti per il badge che segue il trascinamento — i
    // secondi (già visibili per esteso nella barra info in alto) sono
    // un dettaglio di troppo in un'etichetta pensata per essere letta
    // al volo mentre si trascina.
    currentTimeBreve(): string {
      return this.currentShot ? this.currentShot.timestamp.format('HH:mm') : '';
    },
  },
  watch: {
    // Ogni volta che il blocco cambia (nuova apertura della galleria per
    // un blocco diverso), riparte dal primo titolo e senza filtro attivo
    // — mai lasciare la vista in uno stato ereditato dal blocco precedente.
    groups: {
      immediate: true,
      handler(nuovi: { title: string }[]) {
        if (!nuovi.some(g => g.title === this.finestraSelezionata)) {
          this.finestraSelezionata = nuovi[0]?.title || '';
        }
      },
    },
  },
  mounted() {
    window.addEventListener('keydown', this.onKeydown);
  },
  beforeDestroy() {
    window.removeEventListener('keydown', this.onKeydown);
    // Defensive — in case the component is torn down mid-drag.
    window.removeEventListener('mousemove', this.onDragMove);
    window.removeEventListener('mouseup', this.onDragEnd);
    window.removeEventListener('mousemove', this.onScrubMouseMove);
    window.removeEventListener('mouseup', this.onScrubMouseUp);
  },
  methods: {
    titleForTimestamp(ts: moment.Moment): string {
      const seg = this.titleSegments.find(
        (s: TitleSegment) => !ts.isBefore(s.start) && ts.isBefore(s.end)
      );
      return seg ? seg.title : (this.$t('home.screenshotGallery.unknown') as string);
    },
    // Stesso colore finché il titolo resta lo stesso, il più possibile
    // diverso dagli altri titoli già assegnati in QUESTA galleria — vedi
    // dailyColorPalette.ts (stesso meccanismo già usato per i blocchi
    // Timeline di corsie senza icona propria, es. VPN/Claude/VS Code).
    // laneKey scoped a questa singola apertura (nome + primo scatto) così
    // gallerie diverse non condividono/esauriscono la stessa mappa.
    coloreForTitle(title: string): string {
      const laneKey = `screenshot-gallery:${this.displayName}:${this.screenshots[0]?.filename || ''}`;
      return colorePerGiorno(laneKey, title);
    },
    rangeForGroup(g: { shots: ScreenshotConTitolo[] }): string {
      return `${g.shots[0].time} – ${g.shots[g.shots.length - 1].time}`;
    },
    toggleFiltro(title: string) {
      this.filtroTitolo = this.filtroTitolo === title ? null : title;
    },
    resetZoom() {
      this.imageZoom = 1;
      this.panX = 0;
      this.panY = 0;
    },
    // Apre sempre sull'intero blocco (flatScreenshots), non solo sulla
    // vista/il filtro da cui si è cliccato — richiesta esplicita: la
    // barra cronologica in fondo mostra tutto il blocco, quindi
    // Prec/Succ e lo scrubbing devono poter raggiungere qualunque foto.
    openLightbox(shot: ScreenshotConTitolo) {
      this.lightboxIndex = this.flatScreenshots.findIndex(s => s.filename === shot.filename);
      this.resetZoom();
    },
    closeLightbox() {
      this.lightboxIndex = null;
      this.resetZoom();
    },
    prev() {
      if (this.lightboxIndex !== null && this.lightboxIndex > 0) {
        this.lightboxIndex--;
        this.resetZoom();
      }
    },
    next() {
      if (this.lightboxIndex !== null && this.lightboxIndex < this.flatScreenshots.length - 1) {
        this.lightboxIndex++;
        this.resetZoom();
      }
    },
    // Intervallo "pulito" (1/2/5/10/15/30/60... minuti) scelto in modo
    // che ce ne siano circa 6-12 lungo tutta la barra — condiviso tra
    // le tacche etichettate e quelle minori (un quarto di questo).
    intervalloTacche(): number {
      if (!this.timelineStart || !this.timelineEnd) return 1;
      const totalMinuti = Math.max(this.timelineEnd.diff(this.timelineStart, 'minutes'), 1);
      const candidati = [1, 2, 5, 10, 15, 30, 60, 120, 240];
      return candidati.find(c => totalMinuti / c <= 12) ?? candidati[candidati.length - 1];
    },
    pctPerMomento(m: moment.Moment): number {
      if (!this.timelineStart || !this.timelineEnd) return 0;
      const totale = this.timelineEnd.diff(this.timelineStart) || 1;
      const pct = (m.diff(this.timelineStart) / totale) * 100;
      return Math.min(100, Math.max(0, pct));
    },
    // Dalla percentuale lungo la barra (posizione del mouse durante lo
    // scrubbing) trova la foto più vicina nel tempo in TUTTO il blocco e
    // ci salta direttamente — non serve un indice esatto, la foto più
    // vicina è sempre quella "giusta" da mostrare mentre si trascina.
    saltaAPercentuale(pct: number) {
      if (!this.timelineStart || !this.timelineEnd || !this.flatScreenshots.length) return;
      const totale = this.timelineEnd.diff(this.timelineStart);
      const target = this.timelineStart.clone().add((totale * pct) / 100, 'milliseconds');
      let indiceMigliore = 0;
      let diffMigliore = Infinity;
      this.flatScreenshots.forEach((s, i) => {
        const diff = Math.abs(s.timestamp.diff(target));
        if (diff < diffMigliore) {
          diffMigliore = diff;
          indiceMigliore = i;
        }
      });
      this.lightboxIndex = indiceMigliore;
      this.resetZoom();
    },
    percentualeDaEvento(evt: MouseEvent): number {
      const track = this.$refs.scrubTrack as HTMLElement | undefined;
      if (!track) return 0;
      const rect = track.getBoundingClientRect();
      return Math.min(100, Math.max(0, ((evt.clientX - rect.left) / rect.width) * 100));
    },
    onScrubMouseDown(evt: MouseEvent) {
      // Senza questo, trascinare il mouse sopra la barra selezionava il
      // testo dei segmenti/etichette vicine (comportamento di default
      // del browser su un mousedown+move) — bug reale segnalato
      // dall'utente, il trascinamento in sé funzionava già.
      evt.preventDefault();
      this.isScrubbing = true;
      this.saltaAPercentuale(this.percentualeDaEvento(evt));
      window.addEventListener('mousemove', this.onScrubMouseMove);
      window.addEventListener('mouseup', this.onScrubMouseUp);
    },
    onScrubMouseMove(evt: MouseEvent) {
      if (!this.isScrubbing) return;
      this.saltaAPercentuale(this.percentualeDaEvento(evt));
    },
    onScrubMouseUp() {
      this.isScrubbing = false;
      window.removeEventListener('mousemove', this.onScrubMouseMove);
      window.removeEventListener('mouseup', this.onScrubMouseUp);
    },
    // Richiesta esplicita: la rotellina SOPRA la barra avanza/arretra di
    // una foto alla volta (non uno scrub continuo come il trascinamento).
    onScrubWheel(evt: WheelEvent) {
      evt.preventDefault();
      if (evt.deltaY > 0) this.next();
      else this.prev();
    },
    // Explicit request: scrolling the wheel while hovering the
    // full-size image zooms it — scoped to the <img> itself (not the
    // whole lightbox) via @wheel there, and preventDefault()+stop()
    // stops it from bubbling up to anything that might otherwise treat
    // the scroll as page/Timeline navigation.
    onImageWheel(evt: WheelEvent) {
      evt.preventDefault();
      evt.stopPropagation();
      const ZOOM_STEP = 0.15;
      const MIN_ZOOM = 1;
      const MAX_ZOOM = 5;
      const direction = evt.deltaY > 0 ? -1 : 1;
      this.imageZoom = Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, this.imageZoom + direction * ZOOM_STEP));
      // Back at 1x there's nothing to pan into — drop any leftover
      // offset instead of leaving the image visibly shifted once it
      // snaps back to its normal (unscaled) size.
      if (this.imageZoom === MIN_ZOOM) {
        this.panX = 0;
        this.panY = 0;
      }
    },
    // Explicit follow-up request: once zoomed in, drag the image around
    // to reach the parts that scaled outside the visible area. Only
    // starts a drag when actually zoomed in — at 1x the image can't
    // overflow its box, so there'd be nowhere to pan to. mousemove/up
    // are bound on `window` rather than the image itself so the drag
    // keeps tracking even if the cursor slips past the image's edges
    // mid-drag (very likely once zoomed in, since the image can extend
    // beyond the lightbox's own bounds).
    onImageMouseDown(evt: MouseEvent) {
      if (this.imageZoom <= 1) return;
      this.isDragging = true;
      this.dragStartMouse = { x: evt.clientX, y: evt.clientY };
      this.dragStartPan = { x: this.panX, y: this.panY };
      window.addEventListener('mousemove', this.onDragMove);
      window.addEventListener('mouseup', this.onDragEnd);
    },
    onDragMove(evt: MouseEvent) {
      this.panX = this.dragStartPan.x + (evt.clientX - this.dragStartMouse.x);
      this.panY = this.dragStartPan.y + (evt.clientY - this.dragStartMouse.y);
    },
    onDragEnd() {
      this.isDragging = false;
      window.removeEventListener('mousemove', this.onDragMove);
      window.removeEventListener('mouseup', this.onDragEnd);
    },
    // Explicit request was for on-screen Prev/Next buttons — arrow-key
    // support is the same navigation via a second, expected input path
    // for a lightbox, not a separate feature.
    onKeydown(evt: KeyboardEvent) {
      if (this.lightboxIndex === null) {
        // Nessuna lightbox aperta — Esc chiude l'intera galleria (il
        // footer lo promette esplicitamente, "Esc per chiudere").
        if (evt.key === 'Escape') this.$emit('close');
        return;
      }
      if (evt.key === 'ArrowLeft') this.prev();
      else if (evt.key === 'ArrowRight') this.next();
      else if (evt.key === 'Escape') this.closeLightbox();
    },
  },
};
</script>

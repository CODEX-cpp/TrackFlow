<template lang="pug">
div.search-wrap
  transition(name="search-pop")
    div.search-form(v-if="open")
      input.search-input(
        :value="value"
        @input="$emit('input', $event.target.value)"
        :placeholder="placeholder"
        ref="searchInput"
      )
      div.popover-backdrop(v-if="showSuggestions" @click="$emit('close')")
      div.search-suggestions(v-if="showSuggestions")
        div.search-suggestion-empty(v-if="suggestions.length === 0") {{ emptyLabel }}
        div.search-suggestion-row(
          v-for="item in suggestions"
          :key="item.id"
          @click="$emit('select', item)"
        )
          div.search-suggestion-label {{ item.label }}
          div.search-suggestion-sublabel(v-if="item.sublabel") {{ item.sublabel }}
</template>

<style lang="scss" scoped>
@import '../style/theme.css';

.search-wrap {
  position: relative;
}

.search-form {
  display: flex;
}

.search-input {
  padding: 8px 14px;
  border-radius: var(--radius-md);
  background-color: var(--color-surface2);
  font-size: var(--font-size-sm);
  color: var(--color-text);
  border: 1px solid var(--color-accent1);
  width: 160px;
  font-family: inherit;

  &:focus {
    outline: none;
    filter: brightness(1.1);
  }
}

.popover-backdrop {
  position: fixed;
  inset: 0;
  z-index: 39;
}

.search-suggestions {
  position: absolute;
  top: calc(100% + 8px);
  right: 0;
  z-index: 40;
  width: 220px;
  max-height: 260px;
  overflow-y: auto;
  background-color: var(--color-bg-elev);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-elevated);
  padding: 6px;
}

.search-suggestion-row {
  padding: 8px 10px;
  border-radius: var(--radius-sm);
  cursor: pointer;

  &:hover {
    background-color: var(--color-surface2);
  }
}

.search-suggestion-label {
  font-size: var(--font-size-sm);
  color: var(--color-text-dim);
}

.search-suggestion-row:hover .search-suggestion-label {
  color: var(--color-text);
}

.search-suggestion-sublabel {
  font-size: var(--font-size-xs);
  color: var(--color-text-faint);
  margin-top: 1px;
}

.search-suggestion-empty {
  padding: 8px 10px;
  font-size: var(--font-size-sm);
  color: var(--color-text-faint);
}

// Small appearance/disappearance animation for the search form —
// scale + fade instead of a hard pop.
.search-pop-enter-active,
.search-pop-leave-active {
  transition: opacity 0.15s ease, transform 0.15s ease;
  transform-origin: top right;
}

.search-pop-enter,
.search-pop-leave-to {
  opacity: 0;
  transform: scale(0.96) translateY(-4px);
}
</style>

<script lang="ts">
// Generic search box + suggestions dropdown, used in the topbar.
// Deliberately knows nothing about projects (or any other data) —
// the caller decides what counts as a match and what happens when a
// suggestion is picked, this component only handles the input field,
// the dropdown list, the outside-click-to-close backdrop, and the
// open/close animation.
export default {
  name: 'SearchDropdown',
  props: {
    open: { type: Boolean, required: true },
    value: { type: String, default: '' },
    suggestions: {
      type: Array as () => { id: string; label: string; sublabel?: string }[],
      required: true,
    },
    showSuggestions: { type: Boolean, required: true },
    // No defaults for the user-facing strings on purpose — a silent
    // English fallback on an Italian page is exactly the kind of bug
    // that's easy to miss (it happened once already with ConfirmModal).
    placeholder: { type: String, required: true },
    emptyLabel: { type: String, required: true },
  },
  watch: {
    open(isOpen: boolean) {
      if (isOpen) {
        this.$nextTick(() => (this.$refs.searchInput as HTMLInputElement)?.focus());
      }
    },
  },
};
</script>

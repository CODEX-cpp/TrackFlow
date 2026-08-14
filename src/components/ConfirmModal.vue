<template lang="pug">
div
  div.modal-backdrop(@click="$emit('cancel')")
  div.edit-modal
    div.edit-modal-title {{ title }}

    slot

    div.edit-modal-actions
      div.pill-btn-ghost(@click="$emit('cancel')") {{ cancelLabel }}
      div.pill-btn-danger(@click="$emit('confirm')") {{ confirmLabel }}
</template>

<style lang="scss" scoped>
@import '../style/theme.css';
@import '../style/modals.css';
</style>

<script lang="ts">
// Generic centered confirm/cancel popup, styled like the rest of the
// app instead of the browser's native confirm() — which turned out
// to be unreliable in real-world testing (see Progetti.vue history).
// The caller supplies the body via the default slot, so this stays
// usable for any "are you sure?" prompt, not just project actions.
export default {
  name: 'ConfirmModal',
  props: {
    title: { type: String, required: true },
    confirmLabel: { type: String, default: 'Confirm' },
    cancelLabel: { type: String, default: 'Cancel' },
  },
};
</script>

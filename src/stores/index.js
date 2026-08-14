// Initializes Pinia, does not import stores

import { createPinia, setActivePinia, PiniaVuePlugin } from 'pinia';
import Vue from 'vue';

Vue.use(PiniaVuePlugin); // Only needed for Vue 2

const rootStore = createPinia();
// Router's mixin registers before Pinia's (route.js's `Vue.use(VueRouter)`
// runs at import-eval time, ahead of this module's Vue.use above) and
// starts the initial navigation synchronously during the root instance's
// beforeCreate — before Pinia's own beforeCreate would normally activate
// this instance. Any store used from a route guard on the very first
// navigation (e.g. route.js's `/` beforeEnter) would otherwise hit
// getActivePinia() with nothing active yet. Setting it eagerly here,
// independent of Vue's mixin ordering, closes that race.
setActivePinia(rootStore);
export default rootStore;

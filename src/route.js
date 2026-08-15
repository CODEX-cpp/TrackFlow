import Vue from 'vue';
import VueRouter from 'vue-router';
import { useBucketsStore } from '~/stores/buckets';

// The old Bootstrap "Activity" page (Activity.vue/ActivityView.vue —
// toolbar with day/week/month/year buttons, Summary/Window/Browser/
// Editor tabs, view management) has been removed (explicit request):
// the new themed Home (App.vue's home-timeline-section/home-modules-
// section/active-projects-section, rendered directly, no router-view)
// replaces it entirely, and the old period-switching/multi-view
// concept was deliberately never carried over. The route below is
// kept ONLY so /activity/... URLs still resolve — Sidebar.vue and
// Topbar.vue's day navigation both construct paths in this exact
// shape (down to the trailing "view/"), and App.vue never actually
// renders this route's component on the Home page (isHomePage skips
// router-view entirely there), so what it points to doesn't matter as
// long as it's a valid component.
// NOTE: TrackFlow is single-device (mono-client, explicit design
// decision) — the route used to carry a :host param, but the host is
// now resolved automatically wherever needed (bucketsStore.host) and
// never appears in the URL.
const HomeRoutePlaceholder = { render: h => h('div') };

const Buckets = () => import('./views/Buckets.vue');
const Bucket = () => import('./views/Bucket.vue');
const Settings = () => import('./views/settings/Settings.vue');
const Progetti = () => import('./views/Progetti.vue');

Vue.use(VueRouter);

const router = new VueRouter({
  routes: [
    {
      // No static redirect target: fresh installs with no buckets yet
      // should land on Raw Data (the "install a watcher" hint) instead of
      // an empty Timeline. That needs the buckets store loaded, which
      // redirect() can't await (it must return synchronously) — beforeEnter
      // can, so we check here instead.
      path: '/',
      component: HomeRoutePlaceholder,
      beforeEnter: async (to, from, next) => {
        try {
          const bucketsStore = useBucketsStore();
          await bucketsStore.ensureLoaded();
          next(bucketsStore.buckets.length > 0 ? '/activity/view/' : '/buckets');
        } catch (e) {
          next('/buckets');
        }
      },
    },
    {
      path: '/activity/:periodLength?/:date?',
      component: HomeRoutePlaceholder,
      props: true,
      children: [
        {
          path: 'view/:view_id?',
          meta: { subview: 'view' },
          name: 'activity-view',
          component: HomeRoutePlaceholder,
          props: true,
        },
        // Unspecified should redirect to summary view is the summary view
        // (needs to be last since otherwise it'll always match first)
        {
          path: '',
          redirect: 'view/',
        },
      ],
    },
    { path: '/buckets', component: Buckets },
    { path: '/buckets/:id', component: Bucket, props: true },
    // bareLayout (same as /stopwatch below) skips the old Bootstrap
    // .container/.aw-container wrapper App.vue still renders pages
    // through by default — that wrapper is what dark.css's #0f131a/
    // #1a1d24 body/.aw-container overrides were painting blue-navy
    // behind Settings, on top of an otherwise fully re-themed page
    // (explicit bug report). Settings.vue now themes itself entirely
    // via theme.css, same as Progetti already did.
    { path: '/settings', component: Settings, meta: { bareLayout: true } },
    // :group lets the active settings panel survive reloads / be linkable.
    // New groups added in Settings.vue should also be added here.
    {
      path: '/settings/:group(general|home|appearance|categorization|integrations|privacy|developer|notifications|about)',
      component: Settings,
      props: true,
      meta: { bareLayout: true },
    },
    { path: '/stopwatch', component: Progetti, meta: { bareLayout: true } },
  ],
});

export default router;

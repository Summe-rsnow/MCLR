import type { RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = [
  {
    path: '/comp',
    redirect: '/comp/mod',
  },
  {
    path: '/comp/mod',
    name: 'comp-mod',
    component: () => import('./CompBrowserPage.vue'),
    meta: {
      nav: {
        titleKey: 'comp.nav',
        icon: 'i-lucide-store',
        order: 35,
        group: 'discover',
      },
    },
  },
  {
    path: '/comp/modpack',
    name: 'comp-modpack',
    component: () => import('./CompBrowserPage.vue'),
  },
  {
    path: '/comp/resourcepack',
    name: 'comp-resourcepack',
    component: () => import('./CompBrowserPage.vue'),
  },
  {
    path: '/comp/shader',
    name: 'comp-shader',
    component: () => import('./CompBrowserPage.vue'),
  },
  {
    path: '/comp/datapack',
    name: 'comp-datapack',
    component: () => import('./CompBrowserPage.vue'),
  },
  {
    path: '/comp/world',
    name: 'comp-world',
    component: () => import('./CompBrowserPage.vue'),
  },
  {
    path: '/comp/favorites',
    name: 'comp-favorites',
    component: () => import('./CompBrowserPage.vue'),
  },
  {
    path: '/comp/project/:platform/:id',
    name: 'comp-project-detail',
    component: () => import('./CompDetailPage.vue'),
  },
]

export default routes

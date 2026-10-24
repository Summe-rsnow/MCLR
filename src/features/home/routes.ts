import type { RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = [
  {
    path: '/home',
    name: 'home',
    component: () => import('./HomePage.vue'),
    meta: {
      nav: {
        titleKey: 'home.nav',
        icon: 'i-lucide-house',
        order: 1,
        group: 'start',
      },
    },
  },
  {
    path: '/help',
    name: 'help',
    redirect: { path: '/home', query: { tab: 'help' } },
  },
]

export default routes

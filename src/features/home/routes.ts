import type { RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = [
  {
    path: '/home',
    name: 'home',
    component: () => import('./HomePage.vue'),
    meta: {
      nav: {
        titleKey: 'home.nav',
        icon: 'i-lucide-home',
        order: 1,
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

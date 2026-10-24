import type { RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = [
  {
    path: '/accounts',
    name: 'accounts',
    component: () => import('./AccountsPage.vue'),
    meta: {
      nav: {
        titleKey: 'accounts.nav',
        icon: 'i-lucide-user-round',
        order: 20,
        group: 'game',
      },
    },
  },
]

export default routes

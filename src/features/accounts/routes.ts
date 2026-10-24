import type { RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = [
  {
    path: '/accounts',
    name: 'accounts',
    component: () => import('./AccountsPage.vue'),
    meta: {
      nav: {
        titleKey: 'accounts.nav',
        icon: 'i-lucide-user-circle',
        order: 20,
      },
    },
  },
]

export default routes

import type { RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = [
  {
    path: '/logs',
    name: 'logs',
    component: () => import('./LogsPage.vue'),
    meta: {
      nav: {
        titleKey: 'logs.nav',
        icon: 'i-lucide-terminal',
        order: 80,
      },
    },
  },
]

export default routes

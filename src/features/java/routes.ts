import type { RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = [
  {
    path: '/java',
    name: 'java',
    component: () => import('./JavaPage.vue'),
    meta: {
      nav: {
        titleKey: 'java.nav',
        icon: 'i-lucide-coffee',
        order: 25,
        group: 'game',
      },
    },
  },
]

export default routes

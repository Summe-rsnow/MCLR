import type { RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = [
  {
    path: '/instances',
    name: 'instances',
    component: () => import('./InstancesPage.vue'),
    meta: {
      nav: {
        titleKey: 'instances.nav',
        icon: 'i-lucide-boxes',
        order: 10,
        group: 'game',
      },
    },
  },
]

export default routes

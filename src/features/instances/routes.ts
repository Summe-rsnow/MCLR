import type { RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = [
  {
    path: '/instances',
    name: 'instances',
    component: () => import('./InstancesPage.vue'),
    meta: {
      nav: {
        titleKey: 'instances.nav',
        icon: 'i-lucide-layers',
        order: 10,
      },
    },
  },
]

export default routes

import type { RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = [
  {
    path: '/download',
    name: 'download',
    component: () => import('./InstallPage.vue'),
    meta: {
      nav: {
        titleKey: 'download.nav',
        icon: 'i-lucide-cloud-download',
        order: 30,
        group: 'discover',
      },
    },
  },
]

export default routes

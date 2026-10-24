import type { RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = [
  {
    path: '/download',
    name: 'download',
    component: () => import('./InstallPage.vue'),
    meta: {
      nav: {
        titleKey: 'download.nav',
        icon: 'i-lucide-download',
        order: 30,
      },
    },
  },
]

export default routes

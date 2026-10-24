import type { RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = [
  {
    path: '/dev/ui',
    name: 'dev-ui',
    component: () => import('./DevUiPage.vue'),
    meta: {
      nav: {
        titleKey: 'shell.nav.devUi',
        icon: 'i-lucide-palette',
        order: 999,
      },
    },
  },
]

export default routes

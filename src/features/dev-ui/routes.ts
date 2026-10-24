import type { RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = import.meta.env.DEV
  ? [
      {
        path: '/dev/ui',
        name: 'dev-ui',
        component: () => import('./DevUiPage.vue'),
        meta: {
          nav: {
            titleKey: 'shell.nav.devUi',
            icon: 'i-lucide-swatch-book',
            order: 999,
            group: 'dev',
          },
        },
      },
    ]
  : []

export default routes

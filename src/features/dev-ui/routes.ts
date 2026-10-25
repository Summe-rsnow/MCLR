import type { RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = import.meta.env.DEV
  ? [
      {
        path: '/dev/ui',
        name: 'dev-ui',
        component: () => import('./DevUiPage.vue'),
      },
    ]
  : []

export default routes

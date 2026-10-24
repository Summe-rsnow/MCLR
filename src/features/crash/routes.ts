import type { RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = [
  {
    path: '/crash',
    name: 'crash',
    component: () => import('./CrashPage.vue'),
    meta: {
      nav: null,
    },
  },
]

export default routes

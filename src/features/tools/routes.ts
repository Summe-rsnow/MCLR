import type { RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = [
  {
    path: '/tools',
    name: 'tools',
    component: () => import('./ToolsPage.vue'),
    meta: {
      nav: {
        titleKey: 'tools.nav',
        icon: 'i-lucide-wrench',
        order: 50,
      },
    },
  },
]

export default routes

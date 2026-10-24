import type { RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = [
  {
    path: '/tasks',
    name: 'tasks',
    component: () => import('./TasksPage.vue'),
    meta: {
      nav: {
        titleKey: 'tasks.nav',
        icon: 'i-lucide-list-checks',
        order: 90,
        group: 'system',
      },
    },
  },
]

export default routes

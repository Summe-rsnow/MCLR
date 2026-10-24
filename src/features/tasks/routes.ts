import type { RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = [
  {
    path: '/tasks',
    name: 'tasks',
    component: () => import('./TasksPage.vue'),
    meta: {
      nav: {
        titleKey: 'tasks.nav',
        icon: 'i-lucide-list-todo',
        order: 90,
      },
    },
  },
]

export default routes

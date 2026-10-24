import type { RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = [
  {
    path: '/launch',
    name: 'launch',
    component: () => import('./LaunchPage.vue'),
    meta: {
      nav: {
        titleKey: 'launch.nav',
        icon: 'i-lucide-play',
        order: 5,
      },
    },
  },
]

export default routes

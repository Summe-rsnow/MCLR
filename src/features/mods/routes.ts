import type { RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = [
  {
    path: '/mods',
    name: 'mods',
    component: () => import('./ModListPanel.vue'),
    meta: {
      nav: {
        titleKey: 'mods.nav',
        icon: 'i-lucide-puzzle',
        order: 25,
        group: 'game',
      },
    },
  },
]

export default routes

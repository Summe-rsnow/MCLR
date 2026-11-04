import type { RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = [
  {
    path: '/presets',
    name: 'presets',
    component: () => import('./PresetsPage.vue'),
    meta: {
      nav: {
        titleKey: 'presets.nav',
        icon: 'i-lucide-package-check',
        order: 36,
        group: 'discover',
      },
    },
  },
]

export default routes

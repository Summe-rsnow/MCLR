import type { RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = [
  {
    path: '/settings/appearance',
    name: 'settings-appearance',
    component: () => import('./SettingsAppearancePage.vue'),
    meta: {
      nav: {
        titleKey: 'settingsAppearance.nav',
        icon: 'i-lucide-palette',
        order: 65,
      },
    },
  },
]

export default routes

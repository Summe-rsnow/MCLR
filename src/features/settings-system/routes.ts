import type { RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = [
  {
    path: '/settings/system',
    name: 'settings-system',
    component: () => import('./SettingsSystemPage.vue'),
    meta: {
      nav: {
        titleKey: 'settingsSystem.nav',
        icon: 'i-lucide-settings',
        order: 75,
      },
    },
  },
]

export default routes

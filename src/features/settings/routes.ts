import type { RouteRecordRaw } from 'vue-router'
import SettingsLayout from './SettingsLayout.vue'

const routes: RouteRecordRaw[] = [
  {
    path: '/settings',
    name: 'settings',
    component: SettingsLayout,
    redirect: '/settings/appearance',
    meta: {
      nav: {
        titleKey: 'shell.nav.settings',
        icon: 'i-lucide-settings',
        order: 100,
        group: 'system',
      },
    },
    children: [
      {
        path: 'appearance',
        name: 'settings-appearance',
        component: () => import('../settings-appearance/SettingsAppearancePage.vue'),
      },
      {
        path: 'game',
        name: 'settings-game',
        component: () => import('../settings-game/SettingsGamePage.vue'),
      },
      {
        path: 'system',
        name: 'settings-system',
        component: () => import('../settings-system/SettingsSystemPage.vue'),
      },
    ],
  },
]
export default routes

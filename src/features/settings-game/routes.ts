import type { RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = [
  {
    path: '/settings/game',
    name: 'settings-game',
    component: () => import('./SettingsGamePage.vue'),
    meta: {
      nav: {
        titleKey: 'settingsGame.nav',
        icon: 'i-lucide-sliders-horizontal',
        order: 70,
      },
    },
  },
]

export default routes

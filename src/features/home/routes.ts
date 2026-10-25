import type { RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = [
  {
    path: '/home',
    name: 'home',
    redirect: '/launch',
  },
  {
    path: '/help',
    name: 'help',
    redirect: '/settings/about',
  },
]

export default routes

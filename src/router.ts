import { createRouter, createWebHashHistory, type RouteRecordRaw } from 'vue-router'

import type { NavGroupKey } from './features/shell/navGroups'
export type { NavGroupKey }
export interface NavMeta {
  titleKey: string
  icon: string
  order: number
  group?: NavGroupKey
}

const routeModules = import.meta.glob<{ default: RouteRecordRaw[] }>(
  './features/*/routes.ts',
  { eager: true },
)

export const featureRoutes: RouteRecordRaw[] = Object.values(routeModules).flatMap(
  (mod) => mod.default ?? [],
)

export const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    ...featureRoutes,
    {
      path: '/',
      name: 'root',
      redirect: '/launch',
    },
  ],
})

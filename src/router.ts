import { createRouter, createWebHashHistory, type RouteRecordRaw } from 'vue-router'

export interface NavMeta {
  titleKey: string
  icon: string
  order: number
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
    {
      path: '/',
      name: 'root',
      redirect: '/dev/ui',
    },
    ...featureRoutes,
  ],
})

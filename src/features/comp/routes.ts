import type { RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = [
  {
    path: '/comp/project/:platform/:id',
    name: 'comp-project-detail',
    component: () => import('./CompDetailPage.vue'),
  },
  {
    path: '/comp/:type?',
    name: 'comp',
    component: () => import('./CompBrowserPage.vue'),
    meta: {
      nav: {
        titleKey: 'comp.nav',
        icon: 'i-lucide-store',
        order: 35,
        group: 'discover',
      },
    },
  },
]

export default routes

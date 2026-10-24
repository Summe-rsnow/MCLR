import type { RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = [
  {
    path: '/instances/:id',
    name: 'instance-detail',
    component: () => import('./InstanceDetailPage.vue'),
    props: true,
  },
]

export default routes

import type { RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = [
  {
    path: '/download',
    name: 'download',
    component: () => import('./InstallPage.vue'),
    meta: {
      nav: {
        titleKey: 'download.nav',
        icon: 'i-lucide-download',
        order: 30,
      },
    },
  },
  {
    path: '/download/loader/:loader',
    name: 'download-loader',
    component: () => import('./LoaderPage.vue'),
  },
  {
    path: '/download/fabric',
    name: 'download-fabric',
    component: () => import('./LoaderPage.vue'),
    props: { loader: 'fabric' },
  },
  {
    path: '/download/forge',
    name: 'download-forge',
    component: () => import('./LoaderPage.vue'),
    props: { loader: 'forge' },
  },
  {
    path: '/download/neoforge',
    name: 'download-neoforge',
    component: () => import('./LoaderPage.vue'),
    props: { loader: 'neoForge' },
  },
  {
    path: '/download/optifine',
    name: 'download-optifine',
    component: () => import('./LoaderPage.vue'),
    props: { loader: 'optiFine' },
  },
  {
    path: '/download/legacyfabric',
    name: 'download-legacyfabric',
    component: () => import('./LoaderPage.vue'),
    props: { loader: 'legacyFabric' },
  },
  {
    path: '/download/liteloader',
    name: 'download-liteloader',
    component: () => import('./LoaderPage.vue'),
    props: { loader: 'liteLoader' },
  },
  {
    path: '/download/labymod',
    name: 'download-labymod',
    component: () => import('./LoaderPage.vue'),
    props: { loader: 'labyMod' },
  },
  {
    path: '/download/cleanroom',
    name: 'download-cleanroom',
    component: () => import('./LoaderPage.vue'),
    props: { loader: 'cleanroom' },
  },
]

export default routes

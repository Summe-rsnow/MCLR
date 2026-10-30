<script setup lang="ts">
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import MPageHeader from '../../ui/MPageHeader.vue'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()

interface SubNavItem {
  key: string
  titleKey: string
  icon: string
  to: string
  path: string
  tab?: string
}

interface NavGroup {
  key: string
  titleKey: string
  items: SubNavItem[]
}

const navGroups: NavGroup[] = [
  {
    key: 'personal',
    titleKey: 'settings.group.personal',
    items: [
      {
        key: 'appearance',
        titleKey: 'settings.nav.appearance',
        icon: 'i-lucide-palette',
        to: '/settings/appearance',
        path: '/settings/appearance',
      },
      {
        key: 'language',
        titleKey: 'settings.nav.language',
        icon: 'i-lucide-languages',
        to: '/settings/system?tab=language',
        path: '/settings/system',
        tab: 'language',
      },
    ],
  },
  {
    key: 'game',
    titleKey: 'settings.group.game',
    items: [
      {
        key: 'launch',
        titleKey: 'settings.nav.launch',
        icon: 'i-lucide-rocket',
        to: '/settings/game?tab=launch',
        path: '/settings/game',
        tab: 'launch',
      },
      {
        key: 'gameManage',
        titleKey: 'settings.nav.gameManage',
        icon: 'i-lucide-folder-cog',
        to: '/settings/system?tab=gameManage',
        path: '/settings/system',
        tab: 'gameManage',
      },
    ],
  },
  {
    key: 'launcher',
    titleKey: 'settings.group.launcher',
    items: [
      {
        key: 'network',
        titleKey: 'settings.nav.network',
        icon: 'i-lucide-network',
        to: '/settings/system?tab=network',
        path: '/settings/system',
        tab: 'network',
      },
      {
        key: 'system',
        titleKey: 'settings.nav.system',
        icon: 'i-lucide-stethoscope',
        to: '/settings/system?tab=system',
        path: '/settings/system',
        tab: 'system',
      },
      {
        key: 'update',
        titleKey: 'settings.nav.update',
        icon: 'i-lucide-refresh-cw',
        to: '/settings/system?tab=update',
        path: '/settings/system',
        tab: 'update',
      },
    ],
  },
  {
    key: 'other',
    titleKey: 'settings.group.other',
    items: [
      {
        key: 'about',
        titleKey: 'settings.nav.aboutHelp',
        icon: 'i-lucide-circle-question-mark',
        to: '/settings/about',
        path: '/settings/about',
      },
      {
        key: 'sponsor',
        titleKey: 'settings.nav.sponsor',
        icon: 'i-lucide-heart-handshake',
        to: '/settings/sponsor',
        path: '/settings/sponsor',
      },
    ],
  },
]

function isActive(item: SubNavItem): boolean {
  if (!route) return false
  if (item.path !== route.path) return false
  if (!item.tab) return true

  const currentTab =
    (route.query?.tab as string) ||
    (route.path === '/settings/game'
      ? 'launch'
      : route.path === '/settings/system'
        ? 'gameManage'
        : '')
  return currentTab === item.tab
}

function navigate(item: SubNavItem) {
  router.push(item.to)
}
</script>

<template>
  <div class="m-page">
    <MPageHeader
      :title="t('settings.title')"
      :description="t('settings.subtitle')"
      icon="i-lucide-settings"
    />

    <div class="flex flex-col md:flex-row gap-6 items-start">
      <!-- Left Sub-Nav -->
      <aside class="w-full md:w-[220px] shrink-0 m-card rounded-xl p-2 flex flex-col gap-3">
        <div v-for="group in navGroups" :key="group.key" class="flex flex-col gap-1">
          <div class="px-2.5 py-1 text-xs font-medium tracking-wide text-fg-subtle">
            {{ t(group.titleKey) }}
          </div>
          <button
            v-for="item in group.items"
            :key="item.key"
            type="button"
            class="h-9 px-3 rounded-lg flex items-center gap-2.5 text-xs text-left transition-colors"
            :class="[
              isActive(item)
                ? 'bg-primary-soft text-primary-soft-fg font-medium'
                : 'text-fg-muted hover:bg-sunken hover:text-fg',
            ]"
            @click="navigate(item)"
          >
            <span :class="[item.icon, 'w-4 h-4 shrink-0']" />
            <span class="truncate">{{ t(item.titleKey) }}</span>
          </button>
        </div>
      </aside>

      <!-- Right Content -->
      <div class="flex-1 min-w-0 w-full">
        <RouterView />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { featureRoutes, type NavMeta } from '../../router'
import { NAV_GROUPS } from './navGroups'
import AccountChip from './AccountChip.vue'
import { MLogo } from '../../ui'
import { useThemeStore } from '../../stores/theme'
const { t } = useI18n()
const router = useRouter()
const route = useRoute()
const themeStore = useThemeStore()
// Read initial collapsed state
function getSavedCollapsed(): boolean | null {
  try {
    if (typeof localStorage !== 'undefined' && typeof localStorage.getItem === 'function') {
      const val = localStorage.getItem('mclr.sidebar')
      if (val === 'collapsed') return true
      if (val === 'expanded') return false
    }
  } catch {
    // Ignore storage errors in test or restricted environments
  }
  return null
}

const savedPreference = ref<boolean | null>(getSavedCollapsed())
const isAutoCollapsed = ref(false)

const collapsed = computed(() => {
  if (savedPreference.value !== null) {
    return savedPreference.value
  }
  return isAutoCollapsed.value
})

watch(
  collapsed,
  (c) => {
    if (typeof document !== 'undefined') {
      document.documentElement.style.setProperty('--sidebar-w', c ? '68px' : '196px')
    }
  },
  { immediate: true },
)
function toggleCollapse() {
  const next = !collapsed.value
  savedPreference.value = next
  try {
    if (typeof localStorage !== 'undefined' && typeof localStorage.setItem === 'function') {
      localStorage.setItem('mclr.sidebar', next ? 'collapsed' : 'expanded')
    }
  } catch {
    // Ignore storage errors
  }
}

function handleResize() {
  if (typeof window !== 'undefined') {
    isAutoCollapsed.value = window.innerWidth < 1040
  }
}

onMounted(() => {
  handleResize()
  if (typeof window !== 'undefined') {
    window.addEventListener('resize', handleResize)
  }
})

onUnmounted(() => {
  if (typeof window !== 'undefined') {
    window.removeEventListener('resize', handleResize)
  }
})

interface NavItem {
  path: string
  name: string
  meta: NavMeta
}

const allNavItems = computed(() => {
  const items: NavItem[] = []
  for (const r of featureRoutes) {
    if (r.meta && 'nav' in r.meta && r.meta.nav && r.path !== '/tasks') {
      const cleanPath = r.path.replace(/\/:\w+\??.*/, '') || '/'
      items.push({
        path: cleanPath,
        name: String(r.name ?? r.path),
        meta: r.meta.nav as NavMeta,
      })
    }
  }
  return items.sort((a, b) => (a.meta.order ?? 0) - (b.meta.order ?? 0))
})

const groupedNav = computed(() => {
  return NAV_GROUPS.map((g) => {
    const items = allNavItems.value.filter((item) => (item.meta.group ?? 'system') === g.key)
    return {
      ...g,
      items,
    }
  }).filter((g) => g.items.length > 0)
})

function isItemActive(itemPath: string): boolean {
  if (!route?.path) return false
  // Special case for comp items: any /comp/* subroute activates /comp
  if ((itemPath === '/comp' || itemPath === '/comp/mod') && route.path.startsWith('/comp')) {
    return true
  }
  // Special case for settings items: any /settings/* activates /settings
  if (itemPath === '/settings' && route.path.startsWith('/settings')) {
    return true
  }
  return route.path === itemPath || route.path.startsWith(itemPath + '/')
}

const navContainerRef = ref<HTMLElement | null>(null)
const itemRefs = ref<Record<string, HTMLElement | null>>({})
const pillTop = ref(0)
const pillHeight = ref(36)
const pillVisible = ref(false)

function updatePill() {
  nextTick(() => {
    const container = navContainerRef.value
    if (!container) {
      pillVisible.value = false
      return
    }
    const activeItem = allNavItems.value.find((item) => isItemActive(item.path))
    if (!activeItem) {
      pillVisible.value = false
      return
    }
    const el = itemRefs.value[activeItem.path]
    if (el) {
      const containerRect = container.getBoundingClientRect()
      const elRect = el.getBoundingClientRect()
      if (elRect.height === 0 || elRect.width === 0) {
        pillVisible.value = false
        return
      }
      pillTop.value = elRect.top - containerRect.top + container.scrollTop
      pillHeight.value = elRect.height || 36
      pillVisible.value = true
    } else {
      pillVisible.value = false
    }
  })
}

let containerResizeObserver: ResizeObserver | null = null

function onContainerScroll() {
  updatePill()
}

watch(
  () => route?.path,
  () => updatePill(),
  { immediate: true },
)
watch(collapsed, () => updatePill())
watch(
  () => themeStore.scale,
  () => updatePill(),
)

onMounted(() => {
  updatePill()
  if (navContainerRef.value) {
    navContainerRef.value.addEventListener('scroll', onContainerScroll, { passive: true })
    if (typeof ResizeObserver !== 'undefined') {
      containerResizeObserver = new ResizeObserver(() => {
        updatePill()
      })
      containerResizeObserver.observe(navContainerRef.value)
    }
  }
})

onUnmounted(() => {
  if (navContainerRef.value) {
    navContainerRef.value.removeEventListener('scroll', onContainerScroll)
  }
  if (containerResizeObserver) {
    containerResizeObserver.disconnect()
    containerResizeObserver = null
  }
})
function setItemRef(path: string, el: unknown) {
  if (el) {
    itemRefs.value[path] = el as HTMLElement
  } else {
    delete itemRefs.value[path]
  }
}
</script>

<template>
  <aside
    class="m-panel shrink-0 flex flex-col justify-between transition-all duration-200 select-none overflow-hidden"
    :class="collapsed ? 'w-[68px] px-2 pb-2' : 'w-[196px] px-2.5 pb-2'"
  >
    <div class="flex-1 min-h-0 flex flex-col">
      <!-- Sidebar Header (Logo + App Name) -->
      <div
        data-tauri-drag-region
        class="h-12 shrink-0 flex items-center mb-2 data-tauri-drag-region"
        :class="collapsed ? 'justify-center px-0' : 'px-3'"
      >
        <MLogo data-tauri-drag-region class="w-7 h-7 shrink-0 text-primary" />
        <span
          v-if="!collapsed"
          data-tauri-drag-region
          class="text-base font-bold tracking-wider text-fg ml-3 select-none"
        >
          {{ t('app.name') }}
        </span>
      </div>

      <!-- Nav Groups -->
      <div ref="navContainerRef" class="relative flex-1 overflow-y-auto flex flex-col gap-2">
        <div
          v-if="pillVisible"
          class="m-nav-pill absolute left-0 right-0 rounded-xl pointer-events-none transition-all duration-240"
          :style="{
            top: `${pillTop}px`,
            height: `${pillHeight}px`,
            transitionTimingFunction: 'var(--ease)',
          }"
        >
          <!-- Left Accent Bar -->
          <span
            class="m-nav-accent absolute left-1 top-2.5 bottom-2.5 w-[3px] bg-primary rounded-full"
          />
        </div>

        <div
          v-for="(group, gIdx) in groupedNav"
          :key="group.key"
          class="flex flex-col gap-1.5 relative z-10"
        >
          <div
            v-if="!collapsed"
            class="text-[11px] font-medium tracking-wide text-fg-subtle px-3 pt-2 pb-0.5"
          >
            {{ t(group.titleKey) }}
          </div>
          <div v-else-if="gIdx > 0" class="border-t border-line my-1 mx-1" />

          <!-- Nav Items -->
          <button
            v-for="item in group.items"
            :key="item.path"
            :ref="(el) => setItemRef(item.path, el)"
            type="button"
            :title="collapsed ? t(item.meta.titleKey) : undefined"
            class="relative h-9 rounded-xl transition-colors cursor-pointer flex items-center"
            :class="[
              collapsed ? 'justify-center w-full px-0' : 'gap-2.5 px-3 text-left w-full',
              isItemActive(item.path)
                ? 'text-primary-soft-fg font-medium'
                : 'text-fg-muted hover:bg-sunken hover:text-fg',
            ]"
            @click="router?.push ? router.push(item.path) : undefined"
          >
            <span :class="item.meta.icon" class="w-5 h-5 shrink-0" />
            <span v-if="!collapsed" class="text-xs truncate">
              {{ t(item.meta.titleKey) }}
            </span>
          </button>
        </div>
      </div>
    </div>
    <!-- Footer: Collapse Toggle & Account Chip -->
    <div class="pt-2 border-t border-line flex flex-col gap-2 shrink-0">
      <!-- Collapse toggle button -->
      <button
        type="button"
        :title="collapsed ? t('shell.sidebar.expand') : t('shell.sidebar.collapse')"
        class="h-8 rounded-lg flex items-center text-fg-muted hover:text-fg hover:bg-sunken cursor-pointer transition-colors"
        :class="collapsed ? 'justify-center w-full' : 'gap-2 px-2 w-full text-xs'"
        @click="toggleCollapse"
      >
        <span
          :class="collapsed ? 'i-lucide-panel-left-open' : 'i-lucide-panel-left-close'"
          class="w-4 h-4 shrink-0"
        />
        <span v-if="!collapsed" class="truncate">
          {{ t('shell.sidebar.collapse') }}
        </span>
      </button>

      <!-- Account Chip -->
      <AccountChip :collapsed="collapsed" />
    </div>
  </aside>
</template>

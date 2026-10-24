<script setup lang="ts">
import { computed } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { featureRoutes, type NavMeta } from '../../router'
import { useThemeStore } from '../../stores/theme'
import { MToast, MDialog, useDialog } from '../../ui'

const { t } = useI18n()
const router = useRouter()
const route = useRoute()
const themeStore = useThemeStore()
const { activeDialog, handleConfirm, handleCancel } = useDialog()

const navItems = computed(() => {
  const items: Array<{ path: string; name: string; meta: NavMeta }> = []
  for (const r of featureRoutes) {
    if (r.meta && 'nav' in r.meta && r.meta.nav) {
      items.push({
        path: r.path,
        name: String(r.name ?? r.path),
        meta: r.meta.nav as NavMeta,
      })
    }
  }
  return items.sort((a, b) => (a.meta.order ?? 0) - (b.meta.order ?? 0))
})

async function minimizeWindow() {
  try {
    const win = getCurrentWindow()
    await win.minimize()
  } catch {
    // Non-tauri browser mode fallback
  }
}

async function toggleMaximizeWindow() {
  try {
    const win = getCurrentWindow()
    await win.toggleMaximize()
  } catch {
    // Non-tauri browser mode fallback
  }
}

async function closeWindow() {
  try {
    const win = getCurrentWindow()
    await win.close()
  } catch {
    // Non-tauri browser mode fallback
  }
}

function toggleMode() {
  if (themeStore.mode === 'light') {
    themeStore.setMode('dark')
  } else if (themeStore.mode === 'dark') {
    themeStore.setMode('system')
  } else {
    themeStore.setMode('light')
  }
}

function toggleMaterial() {
  themeStore.setMaterial(themeStore.material === 'glass' ? 'flat' : 'glass')
}
</script>

<template>
  <div class="w-screen h-screen flex flex-col bg-surface text-text select-none overflow-hidden">
    <!-- SVG Refraction Filter for Liquid Glass -->
    <svg class="absolute w-0 h-0 overflow-hidden" aria-hidden="true">
      <defs>
        <filter id="mclr-glass" x="-20%" y="-20%" width="140%" height="140%">
          <feTurbulence
            type="fractalNoise"
            baseFrequency="0.04"
            numOctaves="3"
            result="noise"
          />
          <feDisplacementMap
            in="SourceGraphic"
            in2="noise"
            scale="4"
            xChannelSelector="R"
            yChannelSelector="G"
          />
        </filter>
      </defs>
    </svg>

    <!-- Custom Frameless Titlebar -->
    <header
      data-tauri-drag-region
      class="h-10 shrink-0 flex items-center justify-between px-3 border-b border-outline/40 bg-surfaceVariant/30 backdrop-blur-md z-40 select-none"
    >
      <div data-tauri-drag-region class="flex items-center gap-2.5">
        <div class="w-5 h-5 rounded bg-primary text-onPrimary flex items-center justify-center font-bold text-xs shadow-sm">
          M
        </div>
        <span data-tauri-drag-region class="text-xs font-semibold tracking-wide text-text">
          {{ t('app.name') }}
        </span>
      </div>

      <!-- Center Titlebar / Task Indicator Slot -->
      <div data-tauri-drag-region class="flex-1 flex items-center justify-center px-4">
        <div id="mclr-task-indicator-slot" class="flex items-center gap-2 text-xs text-textMuted">
          <!-- Populated by Task Indicator in v0.0.3 -->
        </div>
      </div>

      <!-- Titlebar Quick Actions & Window Controls -->
      <div class="flex items-center gap-1">
        <button
          type="button"
          :title="t('shell.titlebar.theme')"
          class="w-7 h-7 flex items-center justify-center rounded hover:bg-surfaceVariant/80 text-textMuted hover:text-text cursor-pointer transition-colors"
          @click="toggleMode"
        >
          <span
            class="w-3.5 h-3.5"
            :class="{
              'i-lucide-sun': themeStore.mode === 'light',
              'i-lucide-moon': themeStore.mode === 'dark',
              'i-lucide-laptop': themeStore.mode === 'system',
            }"
          />
        </button>

        <button
          type="button"
          :title="t('shell.titlebar.material')"
          class="w-7 h-7 flex items-center justify-center rounded hover:bg-surfaceVariant/80 text-textMuted hover:text-text cursor-pointer transition-colors"
          :class="{ 'text-primary': themeStore.material === 'glass' }"
          @click="toggleMaterial"
        >
          <span class="i-lucide-sparkles w-3.5 h-3.5" />
        </button>

        <div class="w-px h-3.5 bg-outline/40 mx-1" />

        <button
          type="button"
          :title="t('app.window.minimize')"
          class="w-7 h-7 flex items-center justify-center rounded hover:bg-surfaceVariant/80 text-textMuted hover:text-text cursor-pointer transition-colors"
          @click="minimizeWindow"
        >
          <span class="i-lucide-minus w-3.5 h-3.5" />
        </button>

        <button
          type="button"
          :title="t('app.window.maximize')"
          class="w-7 h-7 flex items-center justify-center rounded hover:bg-surfaceVariant/80 text-textMuted hover:text-text cursor-pointer transition-colors"
          @click="toggleMaximizeWindow"
        >
          <span class="i-lucide-square w-3 h-3" />
        </button>

        <button
          type="button"
          :title="t('app.window.close')"
          class="w-7 h-7 flex items-center justify-center rounded hover:bg-danger hover:text-white text-textMuted cursor-pointer transition-colors"
          @click="closeWindow"
        >
          <span class="i-lucide-x w-3.5 h-3.5" />
        </button>
      </div>
    </header>

    <!-- Main Body: Sidebar + Content -->
    <div class="flex-1 flex overflow-hidden">
      <!-- Left Navigation Bar -->
      <nav class="w-48 shrink-0 flex flex-col gap-1 p-2 border-r border-outline/40 bg-surfaceVariant/20 select-none">
        <button
          v-for="item in navItems"
          :key="item.path"
          type="button"
          class="flex items-center gap-2.5 px-3 py-2 text-xs font-medium rounded-xl transition-all cursor-pointer text-left"
          :class="
            route.path === item.path
              ? 'bg-primary text-onPrimary font-semibold shadow-sm'
              : 'text-textMuted hover:text-text hover:bg-surfaceVariant/60'
          "
          @click="router.push(item.path)"
        >
          <span :class="item.meta.icon" class="w-4 h-4 shrink-0" />
          <span class="truncate">{{ t(item.meta.titleKey) }}</span>
        </button>
      </nav>

      <!-- Content Area -->
      <main class="flex-1 overflow-auto bg-surface/50 p-6">
        <RouterView v-slot="{ Component }">
          <Transition name="page" mode="out-in">
            <component :is="Component" />
          </Transition>
        </RouterView>
      </main>
    </div>

    <!-- Global Toast & Dialog Hosts -->
    <MToast />

    <MDialog
      v-if="activeDialog"
      :open="activeDialog.open"
      :title="activeDialog.title"
      :confirm-text="activeDialog.confirmText"
      :cancel-text="activeDialog.cancelText"
      @confirm="handleConfirm"
      @cancel="handleCancel"
    >
      <p>{{ activeDialog.content }}</p>
    </MDialog>
  </div>
</template>

<style scoped>
.page-enter-active,
.page-leave-active {
  transition: opacity 0.15s ease, transform 0.15s ease;
}
.page-enter-from {
  opacity: 0;
  transform: translateY(4px);
}
.page-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}
</style>

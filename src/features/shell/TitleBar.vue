<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { getCurrentWindow } from '@tauri-apps/api/window'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { MLogo } from '../../ui'
import TaskIndicator from '../tasks/TaskIndicator.vue'
import QuickSettings from './QuickSettings.vue'

defineProps<{}>()

const { t } = useI18n()
const isMaximized = ref(false)
let unlistenResized: UnlistenFn | null = null

async function checkMaximized() {
  try {
    const win = getCurrentWindow()
    isMaximized.value = await win.isMaximized()
  } catch {
    // Non-tauri browser mode fallback
  }
}

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
    await checkMaximized()
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

onMounted(async () => {
  await checkMaximized()
  try {
    const win = getCurrentWindow()
    unlistenResized = await win.onResized(async () => {
      await checkMaximized()
    })
  } catch {
    // Non-tauri browser mode
  }
})

onUnmounted(() => {
  if (unlistenResized) {
    unlistenResized()
    unlistenResized = null
  }
})
</script>

<template>
  <header
    data-tauri-drag-region
    class="relative h-11 shrink-0 flex items-center justify-between px-3 border-b border-line bg-transparent z-40 select-none"
  >
    <!-- Left Logo + App Name -->
    <div data-tauri-drag-region class="flex items-center gap-2.5">
      <MLogo data-tauri-drag-region class="w-5 h-5 shrink-0" />
      <span data-tauri-drag-region class="text-xs font-semibold tracking-wide text-fg">
        {{ t('app.name') }}
      </span>
    </div>

    <!-- Center Task Indicator -->
    <div data-tauri-drag-region class="flex-1 flex items-center justify-center px-4">
      <div id="mclr-task-indicator-slot" class="flex items-center gap-2 text-xs text-fg-muted">
        <TaskIndicator />
      </div>
    </div>

    <!-- Right Controls: QuickSettings, Window Buttons -->
    <div class="flex items-center">
      <!-- Quick Settings Popover -->
      <div class="mr-2">
        <QuickSettings />
      </div>

      <!-- Windows-style Caption Buttons (46x44 px) -->
      <div class="flex items-center -mr-3">
        <!-- Minimize -->
        <button
          type="button"
          :title="t('app.window.minimize')"
          class="w-[46px] h-11 flex items-center justify-center text-fg-muted hover:text-fg hover:bg-sunken cursor-pointer transition-colors"
          @click="minimizeWindow"
        >
          <span class="i-lucide-minus w-4 h-4" />
        </button>

        <!-- Maximize / Restore -->
        <button
          type="button"
          :title="isMaximized ? t('app.window.restore') : t('app.window.maximize')"
          class="w-[46px] h-11 flex items-center justify-center text-fg-muted hover:text-fg hover:bg-sunken cursor-pointer transition-colors"
          @click="toggleMaximizeWindow"
        >
          <span :class="isMaximized ? 'i-lucide-copy' : 'i-lucide-square'" class="w-3.5 h-3.5" />
        </button>

        <!-- Close -->
        <button
          type="button"
          :title="t('app.window.close')"
          class="w-[46px] h-11 flex items-center justify-center text-fg-muted hover:text-white hover:bg-danger cursor-pointer transition-colors"
          @click="closeWindow"
        >
          <span class="i-lucide-x w-4 h-4" />
        </button>
      </div>
    </div>
  </header>
</template>

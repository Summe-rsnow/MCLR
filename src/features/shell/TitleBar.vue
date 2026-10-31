<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { getCurrentWindow } from '@tauri-apps/api/window'
import type { UnlistenFn } from '@tauri-apps/api/event'
import TaskIndicator from '../tasks/TaskIndicator.vue'
import QuickSettings from './QuickSettings.vue'
import { ipcClient } from '../../ipc/client'
import { MButton, MCheckbox, MDialog } from '../../ui'

defineProps<{
  scrolled?: boolean
}>()

const { t } = useI18n()
const isMaximized = ref(false)
const closePromptOpen = ref(false)
const rememberCloseChoice = ref(false)
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
  // First close asks what the button should do; the answer (and whether to keep
  // asking) is stored in settings, so this only interrupts once.
  try {
    const settings = await ipcClient.settingsGet()
    if (!settings.system?.closePromptDismissed) {
      rememberCloseChoice.value = false
      closePromptOpen.value = true
      return
    }
  } catch {
    // Non-tauri browser mode fallback: close straight away
  }
  await requestClose()
}

async function requestClose() {
  try {
    const win = getCurrentWindow()
    await win.close()
  } catch {
    // Non-tauri browser mode fallback
  }
}

async function chooseCloseBehavior(hideToTray: boolean) {
  closePromptOpen.value = false
  try {
    await ipcClient.settingsPatch({
      system: {
        keepInBackground: hideToTray,
        closePromptDismissed: rememberCloseChoice.value,
      },
    })
  } catch {
    // Settings unavailable: still honour the picked action for this close
  }
  await requestClose()
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
    class="relative h-11 shrink-0 flex items-center justify-between px-3 z-40 select-none"
    :class="[
      scrolled
        ? 'm-titlebar-scrolled transition-colors duration-200'
        : 'bg-transparent border-b border-transparent transition-colors duration-200',
    ]"
  >
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

    <!-- First close: pick what the button does, optionally stop asking -->
    <MDialog
      :open="closePromptOpen"
      :title="t('app.closePrompt.title')"
      hide-footer
      max-width="sm"
      @update:open="closePromptOpen = $event"
    >
      <div class="flex flex-col gap-4" data-testid="close-prompt">
        <p class="text-sm text-fg-muted leading-relaxed">{{ t('app.closePrompt.desc') }}</p>
        <MCheckbox v-model="rememberCloseChoice" :label="t('app.closePrompt.remember')" />
        <div class="flex flex-col gap-2">
          <MButton
            variant="primary"
            block
            icon="i-lucide-minus-square"
            data-testid="close-prompt-hide"
            @click="chooseCloseBehavior(true)"
          >
            {{ t('app.closePrompt.hideToTray') }}
          </MButton>
          <MButton
            variant="tonal"
            block
            icon="i-lucide-power"
            data-testid="close-prompt-quit"
            @click="chooseCloseBehavior(false)"
          >
            {{ t('app.closePrompt.quit') }}
          </MButton>
        </div>
        <div class="flex justify-end">
          <MButton variant="text" size="sm" @click="closePromptOpen = false">
            {{ t('common.cancel') }}
          </MButton>
        </div>
      </div>
    </MDialog>
  </header>
</template>

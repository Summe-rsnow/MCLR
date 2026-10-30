<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { useThemeStore } from '../../stores/theme'
import {
  MToast,
  MDialog,
  MCheckbox,
  useDialog,
  useToast,
  GlassDefs,
  AmbientBackdrop,
  MImage,
} from '../../ui'
import { useI18n } from 'vue-i18n'
import { ipcClient } from '../../ipc/client'
import { useCrashStore } from '../crash/store'
import CrashDialog from '../crash/CrashDialog.vue'
import TitleBar from './TitleBar.vue'
import SidebarNav from './SidebarNav.vue'
import { useBackgroundMedia } from './useBackgroundMedia'

const { t } = useI18n()
const toast = useToast()
const themeStore = useThemeStore()
const { activeDialog, handleConfirm, handleCancel } = useDialog()
const checkboxAgreed = ref(false)
const isConfirmDisabled = computed(() =>
  Boolean(activeDialog.value?.requireCheckbox && !checkboxAgreed.value),
)

watch(
  () => activeDialog.value,
  () => {
    checkboxAgreed.value = false
  },
)

function onDialogConfirm() {
  if (isConfirmDisabled.value) return
  checkboxAgreed.value = false
  handleConfirm()
}

function onDialogCancel() {
  checkboxAgreed.value = false
  handleCancel()
}

const crashStore = useCrashStore()

// Background composable
const { backgroundMediaSrc, cleanup: cleanupBackground } = useBackgroundMedia()

const isScrolled = ref(false)
function onMainScroll(e: Event) {
  const target = e.target as HTMLElement | null
  isScrolled.value = (target?.scrollTop ?? 0) > 6
}

/**
 * Settings -> Update -> "check for updates on startup". Only runs when the build has a signed
 * feed; failures never block startup.
 */
async function checkUpdatesOnStartup() {
  try {
    const settings = await ipcClient.settingsGet()
    if (settings.update?.autoCheck === false) return
    const secrets = await ipcClient.secretsStatus()
    if (!secrets.updateEndpoint || !secrets.updatePubkey) return
    const result = await ipcClient.updateCheck(settings.update?.channel)
    if (result.available && result.version) {
      toast.show({
        message: t('shell.updateAvailable', { version: result.version }),
        type: 'info',
        duration: 8000,
      })
    }
  } catch {
    // Offline / unconfigured: stay quiet
  }
}

onMounted(() => {
  crashStore.startListening()
  void checkUpdatesOnStartup()
})

onUnmounted(() => {
  crashStore.stopListening()
  cleanupBackground()
})
</script>

<template>
  <div
    class="relative w-screen h-screen flex flex-row bg-canvas text-fg select-none overflow-hidden"
  >
    <!-- SVG Refraction Filter for Liquid Glass -->
    <GlassDefs />

    <!-- Ambient dynamic backdrop blobs -->
    <AmbientBackdrop />

    <!-- Background Layer (Behind shell content) -->
    <div
      v-if="themeStore.backgroundMode !== 'none' && backgroundMediaSrc"
      data-testid="bg-layer"
      class="pointer-events-none fixed inset-0 z-0 overflow-hidden"
    >
      <MImage
        data-testid="bg-img"
        class="w-full h-full pointer-events-none"
        :src="backgroundMediaSrc"
        :style="{
          filter: `blur(${themeStore.backgroundBlur}px)`,
          opacity: themeStore.backgroundOpacity,
        }"
        alt="Background"
        fallback-icon="i-lucide-image"
      />
    </div>

    <!-- Grouped Sidebar Navigation -->
    <SidebarNav class="relative z-10" />

    <!-- Main Body: TitleBar + Content Column -->
    <div class="relative z-10 flex-1 min-w-0 flex flex-col overflow-hidden">
      <!-- Custom Frameless Titlebar (Sticky/fixed over scrolling content) -->
      <TitleBar :scrolled="isScrolled" class="absolute top-0 inset-x-0 z-40" />

      <div class="relative flex-1 min-h-0 flex flex-col">
        <!-- Float dock for floating controls (e.g. MPagination) -->
        <div
          id="m-float-dock"
          class="pointer-events-none absolute inset-x-0 bottom-4 z-30 flex justify-center"
        />

        <!-- Content Area with Page Transition -->
        <main
          class="relative flex-1 overflow-y-auto overflow-x-hidden [overflow-y:overlay] pt-11"
          @scroll.passive="onMainScroll"
        >
          <RouterView v-slot="{ Component }">
            <Transition name="m-page-fade" mode="out-in">
              <component :is="Component" />
            </Transition>
          </RouterView>
        </main>
      </div>
    </div>

    <!-- Global Toast & Dialog Hosts -->
    <MToast />

    <MDialog
      v-if="activeDialog"
      :open="activeDialog.open"
      :title="activeDialog.title"
      :confirm-text="activeDialog.confirmText"
      :cancel-text="activeDialog.cancelText"
      :confirm-variant="activeDialog.confirmVariant"
      :confirm-disabled="isConfirmDisabled"
      @confirm="onDialogConfirm"
      @cancel="onDialogCancel"
    >
      <div class="space-y-3">
        <div class="flex items-start gap-3">
          <div
            v-if="activeDialog.severity"
            class="shrink-0 w-8 h-8 rounded-lg flex items-center justify-center mt-0.5"
            :class="[
              activeDialog.severity === 'danger'
                ? 'bg-danger-soft text-danger-fg'
                : activeDialog.severity === 'info'
                  ? 'bg-primary-soft text-primary-soft-fg'
                  : 'bg-warning-soft text-warning-fg',
            ]"
          >
            <span
              :class="[
                activeDialog.severity === 'info' ? 'i-lucide-info' : 'i-lucide-triangle-alert',
              ]"
              class="w-4 h-4 block"
            />
          </div>
          <div class="flex-1 min-w-0">
            <p class="text-sm text-fg leading-relaxed">{{ activeDialog.content }}</p>
          </div>
        </div>

        <!-- Optional Detail Callout Box -->
        <div
          v-if="activeDialog.detail"
          class="m-well p-3 rounded-lg text-xs text-fg-muted font-mono whitespace-pre-wrap break-all max-h-36 overflow-y-auto"
        >
          {{ activeDialog.detail }}
        </div>

        <!-- Optional Checkbox Gate -->
        <div v-if="activeDialog.requireCheckbox" class="pt-1">
          <MCheckbox v-model="checkboxAgreed" :label="activeDialog.requireCheckbox" />
        </div>
      </div>
    </MDialog>

    <!-- Global Crash Analysis Dialog -->
    <CrashDialog />
  </div>
</template>

<style scoped>
.m-page-fade-enter-active,
.m-page-fade-leave-active {
  transition:
    opacity 0.2s cubic-bezier(0.2, 0.8, 0.2, 1),
    transform 0.2s cubic-bezier(0.2, 0.8, 0.2, 1);
}

.m-page-fade-enter-from {
  opacity: 0;
  transform: translateY(8px);
}

.m-page-fade-leave-to {
  opacity: 0;
  transform: translateY(-8px);
}

@media (prefers-reduced-motion: reduce) {
  .m-page-fade-enter-active,
  .m-page-fade-leave-active {
    transition: none;
  }
  .m-page-fade-enter-from,
  .m-page-fade-leave-to {
    transform: none;
  }
}
</style>

<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'
import { useThemeStore } from '../../stores/theme'
import { MToast, MDialog, useDialog, GlassDefs, AmbientBackdrop } from '../../ui'
import { useCrashStore } from '../crash/store'
import CrashDialog from '../crash/CrashDialog.vue'
import TitleBar from './TitleBar.vue'
import SidebarNav from './SidebarNav.vue'
import { useBackgroundMedia } from './useBackgroundMedia'

const themeStore = useThemeStore()
const { activeDialog, handleConfirm, handleCancel } = useDialog()
const crashStore = useCrashStore()

// Background composable
const { backgroundMediaSrc, cleanup: cleanupBackground } = useBackgroundMedia()


onMounted(() => {
  crashStore.startListening()
})

onUnmounted(() => {
  crashStore.stopListening()
  cleanupBackground()
})
</script>

<template>
  <div class="relative w-screen h-screen flex flex-col bg-canvas text-fg select-none overflow-hidden">
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
      <video
        v-if="themeStore.backgroundMode === 'video'"
        data-testid="bg-video"
        autoplay
        muted
        loop
        playsinline
        class="w-full h-full object-cover transition-opacity duration-500"
        :src="backgroundMediaSrc"
        :style="{
          filter: `blur(${themeStore.backgroundBlur}px)`,
          opacity: themeStore.backgroundOpacity,
        }"
      />
      <img
        v-else-if="themeStore.backgroundMode === 'image' || themeStore.backgroundMode === 'slideshow'"
        data-testid="bg-img"
        class="w-full h-full object-cover transition-opacity duration-500"
        :src="backgroundMediaSrc"
        :style="{
          filter: `blur(${themeStore.backgroundBlur}px)`,
          opacity: themeStore.backgroundOpacity,
        }"
        alt="Background"
       loading="lazy" decoding="async"/>
    </div>

    <!-- Custom Frameless Titlebar -->
    <TitleBar />

    <!-- Main Body: Sidebar + Content (Relative z-10 over background) -->
    <div class="relative z-10 flex-1 flex overflow-hidden">
      <!-- Grouped Sidebar Navigation -->
      <SidebarNav />

      <!-- Content Area with Page Transition -->
      <main class="relative z-10 flex-1 overflow-auto">
        <RouterView v-slot="{ Component }">
          <Transition name="m-page-fade" mode="out-in">
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

    <!-- Global Crash Analysis Dialog -->
    <CrashDialog />
  </div>
</template>

<style scoped>
.m-page-fade-enter-active,
.m-page-fade-leave-active {
  transition: opacity 0.2s cubic-bezier(0.2, 0.8, 0.2, 1), transform 0.2s cubic-bezier(0.2, 0.8, 0.2, 1);
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

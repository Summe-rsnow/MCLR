<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useThemeStore } from '../../stores/theme'
import { MLogo } from '../../ui'

const emit = defineEmits<{
  (e: 'done'): void
}>()

const { t } = useI18n()
const themeStore = useThemeStore()

const isFadingOut = ref(false)
let fadeTimer: ReturnType<typeof setTimeout> | null = null
let doneTimer: ReturnType<typeof setTimeout> | null = null

function cleanup() {
  if (fadeTimer) {
    clearTimeout(fadeTimer)
    fadeTimer = null
  }
  if (doneTimer) {
    clearTimeout(doneTimer)
    doneTimer = null
  }
}

function dismiss() {
  cleanup()
  emit('done')
}

onMounted(() => {
  const motion = themeStore.motion
  if (motion === 'off') {
    emit('done')
    return
  }

  if (motion === 'reduced') {
    fadeTimer = setTimeout(() => {
      isFadingOut.value = true
    }, 500)
    doneTimer = setTimeout(() => {
      emit('done')
    }, 700)
  } else {
    // Full motion
    fadeTimer = setTimeout(() => {
      isFadingOut.value = true
    }, 1300)
    doneTimer = setTimeout(() => {
      emit('done')
    }, 1600)
  }
})

onUnmounted(() => {
  cleanup()
})
</script>

<template>
  <div
    data-tauri-drag-region
    class="fixed inset-0 z-[10000] bg-canvas flex flex-col items-center justify-center gap-4 cursor-pointer select-none transition-opacity duration-300"
    :class="{ 'm-splash-out': isFadingOut }"
    @click="dismiss"
  >
    <MLogo class="w-24 h-24 m-splash-logo" />
    <span class="m-splash-word text-lg font-bold tracking-[0.3em] text-fg">
      {{ t('app.name') }}
    </span>
  </div>
</template>

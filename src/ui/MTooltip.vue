<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'

defineProps<{
  text: string
}>()

const visible = ref(false)
const triggerRef = ref<HTMLElement | null>(null)
const coords = ref<{ top: number; left: number; placement: 'top' | 'bottom' }>({
  top: 0,
  left: 0,
  placement: 'top',
})

const updatePosition = () => {
  if (!triggerRef.value) return
  const rect = triggerRef.value.getBoundingClientRect()

  // Horizontal center
  const left = rect.left + rect.width / 2

  // Smart placement: if rect.top < 100, flip to render below the element, otherwise above
  if (rect.top < 100) {
    coords.value = {
      top: rect.bottom + 8,
      left,
      placement: 'bottom',
    }
  } else {
    coords.value = {
      top: rect.top - 8,
      left,
      placement: 'top',
    }
  }
}

const show = () => {
  updatePosition()
  visible.value = true
}

const hide = () => {
  visible.value = false
}

const onScrollOrResize = () => {
  if (visible.value) {
    updatePosition()
  }
}

onMounted(() => {
  window.addEventListener('scroll', onScrollOrResize, true)
  window.addEventListener('resize', onScrollOrResize)
})

onUnmounted(() => {
  window.removeEventListener('scroll', onScrollOrResize, true)
  window.removeEventListener('resize', onScrollOrResize)
})
</script>

<template>
  <div
    ref="triggerRef"
    class="relative inline-flex items-center"
    @mouseenter="show"
    @mouseleave="hide"
    @focusin="show"
    @focusout="hide"
  >
    <slot />
    <Teleport to="body">
      <Transition name="fade">
        <div
          v-if="visible"
          class="m-overlay fixed px-3 py-2 text-xs rounded-xl text-fg pointer-events-none z-[9999] w-max max-w-[280px] min-w-[120px] whitespace-normal break-words leading-relaxed shadow-lg text-left -translate-x-1/2"
          :class="coords.placement === 'top' ? '-translate-y-full' : ''"
          :style="{
            top: `${coords.top}px`,
            left: `${coords.left}px`,
          }"
        >
          {{ text }}
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<style scoped>
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.14s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>

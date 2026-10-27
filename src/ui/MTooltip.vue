<script setup lang="ts">
import { nextTick, ref, onMounted, onUnmounted } from 'vue'
import { computeFloatingPosition, FLOATING_PADDING } from './floating'

defineProps<{
  text: string
}>()

const visible = ref(false)
const positioned = ref(false)
const triggerRef = ref<HTMLElement | null>(null)
const bubbleRef = ref<HTMLElement | null>(null)
const coords = ref<{ top: number; left: number; placement: 'top' | 'bottom' }>({
  top: 0,
  left: 0,
  placement: 'top',
})

/**
 * Anchors the bubble above the trigger, flipping below when the top edge is
 * tight and shifting horizontally to stay inside the viewport. Needs the bubble
 * mounted to measure it, so the caller renders it invisible first.
 */
const updatePosition = () => {
  if (!triggerRef.value) return
  const anchor = triggerRef.value.getBoundingClientRect()
  const bubble = bubbleRef.value

  // Natural size before clamping; `w-max` caps at 280px via CSS.
  const size = {
    width: bubble?.offsetWidth ?? 240,
    height: bubble?.offsetHeight ?? 40,
  }
  const result = computeFloatingPosition({
    anchor,
    size,
    preferred: anchor.top < 100 ? 'bottom' : 'top',
    align: 'center',
    offset: 8,
    padding: FLOATING_PADDING,
  })

  coords.value = {
    top: result.side === 'top' ? result.top + size.height : result.top,
    left: result.left + size.width / 2,
    placement: result.side,
  }
  if (result.maxHeight !== undefined && bubble) {
    bubble.style.maxHeight = `${result.maxHeight}px`
  }
  positioned.value = true
}

const show = () => {
  positioned.value = false
  visible.value = true
  void nextTick(() => updatePosition())
}

const hide = () => {
  visible.value = false
  positioned.value = false
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
          ref="bubbleRef"
          class="m-overlay fixed px-3 py-2 text-xs rounded-xl text-fg pointer-events-none z-[9999] w-max max-w-[280px] min-w-[120px] whitespace-normal break-words leading-relaxed shadow-lg text-left -translate-x-1/2 select-text"
          :class="[
            coords.placement === 'top' ? '-translate-y-full' : '',
            positioned ? '' : 'opacity-0',
          ]"
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

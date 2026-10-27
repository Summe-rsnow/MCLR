<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { applyFloatingPosition } from './floating'

const props = withDefaults(
  defineProps<{
    open: boolean
    placement?: 'bottom-end' | 'bottom-start' | 'top-end' | 'top-start'
  }>(),
  {
    placement: 'bottom-end',
  },
)

const emit = defineEmits<{
  (e: 'update:open', val: boolean): void
}>()

const anchorRef = ref<HTMLElement | null>(null)
const panelRef = ref<HTMLElement | null>(null)
const posStyle = ref<Record<string, string>>({})

function updatePosition() {
  const el = anchorRef.value?.firstElementChild
    ? anchorRef.value
    : (anchorRef.value?.parentElement ?? anchorRef.value)
  const panel = panelRef.value
  if (!el) return
  if (!panel) return

  // Anchor the panel to the trigger's own box, not the inline wrapper, so a
  // wrapper that also holds the teleported content is not measured twice.
  const result = applyFloatingPosition(panel, el, {
    preferred: props.placement.startsWith('top') ? 'top' : 'bottom',
    align: props.placement.endsWith('end') ? 'end' : 'start',
    offset: 6,
    constrain: false,
    size: { width: panel.offsetWidth, height: panel.offsetHeight },
  })
  if (!result) return

  posStyle.value = {
    left: `${result.left}px`,
    top: `${result.top}px`,
    ...(result.maxWidth !== undefined ? { maxWidth: `${result.maxWidth}px` } : {}),
    ...(result.maxHeight !== undefined ? { maxHeight: `${result.maxHeight}px` } : {}),
  }
}

watch(
  () => props.open,
  async (val) => {
    if (val) {
      await nextTick()
      updatePosition()
    }
  },
  { immediate: true },
)

function onViewportChange() {
  if (props.open) updatePosition()
}

function handlePointerDown(event: MouseEvent) {
  if (!props.open) return
  const target = event.target as Node | null
  if (!target) return
  if (panelRef.value?.contains(target)) return
  const host = anchorRef.value?.firstElementChild
    ? anchorRef.value
    : (anchorRef.value?.parentElement ?? anchorRef.value)
  if (host?.contains(target)) return
  emit('update:open', false)
}

function handleKeyDown(event: KeyboardEvent) {
  if (props.open && event.key === 'Escape') {
    emit('update:open', false)
  }
}

onMounted(() => {
  if (typeof window !== 'undefined') {
    window.addEventListener('mousedown', handlePointerDown)
    window.addEventListener('keydown', handleKeyDown)
    window.addEventListener('resize', onViewportChange)
    window.addEventListener('scroll', onViewportChange, true)
  }
})

onUnmounted(() => {
  if (typeof window !== 'undefined') {
    window.removeEventListener('mousedown', handlePointerDown)
    window.removeEventListener('keydown', handleKeyDown)
    window.removeEventListener('resize', onViewportChange)
    window.removeEventListener('scroll', onViewportChange, true)
  }
})
</script>

<template>
  <span ref="anchorRef" class="inline-flex">
    <slot name="trigger" />
    <Teleport to="body">
      <Transition name="m-pop">
        <div
          v-if="open"
          ref="panelRef"
          data-testid="m-popover"
          class="fixed z-50 m-overlay m-refract rounded-xl p-3 text-fg select-none"
          :style="{ ...posStyle, '--origin': placement.startsWith('top') ? 'bottom' : 'top' }"
        >
          <slot />
        </div>
      </Transition>
    </Teleport>
  </span>
</template>

<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, watch } from 'vue'

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
  if (!el) return
  const rect = el.getBoundingClientRect()
  const vw = typeof window !== 'undefined' ? window.innerWidth : 1280
  const vh = typeof window !== 'undefined' ? window.innerHeight : 800
  const style: Record<string, string> = {}

  if (props.placement.startsWith('top')) {
    style.bottom = `${Math.max(8, vh - rect.top + 6)}px`
  } else {
    style.top = `${Math.max(8, rect.bottom + 6)}px`
  }

  if (props.placement.endsWith('end')) {
    style.right = `${Math.max(8, vw - rect.right)}px`
  } else {
    style.left = `${Math.max(8, rect.left)}px`
  }

  posStyle.value = style
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
    window.addEventListener('resize', updatePosition)
  }
})

onUnmounted(() => {
  if (typeof window !== 'undefined') {
    window.removeEventListener('mousedown', handlePointerDown)
    window.removeEventListener('keydown', handleKeyDown)
    window.removeEventListener('resize', updatePosition)
  }
})
</script>

<template>
  <span ref="anchorRef" class="inline-flex">
    <slot name="trigger" />
    <Teleport to="body">
      <div
        v-if="open"
        ref="panelRef"
        data-testid="m-popover"
        class="fixed z-50 m-overlay m-refract rounded-xl p-3 text-fg select-none"
        :style="posStyle"
      >
        <slot />
      </div>
    </Teleport>
  </span>
</template>

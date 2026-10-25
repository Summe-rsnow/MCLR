<script setup lang="ts">
import { computed, onUnmounted, watch } from 'vue'
import { getNextOverlayId, lockBody, overlayStack, unlockBody } from './overlayStack'

const props = withDefaults(
  defineProps<{
    open: boolean
    zIndex?: number
    blur?: boolean
    closeOnBackdrop?: boolean
    closeOnEsc?: boolean
    align?: 'center' | 'right' | 'left' | 'bottom'
    transition?:
      | 'm-modal'
      | 'm-drawer-right'
      | 'm-drawer-left'
      | 'm-drawer-bottom'
      | 'm-slide-up'
      | 'm-fade'
  }>(),
  {
    zIndex: 50,
    blur: true,
    closeOnBackdrop: true,
    closeOnEsc: true,
    align: 'center',
    transition: 'm-modal',
  },
)

const emit = defineEmits<{
  (e: 'update:open', val: boolean): void
  (e: 'close'): void
}>()

const instanceId = getNextOverlayId()
let isRegistered = false

function handleClose() {
  emit('update:open', false)
  emit('close')
}

function registerOverlay() {
  if (isRegistered) return
  isRegistered = true
  overlayStack.push({
    id: instanceId,
    close: handleClose,
    get closeOnEsc() {
      return props.closeOnEsc
    },
  })
  lockBody()
}

function unregisterOverlay() {
  if (!isRegistered) return
  isRegistered = false
  const idx = overlayStack.findIndex((item) => item.id === instanceId)
  if (idx !== -1) {
    overlayStack.splice(idx, 1)
  }
  unlockBody()
}

watch(
  () => props.open,
  (val) => {
    if (val) {
      registerOverlay()
    } else {
      unregisterOverlay()
    }
  },
  { immediate: true },
)

onUnmounted(() => {
  unregisterOverlay()
})

const alignClass = computed(() => {
  switch (props.align) {
    case 'right':
      return 'flex items-stretch justify-end overflow-hidden'
    case 'left':
      return 'flex items-stretch justify-start overflow-hidden'
    case 'bottom':
      return 'flex items-end justify-center overflow-hidden'
    case 'center':
    default:
      return 'flex items-center justify-center p-4 overflow-y-auto'
  }
})

function onBackdropClick(e: MouseEvent) {
  if (e.target === e.currentTarget && props.closeOnBackdrop) {
    handleClose()
  }
}
</script>

<template>
  <Teleport to="body">
    <Transition :name="transition">
      <div
        v-if="open"
        class="fixed inset-0 bg-black/45"
        :class="[blur ? 'backdrop-blur-xs' : '', alignClass]"
        :style="{ zIndex }"
        @click.self="onBackdropClick"
      >
        <slot />
      </div>
    </Transition>
  </Teleport>
</template>

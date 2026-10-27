<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { computePointerPosition } from './floating'

export interface ContextMenuItem {
  key: string
  label: string
  icon?: string
  danger?: boolean
  disabled?: boolean
}

const props = withDefaults(
  defineProps<{
    items: ContextMenuItem[]
    open?: boolean
    modelValue?: boolean
    x: number
    y: number
  }>(),
  {
    open: false,
    modelValue: undefined,
    x: 0,
    y: 0,
  },
)

const menuRef = ref<HTMLElement | null>(null)
const size = ref<{ width: number; height: number }>({ width: 176, height: 0 })

/** Clamped viewport position: a right-click near an edge must not clip the menu. */
const menuStyle = computed<Record<string, string>>(() => {
  const { left, top, maxWidth, maxHeight } = computePointerPosition(
    { x: props.x, y: props.y },
    size.value,
  )
  return {
    left: `${left}px`,
    top: `${top}px`,
    '--origin': 'top left',
    ...(maxWidth !== undefined ? { maxWidth: `${maxWidth}px` } : {}),
    ...(maxHeight !== undefined ? { maxHeight: `${maxHeight}px` } : {}),
  }
})

watch(
  () => [props.open, props.x, props.y] as const,
  async ([open]) => {
    if (!open) return
    await nextTick()
    const el = menuRef.value
    if (el) size.value = { width: el.offsetWidth, height: el.offsetHeight }
  },
)

const emit = defineEmits<{
  (e: 'update:modelValue', val: boolean): void
  (e: 'select', key: string): void
}>()

function onItemClick(item: ContextMenuItem) {
  if (item.disabled) return
  emit('select', item.key)
  emit('update:modelValue', false)
}

function onClickOutside() {
  if (props.open) {
    emit('update:modelValue', false)
  }
}

onMounted(() => {
  window.addEventListener('click', onClickOutside)
  window.addEventListener('contextmenu', onClickOutside)
})

onUnmounted(() => {
  window.removeEventListener('click', onClickOutside)
  window.removeEventListener('contextmenu', onClickOutside)
})
</script>

<template>
  <Teleport to="body">
    <Transition name="m-pop">
      <div
        v-if="open"
        ref="menuRef"
        data-testid="m-context-menu"
        class="fixed z-50 m-overlay m-refract min-w-40 p-1 rounded-xl text-fg flex flex-col gap-0.5 select-none overflow-y-auto"
        :style="menuStyle"
        @click.stop
        @contextmenu.stop.prevent
      >
        <button
          v-for="item in items"
          :key="item.key"
          type="button"
          :disabled="item.disabled"
          class="flex items-center gap-2 px-3 py-1.5 text-xs rounded-lg transition-colors cursor-pointer text-left disabled:cursor-not-allowed disabled:opacity-45"
          :class="item.danger ? 'text-danger-fg hover:bg-danger-soft' : 'text-fg hover:bg-sunken'"
          @click="onItemClick(item)"
        >
          <span v-if="item.icon" :class="item.icon" class="w-3.5 h-3.5 shrink-0" />
          <span class="flex-1">{{ item.label }}</span>
        </button>
      </div>
    </Transition>
  </Teleport>
</template>

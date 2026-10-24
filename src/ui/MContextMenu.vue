<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'

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
    <div
      v-if="open"
      class="fixed z-50 m-overlay m-refract min-w-40 p-1 rounded-xl text-fg flex flex-col gap-0.5 select-none"
      :style="{ left: `${x}px`, top: `${y}px` }"
      @click.stop
      @contextmenu.stop.prevent
    >
      <button
        v-for="item in items"
        :key="item.key"
        type="button"
        :disabled="item.disabled"
        class="flex items-center gap-2 px-3 py-1.5 text-xs rounded-lg transition-colors cursor-pointer text-left disabled:cursor-not-allowed disabled:opacity-45"
        :class="
          item.danger
            ? 'text-danger-fg hover:bg-danger-soft'
            : 'text-fg hover:bg-sunken'
        "
        @click="onItemClick(item)"
      >
        <span v-if="item.icon" :class="item.icon" class="w-3.5 h-3.5 shrink-0" />
        <span class="flex-1">{{ item.label }}</span>
      </button>
    </div>
  </Teleport>
</template>

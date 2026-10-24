<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(
  defineProps<{
    icon: string
    variant?: 'text' | 'tonal' | 'outline' | 'danger'
    title?: string
    tooltip?: string
    loading?: boolean
    disabled?: boolean
    size?: 'sm' | 'md' | 'lg'
  }>(),
  {
    variant: 'text',
    title: undefined,
    tooltip: undefined,
    loading: false,
    disabled: false,
    size: 'md',
  },
)

defineEmits<{
  (e: 'click', evt: MouseEvent): void
}>()

const sizeClass = computed(() => {
  switch (props.size) {
    case 'sm':
      return 'w-7 h-7'
    case 'lg':
      return 'w-11 h-11'
    case 'md':
    default:
      return 'w-9 h-9'
  }
})

const iconSizeClass = computed(() => {
  switch (props.size) {
    case 'sm':
      return 'w-4 h-4'
    case 'lg':
      return 'w-5 h-5'
    case 'md':
    default:
      return 'w-[18px] h-[18px]'
  }
})
</script>

<template>
  <button
    type="button"
    :title="tooltip ?? title"
    :disabled="disabled || loading"
    class="inline-flex items-center justify-center rounded-lg transition active:scale-[.98] cursor-pointer disabled:cursor-not-allowed disabled:opacity-45 select-none outline-none"
    :class="[
      sizeClass,
      {
        'text-fg hover:bg-sunken': variant === 'text',
        'bg-primary-soft text-primary-soft-fg hover:brightness-[.97]': variant === 'tonal',
        'border border-line-strong text-fg hover:bg-sunken': variant === 'outline',
        'text-danger hover:bg-danger-soft': variant === 'danger',
      },
    ]"
    @click="$emit('click', $event)"
  >
    <span
      v-if="loading"
      class="inline-block border-2 border-current border-t-transparent rounded-full animate-spin"
      :class="iconSizeClass"
    />
    <span v-else :class="[icon, iconSizeClass]" />
  </button>
</template>

<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(
  defineProps<{
    variant?: 'primary' | 'tonal' | 'outline' | 'text' | 'danger'
    size?: 'xs' | 'sm' | 'md' | 'lg'
    icon?: string
    iconRight?: string
    block?: boolean
    loading?: boolean
    disabled?: boolean
    type?: 'button' | 'submit' | 'reset'
  }>(),
  {
    variant: 'primary',
    size: 'md',
    icon: undefined,
    iconRight: undefined,
    block: false,
    loading: false,
    disabled: false,
    type: 'button',
  },
)

defineEmits<{
  (e: 'click', evt: MouseEvent): void
}>()

const sizeClass = computed(() => {
  switch (props.size) {
    case 'xs':
      return 'h-6 px-2 text-xs gap-1.5'
    case 'sm':
      return 'h-8 px-3 text-sm gap-1.5'
    case 'lg':
      return 'h-12 px-6 text-base gap-2'
    case 'md':
    default:
      return 'h-10 px-4 text-sm gap-2'
  }
})

const iconSizeClass = computed(() => {
  switch (props.size) {
    case 'xs':
      return 'w-3.5 h-3.5'
    case 'sm':
      return 'w-4 h-4'
    case 'lg':
      return 'w-5 h-5'
    case 'md':
    default:
      return 'w-4 h-4'
  }
})
</script>

<template>
  <button
    :type="type"
    :disabled="disabled || loading"
    class="inline-flex items-center justify-center font-medium rounded-lg transition active:scale-[.98] cursor-pointer disabled:cursor-not-allowed disabled:opacity-45 select-none outline-none shrink-0 whitespace-nowrap"
    :class="[
      sizeClass,
      block ? 'w-full' : '',
      {
        'm-btn-primary bg-primary text-primary-on hover:bg-primary-hover shadow-1':
          variant === 'primary',
        'bg-primary-soft text-primary-soft-fg hover:brightness-[.97]': variant === 'tonal',
        'border border-line-strong text-fg hover:bg-sunken': variant === 'outline',
        'bg-transparent text-fg hover:bg-sunken': variant === 'text',
        'bg-danger text-primary-on hover:brightness-[.95]': variant === 'danger',
      },
    ]"
    @click="$emit('click', $event)"
  >
    <span
      v-if="loading"
      class="inline-block border-2 border-current border-t-transparent rounded-full animate-spin shrink-0"
      :class="iconSizeClass"
    />
    <slot v-else-if="$slots.icon" name="icon" />
    <span v-else-if="icon" :class="[icon, iconSizeClass]" class="shrink-0" />
    <slot />
    <span v-if="iconRight" :class="[iconRight, iconSizeClass]" class="shrink-0" />
  </button>
</template>

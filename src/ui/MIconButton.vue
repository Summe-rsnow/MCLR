<script setup lang="ts">
import { computed } from 'vue'
import { Button } from '@miuui/vue'
import type { ButtonVariant } from '@miuui/vue'

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

const emit = defineEmits<{
  (e: 'click', evt: MouseEvent): void
}>()

const miuVariant = computed<ButtonVariant>(() => {
  switch (props.variant) {
    case 'tonal':
      return 'tonal'
    case 'outline':
      return 'outline'
    case 'danger':
      return 'danger'
    case 'text':
    default:
      return 'ghost'
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
  <Button
    icon-only
    :variant="miuVariant"
    :size="size"
    :loading="loading"
    :disabled="disabled"
    :title="tooltip ?? title"
    :aria-label="tooltip ?? title ?? icon"
    radius="md"
    :class-names="{ spinner: 'animate-spin' }"
    @click="emit('click', $event)"
  >
    <span :class="[icon, iconSizeClass]" />
  </Button>
</template>

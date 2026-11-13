<script setup lang="ts">
import { computed } from 'vue'
import { Badge } from '@miuui/vue'

const props = withDefaults(
  defineProps<{
    variant?: 'primary' | 'success' | 'warning' | 'danger' | 'neutral' | 'info'
    size?: 'sm' | 'md'
    icon?: string
    dot?: boolean
    pulse?: boolean
  }>(),
  {
    variant: 'primary',
    size: 'md',
    icon: undefined,
    dot: false,
    pulse: false,
  },
)

const variantTextClass = computed(() => {
  switch (props.variant) {
    case 'success':
      return 'text-success-fg'
    case 'warning':
      return 'text-warning-fg'
    case 'danger':
      return 'text-danger-fg'
    case 'neutral':
      return 'text-fg-muted'
    case 'primary':
    case 'info':
    default:
      return 'text-primary-soft-fg'
  }
})
</script>

<template>
  <Badge
    :variant="variant"
    :size="size"
    :dot="dot"
    :pulse="pulse"
    pill
    :class-names="{ root: variantTextClass }"
  >
    <span v-if="icon" :class="icon" class="w-3 h-3 shrink-0" />
    <slot />
  </Badge>
</template>

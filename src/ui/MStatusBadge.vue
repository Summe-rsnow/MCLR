<script setup lang="ts">
import { computed } from 'vue'
import { Badge } from '@miuui/vue'

export type StatusType =
  | 'online'
  | 'offline'
  | 'running'
  | 'waiting'
  | 'warning'
  | 'danger'
  | 'success'

const props = withDefaults(
  defineProps<{
    status: StatusType
    size?: 'sm' | 'md'
    label?: string
    pulse?: boolean
  }>(),
  {
    size: 'md',
    label: undefined,
    pulse: false,
  },
)

const badgeVariant = computed(() => {
  switch (props.status) {
    case 'online':
    case 'success':
      return 'success'
    case 'running':
      return 'primary'
    case 'warning':
      return 'warning'
    case 'danger':
      return 'danger'
    case 'offline':
    case 'waiting':
    default:
      return 'neutral'
  }
})
</script>

<template>
  <Badge :variant="badgeVariant" :size="size" :dot="!pulse" :pulse="pulse" pill>
    <slot>{{ label || status }}</slot>
  </Badge>
</template>

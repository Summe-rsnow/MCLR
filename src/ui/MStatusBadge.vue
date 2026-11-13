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
    material?: 'solid' | 'mica' | 'glass'
    label?: string
    pulse?: boolean
  }>(),
  {
    size: 'md',
    material: undefined,
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

const sizeClass = computed(() =>
  props.size === 'sm'
    ? 'h-5 px-2 text-[11px] leading-none'
    : 'h-[22px] px-2.5 text-xs leading-none',
)
</script>

<template>
  <Badge
    :variant="badgeVariant"
    :size="size"
    :material="material"
    :dot="!pulse"
    :pulse="pulse"
    pill
    :class-names="{ root: sizeClass }"
  >
    <slot>{{ label || status }}</slot>
  </Badge>
</template>

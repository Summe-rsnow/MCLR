<script setup lang="ts">
import { computed } from 'vue'
import MBadge from './MBadge.vue'

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
  <MBadge :variant="badgeVariant" :size="size" :dot="!pulse">
    <span v-if="pulse" class="relative flex h-2 w-2 mr-0.5">
      <span
        class="animate-ping absolute inline-flex h-full w-full rounded-full opacity-75 bg-current"
      />
      <span class="relative inline-flex rounded-full h-2 w-2 bg-current" />
    </span>
    <slot>{{ label || status }}</slot>
  </MBadge>
</template>

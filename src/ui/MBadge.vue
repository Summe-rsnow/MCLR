<script setup lang="ts">
import { computed } from 'vue'
import { Badge } from '@miuui/vue'

const props = withDefaults(
  defineProps<{
    variant?: 'primary' | 'success' | 'warning' | 'danger' | 'neutral' | 'info'
    size?: 'sm' | 'md'
    material?: 'solid' | 'mica' | 'glass'
    icon?: string
    dot?: boolean
    pulse?: boolean
  }>(),
  {
    variant: 'primary',
    size: 'md',
    material: undefined,
    icon: undefined,
    dot: false,
    pulse: false,
  },
)

const sizeClass = computed(() =>
  props.size === 'sm'
    ? 'h-5 px-2 text-[11px] leading-none'
    : 'h-[22px] px-2.5 text-xs leading-none',
)
</script>

<template>
  <Badge
    :variant="variant"
    :size="size"
    :material="material"
    :dot="dot"
    :pulse="pulse"
    pill
    :class-names="{ root: sizeClass }"
  >
    <span v-if="icon" :class="icon" class="w-3 h-3 shrink-0" />
    <slot />
  </Badge>
</template>

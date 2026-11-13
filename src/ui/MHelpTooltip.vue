<script setup lang="ts">
import { computed } from 'vue'
import { HelpTooltip } from '@miuui/vue'

const props = withDefaults(
  defineProps<{
    text: string
    icon?: string
    size?: 'xs' | 'sm' | 'md'
    material?: 'solid' | 'mica' | 'glass'
  }>(),
  {
    icon: 'i-lucide-circle-question-mark',
    size: 'sm',
    material: undefined,
  },
)

const sizeClasses = computed(() => {
  switch (props.size) {
    case 'xs':
      return 'w-3 h-3'
    case 'md':
      return 'w-4.5 h-4.5'
    case 'sm':
    default:
      return 'w-3.5 h-3.5'
  }
})
</script>

<template>
  <HelpTooltip
    :text="text"
    :size="size"
    :material="material"
    max-width="280px"
    :class-names="{ trigger: 'cursor-help' }"
  >
    <template v-if="icon.startsWith('i-')" #icon>
      <span :class="[icon, sizeClasses]" class="shrink-0" />
    </template>
  </HelpTooltip>
</template>

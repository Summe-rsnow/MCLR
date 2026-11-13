<script setup lang="ts">
import { computed } from 'vue'
import { HelpTooltip } from '@miuui/vue'

const props = withDefaults(
  defineProps<{
    text: string
    icon?: string
    size?: 'xs' | 'sm' | 'md'
  }>(),
  {
    icon: 'i-lucide-circle-question-mark',
    size: 'sm',
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
    max-width="280px"
    :class-names="{ trigger: 'cursor-help' }"
  >
    <template v-if="icon.startsWith('i-')" #icon>
      <span :class="[icon, sizeClasses]" class="shrink-0" />
    </template>
  </HelpTooltip>
</template>

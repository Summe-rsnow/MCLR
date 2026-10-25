<script setup lang="ts">
import { computed } from 'vue'
import MTooltip from './MTooltip.vue'

const props = withDefaults(
  defineProps<{
    text: string
    icon?: string
    size?: 'xs' | 'sm' | 'md'
  }>(),
  {
    icon: 'i-lucide-help-circle',
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
  <MTooltip :text="text">
    <button
      type="button"
      class="inline-flex items-center justify-center text-fg-muted hover:text-fg focus:text-fg transition-colors p-0.5 rounded cursor-help outline-none focus-visible:ring-1 focus-visible:ring-primary"
      :aria-label="text"
      tabindex="0"
      @click.stop.prevent
    >
      <span :class="[icon, sizeClasses]" class="shrink-0" />
    </button>
  </MTooltip>
</template>

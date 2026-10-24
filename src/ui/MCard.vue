<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(
  defineProps<{
    variant?: 'raised' | 'well' | 'ghost' | 'tinted'
    padding?: 'none' | 'sm' | 'md' | 'lg'
    hoverable?: boolean
  }>(),
  {
    variant: 'raised',
    padding: 'md',
    hoverable: false,
  },
)

const paddingClass = computed(() => {
  switch (props.padding) {
    case 'none':
      return 'p-0'
    case 'sm':
      return 'p-3'
    case 'lg':
      return 'p-6'
    case 'md':
    default:
      return 'p-5'
  }
})

const variantClass = computed(() => {
  switch (props.variant) {
    case 'well':
      return 'm-well'
    case 'ghost':
      return 'bg-transparent'
    case 'tinted':
      return 'bg-primary-soft text-fg border border-primary/15'
    case 'raised':
    default:
      return 'm-card'
  }
})
</script>

<template>
  <div
    class="rounded-xl transition-all duration-200"
    :class="[
      variantClass,
      paddingClass,
      hoverable ? 'hover:-translate-y-[1px] hover:shadow-2 cursor-pointer' : '',
    ]"
  >
    <slot />
  </div>
</template>

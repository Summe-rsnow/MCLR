<script setup lang="ts">
import { computed } from 'vue'
import MSkeleton from './MSkeleton.vue'

const props = withDefaults(
  defineProps<{
    variant?: 'raised' | 'well' | 'ghost' | 'tinted'
    padding?: 'none' | 'sm' | 'md' | 'lg'
    hoverable?: boolean
    loading?: boolean
    skeletonLines?: number
  }>(),
  {
    variant: 'raised',
    padding: 'md',
    hoverable: false,
    loading: false,
    skeletonLines: 3,
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
      hoverable ? 'm-card-hover hover:-translate-y-[1px] hover:shadow-2 cursor-pointer' : '',
    ]"
  >
    <div v-if="loading" class="space-y-3">
      <MSkeleton height="1.25rem" width="40%" rounded="lg" />
      <MSkeleton
        v-for="i in Math.max(0, skeletonLines - 1)"
        :key="i"
        height="0.875rem"
        :width="i === skeletonLines - 1 ? '60%' : '100%'"
        rounded="md"
      />
    </div>
    <slot v-else />
  </div>
</template>

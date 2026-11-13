<script setup lang="ts">
import { computed } from 'vue'
import { Card } from '@miuui/vue'
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

const miuVariant = computed(() => {
  if (props.variant === 'ghost') return 'ghost'
  if (props.variant === 'well' || props.variant === 'tinted') return 'outline'
  return 'surface'
})

const rootVariantClass = computed(() => {
  switch (props.variant) {
    case 'well':
      return 'm-well'
    case 'ghost':
      return 'bg-transparent border-transparent shadow-none'
    case 'tinted':
      return 'bg-primary-soft text-fg border-primary/15 shadow-none'
    case 'raised':
    default:
      return 'm-card'
  }
})
</script>

<template>
  <Card
    :variant="miuVariant"
    :padding="padding"
    :interactive="hoverable"
    radius="lg"
    :class-names="{
      root: [rootVariantClass, hoverable ? 'm-card-hover' : ''].filter(Boolean).join(' '),
    }"
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
  </Card>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { Skeleton } from '@miuui/vue'

const props = withDefaults(
  defineProps<{
    width?: string
    height?: string
    rounded?: boolean | string
  }>(),
  {
    width: undefined,
    height: undefined,
    rounded: true,
  },
)

const roundedClass = computed(() => {
  if (props.rounded === false) return ''
  if (typeof props.rounded === 'string') return `rounded-${props.rounded}`
  return 'rounded-xl'
})
</script>

<template>
  <Skeleton
    :width="width"
    :height="height"
    :radius="rounded === false ? 'none' : 'md'"
    :class-names="{
      root: ['m-skeleton', roundedClass, !width ? 'w-full' : '', !height ? 'h-4' : '']
        .filter(Boolean)
        .join(' '),
    }"
  />
</template>

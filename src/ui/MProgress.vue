<script setup lang="ts">
import { Progress } from '@miuui/vue'

withDefaults(
  defineProps<{
    value?: number
    max?: number
    size?: 'sm' | 'md'
    indeterminate?: boolean
    showLabel?: boolean
  }>(),
  {
    value: 0,
    max: 1,
    size: 'md',
    indeterminate: false,
    showLabel: false,
  },
)
</script>

<template>
  <Progress
    :model-value="value"
    :max="max"
    :size="size"
    :indeterminate="indeterminate"
    :show-value="showLabel"
    :class-names="{
      indicator: indeterminate ? 'm-progress-indeterminate' : undefined,
      label: 'tabular-nums',
    }"
  />
</template>

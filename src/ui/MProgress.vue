<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(
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

const percentage = computed(() => {
  const denom = props.max > 0 ? props.max : 1
  const ratio = props.value / denom
  return Math.min(100, Math.max(0, Math.round(ratio * 100)))
})
</script>

<template>
  <div class="flex items-center gap-2.5 w-full select-none">
    <div
      class="relative w-full rounded-full bg-sunken overflow-hidden"
      :class="size === 'sm' ? 'h-1.5' : 'h-2.5'"
    >
      <div
        v-if="!indeterminate"
        class="m-progress-fill h-full rounded-full transition-all duration-200 ease-out"
        :style="{ width: `${percentage}%` }"
      />
      <div
        v-else
        class="m-progress-fill h-full rounded-full w-1/3 animate-indeterminate"
      />
    </div>
    <span
      v-if="showLabel && !indeterminate"
      class="text-xs text-fg-muted w-9 text-right tabular-nums shrink-0"
    >
      {{ percentage }}%
    </span>
  </div>
</template>

<style scoped>
.m-progress-fill {
  background: linear-gradient(90deg, rgb(var(--c-primary)), rgb(var(--c-primary-hover)));
  box-shadow: inset 0 1px 0 rgb(255 255 255 / 0.28);
}

@keyframes indeterminate {
  0% {
    transform: translateX(-100%);
  }
  100% {
    transform: translateX(300%);
  }
}

.animate-indeterminate {
  animation: indeterminate 1.4s infinite ease-in-out;
}
</style>

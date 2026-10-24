<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(
  defineProps<{
    value?: number // 0..1
    indeterminate?: boolean
    showLabel?: boolean
  }>(),
  {
    value: 0,
    indeterminate: false,
    showLabel: false,
  },
)

const percentage = computed(() => {
  return Math.min(100, Math.max(0, Math.round(props.value * 100)))
})
</script>

<template>
  <div class="flex items-center gap-2 w-full select-none">
    <div class="relative w-full h-2 rounded-full bg-surfaceVariant overflow-hidden">
      <div
        v-if="!indeterminate"
        class="h-full bg-primary rounded-full transition-all duration-200 ease-out"
        :style="{ width: `${percentage}%` }"
      />
      <div
        v-else
        class="h-full bg-primary rounded-full w-1/3 animate-indeterminate"
      />
    </div>
    <span v-if="showLabel && !indeterminate" class="text-xs text-textMuted w-9 text-right tabular-nums">
      {{ percentage }}%
    </span>
  </div>
</template>

<style scoped>
@keyframes indeterminate {
  0% {
    transform: translateX(-100%);
  }
  100% {
    transform: translateX(300%);
  }
}
.animate-indeterminate {
  animation: indeterminate 1.5s infinite ease-in-out;
}
</style>

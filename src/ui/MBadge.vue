<script setup lang="ts">
withDefaults(
  defineProps<{
    variant?: 'primary' | 'success' | 'warning' | 'danger' | 'neutral'
  }>(),
  {
    variant: 'primary',
  },
)
</script>

<template>
  <span
    class="inline-flex items-center px-2 py-0.5 text-xs font-medium rounded-full select-none"
    :class="{
      'bg-primaryContainer text-primary font-semibold': variant === 'primary',
      'bg-success/15 text-success': variant === 'success',
      'bg-warning/15 text-warning': variant === 'warning',
      'bg-danger/15 text-danger': variant === 'danger',
      'bg-surfaceVariant text-textMuted': variant === 'neutral',
    }"
  >
    <slot />
  </span>
</template>

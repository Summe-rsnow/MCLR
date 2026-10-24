<script setup lang="ts">
withDefaults(
  defineProps<{
    variant?: 'primary' | 'success' | 'warning' | 'danger' | 'neutral' | 'info'
    size?: 'sm' | 'md'
    icon?: string
    dot?: boolean
  }>(),
  {
    variant: 'primary',
    size: 'md',
    icon: undefined,
    dot: false,
  },
)
</script>

<template>
  <span
    class="inline-flex items-center gap-1 font-medium rounded-full select-none shrink-0"
    :class="[
      size === 'sm' ? 'px-2 py-0.5 text-[11px]' : 'px-2.5 py-0.5 text-xs',
      {
        'bg-primary-soft text-primary-soft-fg': variant === 'primary' || variant === 'info',
        'bg-success-soft text-success-fg': variant === 'success',
        'bg-warning-soft text-warning-fg': variant === 'warning',
        'bg-danger-soft text-danger-fg': variant === 'danger',
        'bg-sunken text-fg-muted': variant === 'neutral',
      },
    ]"
  >
    <span v-if="dot" class="w-1.5 h-1.5 rounded-full bg-current shrink-0" />
    <span v-if="icon" :class="icon" class="w-3 h-3 shrink-0" />
    <slot />
  </span>
</template>

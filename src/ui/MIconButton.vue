<script setup lang="ts">
withDefaults(
  defineProps<{
    icon: string
    title?: string
    disabled?: boolean
    size?: 'sm' | 'md' | 'lg'
  }>(),
  {
    title: undefined,
    disabled: false,
    size: 'md',
  },
)

defineEmits<{
  (e: 'click', evt: MouseEvent): void
}>()
</script>

<template>
  <button
    type="button"
    :title="title"
    :disabled="disabled"
    class="inline-flex items-center justify-center rounded-lg transition-colors cursor-pointer text-text hover:bg-surfaceVariant/60 active:bg-surfaceVariant disabled:cursor-not-allowed disabled:opacity-50 select-none outline-none focus-visible:ring-2 focus-visible:ring-primary/50"
    :class="{
      'w-7 h-7 text-xs': size === 'sm',
      'w-9 h-9 text-sm': size === 'md',
      'w-11 h-11 text-base': size === 'lg',
    }"
    @click="$emit('click', $event)"
  >
    <span :class="icon" />
  </button>
</template>

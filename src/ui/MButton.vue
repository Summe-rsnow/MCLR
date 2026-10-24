<script setup lang="ts">
withDefaults(
  defineProps<{
    variant?: 'primary' | 'tonal' | 'text' | 'danger'
    loading?: boolean
    disabled?: boolean
    type?: 'button' | 'submit' | 'reset'
  }>(),
  {
    variant: 'primary',
    loading: false,
    disabled: false,
    type: 'button',
  },
)

defineEmits<{
  (e: 'click', evt: MouseEvent): void
}>()
</script>

<template>
  <button
    :type="type"
    :disabled="disabled || loading"
    class="inline-flex items-center justify-center gap-2 px-4 py-2 text-sm font-medium rounded-lg transition-colors cursor-pointer disabled:cursor-not-allowed disabled:opacity-50 select-none outline-none focus-visible:ring-2 focus-visible:ring-primary/50"
    :class="{
      'bg-primary text-onPrimary hover:opacity-90 active:opacity-95': variant === 'primary',
      'bg-primaryContainer text-text hover:opacity-90 active:opacity-95': variant === 'tonal',
      'bg-transparent text-text hover:bg-surfaceVariant/60 active:bg-surfaceVariant': variant === 'text',
      'bg-danger text-white hover:opacity-90 active:opacity-95': variant === 'danger',
    }"
    @click="$emit('click', $event)"
  >
    <span v-if="loading" class="inline-block w-4 h-4 border-2 border-current border-t-transparent rounded-full animate-spin" />
    <slot />
  </button>
</template>

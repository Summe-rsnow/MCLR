<script setup lang="ts">
withDefaults(
  defineProps<{
    title?: string
    subtitle?: string
    active?: boolean
    clickable?: boolean
    disabled?: boolean
  }>(),
  {
    title: undefined,
    subtitle: undefined,
    active: false,
    clickable: true,
    disabled: false,
  },
)

defineEmits<{
  (e: 'click', evt: MouseEvent): void
}>()
</script>

<template>
  <div
    class="flex items-center justify-between gap-3 px-3 py-2.5 rounded-lg transition-colors select-none"
    :class="[
      clickable && !disabled ? 'cursor-pointer hover:bg-surfaceVariant/60 active:bg-surfaceVariant' : '',
      active ? 'bg-primaryContainer/70 text-primary font-medium' : 'text-text',
      disabled ? 'opacity-50 cursor-not-allowed' : '',
    ]"
    @click="!disabled && $emit('click', $event)"
  >
    <div class="flex items-center gap-3 min-w-0">
      <slot name="prefix" />
      <div class="flex flex-col min-w-0">
        <span v-if="title" class="text-sm truncate">{{ title }}</span>
        <slot v-else />
        <span v-if="subtitle" class="text-xs text-textMuted truncate">{{ subtitle }}</span>
      </div>
    </div>
    <div class="flex items-center gap-2">
      <slot name="suffix" />
    </div>
  </div>
</template>

<script setup lang="ts">
import MButton from './MButton.vue'

withDefaults(
  defineProps<{
    title?: string
    description?: string
    icon?: string
    actionText?: string
  }>(),
  {
    title: undefined,
    description: undefined,
    icon: 'i-lucide-inbox',
    actionText: undefined,
  },
)

const emit = defineEmits<{
  (e: 'action'): void
}>()
</script>

<template>
  <div class="flex flex-col items-center justify-center p-8 text-center select-none">
    <div
      class="w-12 h-12 rounded-2xl bg-sunken border border-line flex items-center justify-center text-fg-subtle mb-3"
    >
      <span :class="icon" class="w-6 h-6" />
    </div>
    <h4 v-if="title" class="text-sm font-medium text-fg mb-1">{{ title }}</h4>
    <p v-if="description" class="text-xs text-fg-muted max-w-xs select-text">{{ description }}</p>
    <div
      v-if="$slots.default || $slots.action || actionText"
      class="mt-4 flex items-center justify-center gap-2"
    >
      <slot />
      <slot name="action">
        <MButton v-if="actionText" variant="primary" size="sm" @click="emit('action')">
          {{ actionText }}
        </MButton>
      </slot>
    </div>
  </div>
</template>

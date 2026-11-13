<script setup lang="ts">
import { EmptyState } from '@miuui/vue'
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
  <EmptyState
    :title="title ?? ''"
    :description="description"
    :class-names="{ description: 'select-text' }"
  >
    <template #icon>
      <div
        class="w-12 h-12 rounded-2xl bg-sunken border border-line flex items-center justify-center text-fg-subtle"
      >
        <span :class="icon" class="w-6 h-6" />
      </div>
    </template>
    <template v-if="$slots.default || $slots.action || actionText" #actions>
      <slot />
      <slot name="action">
        <MButton v-if="actionText" variant="primary" size="sm" @click="emit('action')">
          {{ actionText }}
        </MButton>
      </slot>
    </template>
  </EmptyState>
</template>

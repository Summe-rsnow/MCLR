<script setup lang="ts">
import MCard from './MCard.vue'

withDefaults(
  defineProps<{
    title?: string
    description?: string
    icon?: string
  }>(),
  {
    title: undefined,
    description: undefined,
    icon: undefined,
  },
)
</script>

<template>
  <MCard padding="none">
    <div
      v-if="title || description || icon || $slots.actions"
      class="flex items-center justify-between gap-4 px-5 pt-4 pb-3 border-b border-line"
    >
      <div class="flex items-center gap-2.5 min-w-0">
        <span v-if="icon" :class="icon" class="w-4 h-4 text-primary shrink-0" />
        <div class="min-w-0">
          <h2 v-if="title || $slots.title" class="text-base font-medium text-fg flex items-center gap-1.5 truncate">
            <slot name="title">{{ title }}</slot>
          </h2>
          <p v-if="description || $slots.description" class="text-xs text-fg-muted mt-0.5">
            <slot name="description">{{ description }}</slot>
          </p>
        </div>
      </div>
      <div v-if="$slots.actions" class="flex items-center gap-2 shrink-0">
        <slot name="actions" />
      </div>
    </div>
    <div class="p-5">
      <slot />
    </div>
  </MCard>
</template>

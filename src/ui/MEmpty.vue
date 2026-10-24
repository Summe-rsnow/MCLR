<script setup lang="ts">
withDefaults(
  defineProps<{
    title?: string
    description?: string
    icon?: string
  }>(),
  {
    title: undefined,
    description: undefined,
    icon: 'i-lucide-inbox',
  },
)
</script>

<template>
  <div class="flex flex-col items-center justify-center p-8 text-center select-none">
    <div class="w-12 h-12 rounded-2xl bg-surfaceVariant flex items-center justify-center text-textMuted mb-3">
      <span :class="icon" class="w-6 h-6" />
    </div>
    <h4 v-if="title" class="text-sm font-semibold text-text mb-1">{{ title }}</h4>
    <p v-if="description" class="text-xs text-textMuted max-w-xs">{{ description }}</p>
    <div v-if="$slots.default" class="mt-4">
      <slot />
    </div>
  </div>
</template>

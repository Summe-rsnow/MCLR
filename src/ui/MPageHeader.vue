<script setup lang="ts">
withDefaults(
  defineProps<{
    title: string
    description?: string
    icon?: string
  }>(),
  {
    description: undefined,
    icon: undefined,
  },
)
</script>

<template>
  <header class="flex items-center justify-between gap-4 flex-wrap">
    <div class="flex items-center gap-3.5 min-w-0">
      <div
        v-if="icon"
        class="w-10 h-10 rounded-xl bg-primary-soft text-primary-soft-fg flex items-center justify-center shrink-0"
      >
        <span :class="icon" class="w-5 h-5" />
      </div>
      <div class="min-w-0">
        <h1 class="text-2xl font-bold text-fg tracking-tight truncate">{{ title }}</h1>
        <p v-if="description" class="text-xs text-fg-muted mt-0.5">{{ description }}</p>
      </div>
    </div>
    <div v-if="$slots.actions" class="flex items-center gap-2 shrink-0">
      <slot name="actions" />
    </div>
  </header>
</template>

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
    <div class="flex items-center gap-4 min-w-0">
      <slot name="icon">
        <div
          v-if="icon"
          class="w-12 h-12 rounded-xl bg-primary-soft text-primary-soft-fg flex items-center justify-center shrink-0 shadow-sm"
        >
          <span :class="icon" class="w-6 h-6" />
        </div>
      </slot>
      <div class="min-w-0">
        <div class="flex items-center gap-2.5 flex-wrap">
          <h1 class="text-3xl font-extrabold text-fg tracking-tight truncate">{{ title }}</h1>
          <slot name="badge" />
        </div>
        <p v-if="description" class="text-xs text-fg-muted mt-1 leading-normal">
          {{ description }}
        </p>
      </div>
    </div>
    <div v-if="$slots.actions" class="flex items-center gap-2 shrink-0">
      <slot name="actions" />
    </div>
  </header>
</template>

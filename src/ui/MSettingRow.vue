<script setup lang="ts">
withDefaults(
  defineProps<{
    label: string
    hint?: string
  }>(),
  {
    hint: undefined,
  },
)
</script>

<template>
  <div
    class="m-setting-row flex flex-col sm:flex-row sm:items-center sm:justify-between gap-2 min-h-[52px] py-2.5"
  >
    <div class="min-w-0 pr-4">
      <div class="text-sm font-medium text-fg">{{ label }}</div>
      <div v-if="hint" class="text-xs text-fg-muted mt-0.5">{{ hint }}</div>
    </div>
    <div class="flex items-center gap-2 shrink-0">
      <slot />
    </div>
  </div>
</template>

<style scoped>
.m-setting-row + .m-setting-row {
  border-top: 1px solid rgb(var(--c-line));
}
</style>

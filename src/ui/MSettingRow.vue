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
    class="m-setting-row flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3 min-h-[52px] py-2.5"
  >
    <div class="min-w-[140px] flex-1 pr-4">
      <div class="text-sm font-medium text-fg flex items-center gap-1.5">
        <slot name="label">{{ label }}</slot>
      </div>
      <div v-if="hint || $slots.hint" class="text-xs text-fg-muted mt-0.5">
        <slot name="hint">{{ hint }}</slot>
      </div>
    </div>
    <div class="flex items-center gap-2 max-w-full flex-wrap sm:justify-end shrink-0 sm:shrink">
      <slot />
    </div>
  </div>
</template>

<style scoped>
.m-setting-row + .m-setting-row {
  border-top: 1px solid rgb(var(--c-line));
}
</style>

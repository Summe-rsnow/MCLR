<script setup lang="ts">
withDefaults(
  defineProps<{
    label: string
    hint?: string
    /** Let the control area take the remaining width (full-width controls). */
    stretch?: boolean
  }>(),
  {
    hint: undefined,
    stretch: false,
  },
)
</script>

<template>
  <div
    class="m-setting-row flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3 min-h-[52px] py-2.5"
  >
    <div class="min-w-[140px] pr-4" :class="stretch ? 'shrink-0' : 'flex-1'">
      <div class="text-sm font-medium text-fg flex items-center gap-1.5">
        <slot name="label">{{ label }}</slot>
      </div>
      <div v-if="hint || $slots.hint" class="text-xs text-fg-muted mt-0.5">
        <slot name="hint">{{ hint }}</slot>
      </div>
    </div>
    <!-- `min-w-0` lets a full-width control shrink instead of overflowing the row. -->
    <div
      class="flex items-center gap-2 max-w-full min-w-0 flex-wrap sm:justify-end"
      :class="stretch ? 'flex-1' : 'shrink-0 sm:shrink'"
    >
      <slot />
    </div>
  </div>
</template>

<style scoped>
.m-setting-row + .m-setting-row {
  border-top: 1px solid rgb(var(--c-line));
}
</style>

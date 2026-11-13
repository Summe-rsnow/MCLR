<script setup lang="ts">
import { FormRow } from '@miuui/vue'

withDefaults(
  defineProps<{
    label: string
    hint?: string
    stretch?: boolean
  }>(),
  {
    hint: undefined,
    stretch: false,
  },
)
</script>

<template>
  <FormRow
    :label="label"
    :hint="hint"
    :class-names="{
      root: 'm-setting-row min-h-[52px] py-2.5',
      control: ['flex items-center gap-2 max-w-full min-w-0 flex-wrap', stretch ? 'flex-1' : '']
        .filter(Boolean)
        .join(' '),
    }"
  >
    <template #lead>
      <div class="min-w-[140px] pr-4" :class="stretch ? 'shrink-0' : 'flex-1'">
        <div class="text-sm font-medium text-fg flex items-center gap-1.5">
          <slot name="label">{{ label }}</slot>
        </div>
        <div v-if="hint || $slots.hint" class="text-xs text-fg-muted mt-0.5">
          <slot name="hint">{{ hint }}</slot>
        </div>
      </div>
    </template>
    <slot />
  </FormRow>
</template>

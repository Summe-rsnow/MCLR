<script setup lang="ts">
import { Checkbox } from '@miuui/vue'

withDefaults(
  defineProps<{
    modelValue: boolean
    label?: string
    disabled?: boolean
  }>(),
  {
    label: undefined,
    disabled: false,
  },
)

const emit = defineEmits<{
  (e: 'update:modelValue', val: boolean): void
}>()
</script>

<template>
  <Checkbox
    :model-value="modelValue"
    :label="label"
    :disabled="disabled"
    :class-names="{
      root: disabled ? 'cursor-not-allowed' : 'cursor-pointer',
      label: disabled ? 'cursor-not-allowed text-fg-subtle' : 'cursor-pointer text-fg',
    }"
    @update:model-value="emit('update:modelValue', $event)"
  >
    <slot />
  </Checkbox>
</template>

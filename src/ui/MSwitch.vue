<script setup lang="ts">
import { Switch } from '@miuui/vue'

withDefaults(
  defineProps<{
    modelValue: boolean
    disabled?: boolean
    label?: string
    size?: 'sm' | 'md'
  }>(),
  {
    disabled: false,
    label: undefined,
    size: 'md',
  },
)

const emit = defineEmits<{
  (e: 'update:modelValue', val: boolean): void
}>()
</script>

<template>
  <Switch
    :model-value="modelValue"
    :disabled="disabled"
    :label="label"
    :size="size"
    @update:model-value="emit('update:modelValue', $event)"
  />
</template>

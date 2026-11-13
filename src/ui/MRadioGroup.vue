<script setup lang="ts">
import { RadioGroup } from '@miuui/vue'

withDefaults(
  defineProps<{
    modelValue: string | number
    options: Array<{ label: string; value: string | number; disabled?: boolean }>
    name?: string
  }>(),
  {
    name: 'm-radio-group',
  },
)

const emit = defineEmits<{
  (e: 'update:modelValue', val: string | number): void
}>()
</script>

<template>
  <RadioGroup
    :model-value="modelValue"
    :options="options"
    :name="name"
    @update:model-value="emit('update:modelValue', $event)"
  />
</template>

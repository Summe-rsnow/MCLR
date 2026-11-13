<script setup lang="ts">
import { ColorPicker } from '@miuui/vue'
import { SEED_PRESETS } from './theme'

withDefaults(
  defineProps<{
    modelValue: string
  }>(),
  {
    modelValue: '#3b82f6',
  },
)

const emit = defineEmits<{
  (e: 'update:modelValue', val: string): void
}>()
</script>

<template>
  <ColorPicker
    :model-value="modelValue"
    :presets="[...SEED_PRESETS]"
    @update:model-value="emit('update:modelValue', $event)"
  />
</template>

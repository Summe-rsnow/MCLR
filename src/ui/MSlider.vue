<script setup lang="ts">
withDefaults(
  defineProps<{
    modelValue: number
    min?: number
    max?: number
    step?: number
    disabled?: boolean
  }>(),
  {
    min: 0,
    max: 100,
    step: 1,
    disabled: false,
  },
)

const emit = defineEmits<{
  (e: 'update:modelValue', val: number): void
}>()

function onInput(event: Event) {
  const target = event.target as HTMLInputElement
  emit('update:modelValue', parseFloat(target.value))
}
</script>

<template>
  <div class="inline-flex items-center w-full gap-3">
    <input
      type="range"
      :value="modelValue"
      :min="min"
      :max="max"
      :step="step"
      :disabled="disabled"
      class="w-full h-2 bg-surfaceVariant rounded-lg appearance-none cursor-pointer accent-primary disabled:opacity-50 disabled:cursor-not-allowed"
      @input="onInput"
    />
    <span class="text-xs text-textMuted w-8 text-right tabular-nums">{{ modelValue }}</span>
  </div>
</template>

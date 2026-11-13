<script setup lang="ts">
import { Slider } from '@miuui/vue'

const props = withDefaults(
  defineProps<{
    modelValue: number
    min?: number
    max?: number
    step?: number
    disabled?: boolean
    labels?: string[]
  }>(),
  {
    min: 0,
    max: 100,
    step: 1,
    disabled: false,
    labels: () => [],
  },
)

const emit = defineEmits<{
  (e: 'update:modelValue', val: number): void
  (e: 'change', val: number): void
}>()

function stopValue(index: number) {
  return props.min + index * props.step
}

function isActiveStop(index: number) {
  return Math.abs(props.modelValue - stopValue(index)) < props.step / 2
}

function selectStop(index: number) {
  if (props.disabled) return
  const val = stopValue(index)
  if (val === props.modelValue) return
  emit('update:modelValue', val)
  emit('change', val)
}

function onUpdate(val: number | [number, number]) {
  const num = Array.isArray(val) ? val[0] : val
  emit('update:modelValue', num)
}

function onCommit(val: number | [number, number]) {
  const num = Array.isArray(val) ? val[0] : val
  emit('change', num)
}
</script>

<template>
  <div class="inline-flex flex-col w-full gap-1.5">
    <div class="flex items-center w-full gap-3">
      <Slider
        :model-value="modelValue"
        :min="min"
        :max="max"
        :step="step"
        :disabled="disabled"
        class="flex-1"
        @update:model-value="onUpdate"
        @commit="onCommit"
      />
      <span v-if="labels.length === 0" class="text-xs text-fg-muted w-8 text-right tabular-nums">{{
        modelValue
      }}</span>
    </div>
    <div v-if="labels.length > 0" class="flex items-center justify-between w-full">
      <button
        v-for="(label, index) in labels"
        :key="label"
        type="button"
        class="text-xs transition-colors duration-150 disabled:cursor-not-allowed cursor-pointer"
        :class="
          isActiveStop(index)
            ? 'text-primary font-semibold'
            : 'text-fg-muted hover:text-fg font-normal'
        "
        :disabled="disabled"
        @click="selectStop(index)"
      >
        {{ label }}
      </button>
    </div>
  </div>
</template>

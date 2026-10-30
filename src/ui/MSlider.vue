<script setup lang="ts">
const props = withDefaults(
  defineProps<{
    modelValue: number
    min?: number
    max?: number
    step?: number
    disabled?: boolean
    /**
     * Evenly spaced stop labels under the track (`min`, `min + step`, …).
     * Passing labels turns the slider into a discrete picker: the numeric
     * readout is replaced by the labels, and clicking one jumps to that stop.
     */
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

/** Value of stop `index`; labels are one `step` apart starting at `min`. */
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

function onInput(event: Event) {
  const target = event.target as HTMLInputElement
  emit('update:modelValue', parseFloat(target.value))
}

function onChange(event: Event) {
  const target = event.target as HTMLInputElement
  emit('change', parseFloat(target.value))
}
</script>

<template>
  <div class="inline-flex flex-col w-full gap-1.5">
    <div class="flex items-center w-full gap-3">
      <input
        type="range"
        :value="modelValue"
        :min="min"
        :max="max"
        :step="step"
        :disabled="disabled"
        class="w-full h-2 bg-sunken rounded-lg appearance-none cursor-pointer accent-primary disabled:opacity-45 disabled:cursor-not-allowed"
        @input="onInput"
        @change="onChange"
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
        class="text-xs transition-colors duration-150 disabled:cursor-not-allowed"
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

<script setup lang="ts">
const props = withDefaults(
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

function toggle() {
  if (props.disabled) return
  emit('update:modelValue', !props.modelValue)
}
</script>

<template>
  <label
    class="inline-flex items-center gap-2.5 cursor-pointer select-none"
    :class="{ 'opacity-45 cursor-not-allowed': disabled }"
  >
    <button
      type="button"
      role="switch"
      :aria-checked="modelValue"
      :disabled="disabled"
      class="relative inline-flex shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out outline-none disabled:cursor-not-allowed"
      :class="[
        size === 'sm' ? 'h-5 w-9' : 'h-6 w-11',
        modelValue ? 'bg-primary' : 'bg-line-strong',
      ]"
      @click="toggle"
    >
      <span
        class="pointer-events-none inline-block transform rounded-full bg-white shadow-1 ring-0 transition duration-200 ease-in-out"
        :class="[
          size === 'sm' ? 'h-4 w-4' : 'h-5 w-5',
          modelValue ? (size === 'sm' ? 'translate-x-4' : 'translate-x-5') : 'translate-x-0',
        ]"
      />
    </button>
    <span v-if="label" class="text-sm font-medium text-fg">{{ label }}</span>
  </label>
</template>

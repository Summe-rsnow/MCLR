<script setup lang="ts">
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

function onChange(event: Event) {
  const target = event.target as HTMLInputElement
  emit('update:modelValue', target.checked)
}
</script>

<template>
  <label
    class="inline-flex items-center gap-2 select-none text-sm transition-colors"
    :class="[disabled ? 'opacity-50 cursor-not-allowed text-fg-subtle' : 'cursor-pointer text-fg']"
  >
    <input
      type="checkbox"
      :checked="modelValue"
      :disabled="disabled"
      class="w-4 h-4 rounded border-line-strong text-primary accent-primary focus:ring-primary/40 disabled:cursor-not-allowed disabled:border-line"
      :class="disabled ? 'cursor-not-allowed' : 'cursor-pointer'"
      @change="onChange"
    />
    <span v-if="label">{{ label }}</span>
    <slot />
  </label>
</template>

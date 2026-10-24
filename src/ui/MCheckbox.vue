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
    class="inline-flex items-center gap-2 cursor-pointer select-none text-sm text-text"
    :class="{ 'opacity-50 cursor-not-allowed': disabled }"
  >
    <input
      type="checkbox"
      :checked="modelValue"
      :disabled="disabled"
      class="w-4 h-4 rounded border-outline text-primary focus:ring-primary/40 cursor-pointer disabled:cursor-not-allowed"
      @change="onChange"
    />
    <span v-if="label">{{ label }}</span>
    <slot />
  </label>
</template>

<script setup lang="ts">
withDefaults(
  defineProps<{
    modelValue: string
    rows?: number
    placeholder?: string
    label?: string
    hint?: string
    error?: string
    disabled?: boolean
  }>(),
  {
    rows: 3,
    placeholder: '',
    label: undefined,
    hint: undefined,
    error: undefined,
    disabled: false,
  },
)

const emit = defineEmits<{
  (e: 'update:modelValue', val: string): void
}>()

function onInput(event: Event) {
  const target = event.target as HTMLTextAreaElement
  emit('update:modelValue', target.value)
}
</script>

<template>
  <div class="flex flex-col gap-1 w-full">
    <label v-if="label" class="text-xs font-medium text-fg">{{ label }}</label>
    <textarea
      :value="modelValue"
      :rows="rows"
      :placeholder="placeholder"
      :disabled="disabled"
      class="w-full px-3 py-2 text-sm rounded-lg bg-sunken text-fg placeholder:text-fg-subtle border outline-none transition-colors disabled:cursor-not-allowed disabled:opacity-50 resize-y"
      :class="
        error
          ? 'border-danger focus:border-danger focus:ring-2 focus:ring-danger/25'
          : 'border-line-strong focus:border-primary focus:ring-2 focus:ring-primary/25'
      "
      @input="onInput"
    />
    <p v-if="error" class="text-xs text-danger-fg">{{ error }}</p>
    <p v-else-if="hint" class="text-xs text-fg-muted">{{ hint }}</p>
  </div>
</template>

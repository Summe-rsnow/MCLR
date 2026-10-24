<script setup lang="ts">
withDefaults(
  defineProps<{
    modelValue: string
    rows?: number
    placeholder?: string
    disabled?: boolean
  }>(),
  {
    rows: 3,
    placeholder: '',
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
  <textarea
    :value="modelValue"
    :rows="rows"
    :placeholder="placeholder"
    :disabled="disabled"
    class="w-full px-3 py-2 text-sm rounded-lg bg-surfaceVariant/60 text-text placeholder-textMuted/70 border border-outline/60 outline-none transition-colors focus:border-primary focus:ring-1 focus:ring-primary/40 disabled:cursor-not-allowed disabled:opacity-50 resize-y"
    @input="onInput"
  />
</template>

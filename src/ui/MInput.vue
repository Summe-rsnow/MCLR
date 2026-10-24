<script setup lang="ts">
withDefaults(
  defineProps<{
    modelValue: string
    placeholder?: string
    type?: string
    disabled?: boolean
    clearable?: boolean
  }>(),
  {
    placeholder: '',
    type: 'text',
    disabled: false,
    clearable: false,
  },
)

const emit = defineEmits<{
  (e: 'update:modelValue', val: string): void
  (e: 'clear'): void
}>()

function onInput(event: Event) {
  const target = event.target as HTMLInputElement
  emit('update:modelValue', target.value)
}

function onClear() {
  emit('update:modelValue', '')
  emit('clear')
}
</script>

<template>
  <div class="relative inline-flex items-center w-full">
    <input
      :type="type"
      :value="modelValue"
      :placeholder="placeholder"
      :disabled="disabled"
      class="w-full px-3 py-2 text-sm rounded-lg bg-surfaceVariant/60 text-text placeholder-textMuted/70 border border-outline/60 outline-none transition-colors focus:border-primary focus:ring-1 focus:ring-primary/40 disabled:cursor-not-allowed disabled:opacity-50"
      :class="{ 'pr-8': clearable && modelValue }"
      @input="onInput"
    />
    <button
      v-if="clearable && modelValue && !disabled"
      type="button"
      class="absolute right-2 text-textMuted hover:text-text cursor-pointer p-0.5 rounded"
      @click="onClear"
    >
      <span class="i-lucide-x w-4 h-4 block" />
    </button>
  </div>
</template>

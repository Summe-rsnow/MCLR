<script setup lang="ts">
withDefaults(
  defineProps<{
    modelValue: string | number
    options: Array<{ label: string; value: string | number }>
    disabled?: boolean
  }>(),
  {
    disabled: false,
  },
)

const emit = defineEmits<{
  (e: 'update:modelValue', val: string | number): void
}>()

function onChange(event: Event) {
  const target = event.target as HTMLSelectElement
  emit('update:modelValue', target.value)
}
</script>

<template>
  <div class="relative inline-flex items-center w-full">
    <select
      :value="modelValue"
      :disabled="disabled"
      class="w-full px-3 py-2 pr-8 text-sm rounded-lg bg-surfaceVariant/60 text-text border border-outline/60 outline-none appearance-none cursor-pointer transition-colors focus:border-primary focus:ring-1 focus:ring-primary/40 disabled:cursor-not-allowed disabled:opacity-50"
      @change="onChange"
    >
      <option
        v-for="opt in options"
        :key="String(opt.value)"
        :value="opt.value"
        class="bg-surface text-text"
      >
        {{ opt.label }}
      </option>
    </select>
    <span class="i-lucide-chevron-down w-4 h-4 text-textMuted absolute right-2.5 pointer-events-none" />
  </div>
</template>

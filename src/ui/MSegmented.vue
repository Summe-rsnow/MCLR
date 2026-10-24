<script setup lang="ts">
withDefaults(
  defineProps<{
    modelValue: string
    options: Array<{ label: string; value: string; icon?: string }>
    disabled?: boolean
  }>(),
  {
    disabled: false,
  },
)

const emit = defineEmits<{
  (e: 'update:modelValue', val: string): void
}>()
</script>

<template>
  <div class="inline-flex p-1 rounded-xl bg-surfaceVariant/80 border border-outline/50 select-none">
    <button
      v-for="opt in options"
      :key="opt.value"
      type="button"
      :disabled="disabled"
      class="inline-flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-lg transition-all cursor-pointer disabled:cursor-not-allowed disabled:opacity-50"
      :class="
        modelValue === opt.value
          ? 'bg-surface text-primary shadow-sm font-semibold'
          : 'text-textMuted hover:text-text'
      "
      @click="emit('update:modelValue', opt.value)"
    >
      <span v-if="opt.icon" :class="opt.icon" class="w-3.5 h-3.5" />
      <span>{{ opt.label }}</span>
    </button>
  </div>
</template>

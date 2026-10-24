<script setup lang="ts">
withDefaults(
  defineProps<{
    modelValue: string
    tabs: Array<{ key: string; label: string; icon?: string }>
  }>(),
  {},
)

const emit = defineEmits<{
  (e: 'update:modelValue', val: string): void
}>()
</script>

<template>
  <div class="flex items-center gap-1 border-b border-outline/60 select-none">
    <button
      v-for="tab in tabs"
      :key="tab.key"
      type="button"
      class="relative inline-flex items-center gap-2 px-4 py-2.5 text-sm font-medium transition-colors cursor-pointer"
      :class="
        modelValue === tab.key
          ? 'text-primary font-semibold'
          : 'text-textMuted hover:text-text'
      "
      @click="emit('update:modelValue', tab.key)"
    >
      <span v-if="tab.icon" :class="tab.icon" class="w-4 h-4" />
      <span>{{ tab.label }}</span>
      <div
        v-if="modelValue === tab.key"
        class="absolute bottom-0 left-0 right-0 h-0.5 bg-primary rounded-t"
      />
    </button>
  </div>
</template>

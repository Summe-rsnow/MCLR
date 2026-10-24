<script setup lang="ts">
withDefaults(
  defineProps<{
    modelValue: string | number
    options: Array<{ label: string; value: string | number; disabled?: boolean }>
    name?: string
  }>(),
  {
    name: 'm-radio-group',
  },
)

const emit = defineEmits<{
  (e: 'update:modelValue', val: string | number): void
}>()

function onSelect(val: string | number) {
  emit('update:modelValue', val)
}
</script>

<template>
  <div class="flex flex-col gap-2">
    <label
      v-for="opt in options"
      :key="String(opt.value)"
      class="inline-flex items-center gap-2 cursor-pointer select-none text-sm text-text"
      :class="{ 'opacity-50 cursor-not-allowed': opt.disabled }"
    >
      <input
        type="radio"
        :name="name"
        :value="opt.value"
        :checked="modelValue === opt.value"
        :disabled="opt.disabled"
        class="w-4 h-4 border-outline text-primary focus:ring-primary/40 cursor-pointer disabled:cursor-not-allowed"
        @change="onSelect(opt.value)"
      />
      <span>{{ opt.label }}</span>
    </label>
  </div>
</template>

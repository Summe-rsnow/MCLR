<script setup lang="ts">
import { ref, watch } from 'vue'

const props = withDefaults(
  defineProps<{
    modelValue: string // #rrggbb
  }>(),
  {
    modelValue: '#3b82f6',
  },
)

const emit = defineEmits<{
  (e: 'update:modelValue', val: string): void
}>()

const presets = [
  '#3b82f6', // Blue
  '#06b6d4', // Cyan
  '#10b981', // Emerald
  '#84cc16', // Lime
  '#eab308', // Yellow
  '#f97316', // Orange
  '#ef4444', // Red
  '#ec4899', // Pink
  '#8b5cf6', // Violet
  '#6366f1', // Indigo
]

const internalHex = ref(props.modelValue)

watch(
  () => props.modelValue,
  (val) => {
    internalHex.value = val
  },
)

function onInput(event: Event) {
  const target = event.target as HTMLInputElement
  internalHex.value = target.value
  emit('update:modelValue', target.value)
}

function selectPreset(hex: string) {
  internalHex.value = hex
  emit('update:modelValue', hex)
}
</script>

<template>
  <div class="flex flex-col gap-2 p-3 rounded-xl border border-outline/50 bg-surfaceVariant/40 w-fit select-none">
    <div class="flex items-center gap-3">
      <input
        type="color"
        :value="internalHex"
        class="w-8 h-8 rounded-lg cursor-pointer border-0 bg-transparent p-0"
        @input="onInput"
      />
      <input
        type="text"
        :value="internalHex"
        class="w-24 px-2 py-1 text-xs rounded border border-outline/60 bg-surface text-text font-mono uppercase"
        @input="onInput"
      />
    </div>

    <div class="flex items-center gap-1.5 flex-wrap max-w-48 pt-1 border-t border-outline/40">
      <button
        v-for="color in presets"
        :key="color"
        type="button"
        class="w-5 h-5 rounded-full border border-black/10 cursor-pointer transition-transform hover:scale-110 active:scale-95 focus:outline-none"
        :class="{ 'ring-2 ring-primary ring-offset-1': internalHex.toLowerCase() === color.toLowerCase() }"
        :style="{ backgroundColor: color }"
        @click="selectPreset(color)"
      />
    </div>
  </div>
</template>

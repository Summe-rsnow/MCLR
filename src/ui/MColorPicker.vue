<script setup lang="ts">
import { ref, watch } from 'vue'
import { SEED_PRESETS } from './theme'

const props = withDefaults(
  defineProps<{
    modelValue: string
  }>(),
  {
    modelValue: '#3b82f6',
  },
)

const emit = defineEmits<{
  (e: 'update:modelValue', val: string): void
}>()

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
  <div class="flex flex-col gap-2.5 p-3 rounded-xl m-well w-fit select-none">
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
        class="w-24 px-2.5 py-1 text-xs rounded-lg border border-line-strong bg-raised text-fg font-mono uppercase outline-none focus:border-primary"
        @input="onInput"
      />
    </div>

    <div class="flex items-center gap-1.5 flex-wrap pt-1.5 border-t border-line">
      <button
        v-for="preset in SEED_PRESETS"
        :key="preset.key"
        type="button"
        class="w-6 h-6 rounded-full border border-black/10 flex items-center justify-center cursor-pointer transition-transform hover:scale-110 active:scale-95 outline-none"
        :class="{
          'ring-2 ring-primary ring-offset-1':
            internalHex.toLowerCase() === preset.hex.toLowerCase(),
        }"
        :style="{ backgroundColor: preset.hex }"
        @click="selectPreset(preset.hex)"
      >
        <span
          v-if="internalHex.toLowerCase() === preset.hex.toLowerCase()"
          class="i-lucide-check w-3.5 h-3.5 text-white drop-shadow-xs"
        />
      </button>
    </div>
  </div>
</template>

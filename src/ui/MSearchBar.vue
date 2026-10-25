<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import MInput from './MInput.vue'

withDefaults(
  defineProps<{
    modelValue: string
    placeholder?: string
    size?: 'sm' | 'md'
    loading?: boolean
    disabled?: boolean
    autofocus?: boolean
  }>(),
  {
    placeholder: undefined,
    size: 'md',
    loading: false,
    disabled: false,
    autofocus: false,
  },
)

const emit = defineEmits<{
  (e: 'update:modelValue', val: string): void
  (e: 'clear'): void
  (e: 'search', val: string): void
}>()

let t = (k: string) => k
try {
  const i18n = useI18n()
  t = i18n.t
} catch {}

function onInput(val: string) {
  emit('update:modelValue', val)
  emit('search', val)
}

function onClear() {
  emit('update:modelValue', '')
  emit('clear')
  emit('search', '')
}
</script>

<template>
  <div class="relative w-full">
    <MInput
      :model-value="modelValue"
      :placeholder="placeholder || t('common.search')"
      :size="size"
      :loading="loading"
      :disabled="disabled"
      :autofocus="autofocus"
      clearable
      @update:model-value="onInput"
      @clear="onClear"
    >
      <template #prefix>
        <span class="i-lucide-search w-4 h-4 text-fg-subtle shrink-0" />
      </template>
    </MInput>
  </div>
</template>

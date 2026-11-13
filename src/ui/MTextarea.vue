<script setup lang="ts">
import { computed, useAttrs } from 'vue'
import { Textarea } from '@miuui/vue'

withDefaults(
  defineProps<{
    modelValue: string
    rows?: number
    placeholder?: string
    label?: string
    hint?: string
    error?: string
    disabled?: boolean
    resize?: 'none' | 'y' | 'x' | 'both'
  }>(),
  {
    rows: 3,
    placeholder: '',
    label: undefined,
    hint: undefined,
    error: undefined,
    disabled: false,
    resize: 'none',
  },
)

const attrs = useAttrs()
const hasExplicitWidth = computed(() => {
  const cls = typeof attrs.class === 'string' ? attrs.class : ''
  return /(?:^|\s)(?:[a-z0-9-]+:)*w-(?!auto\b)\S+/.test(cls)
})

const emit = defineEmits<{
  (e: 'update:modelValue', val: string): void
}>()
</script>

<template>
  <div class="flex flex-col gap-1" :class="hasExplicitWidth ? undefined : 'w-full'">
    <label v-if="label" class="text-xs font-medium text-fg">{{ label }}</label>
    <Textarea
      :model-value="modelValue"
      :rows="rows"
      :placeholder="placeholder"
      :disabled="disabled"
      :invalid="Boolean(error)"
      :class-names="{
        textarea:
          resize === 'y'
            ? 'resize-y'
            : resize === 'x'
              ? 'resize-x'
              : resize === 'both'
                ? 'resize'
                : 'resize-none',
      }"
      @update:model-value="emit('update:modelValue', $event)"
    />
    <p v-if="error" class="text-xs text-danger-fg">{{ error }}</p>
    <p v-else-if="hint" class="text-xs text-fg-muted">{{ hint }}</p>
  </div>
</template>

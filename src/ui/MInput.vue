<script setup lang="ts">
import { useI18n } from 'vue-i18n'

import { onMounted, ref } from 'vue'

let t = (key: string) => key
try {
  const i18n = useI18n()
  t = i18n.t
} catch {
  // Mounted without vue-i18n (unit tests): fall back to the raw key.
}

const props = withDefaults(
  defineProps<{
    modelValue: string | number
    placeholder?: string
    type?: string
    label?: string
    hint?: string
    error?: string
    size?: 'sm' | 'md'
    icon?: string
    autofocus?: boolean
    disabled?: boolean
    clearable?: boolean
    min?: number
    max?: number
    step?: number
    loading?: boolean
  }>(),
  {
    placeholder: '',
    type: 'text',
    label: undefined,
    hint: undefined,
    error: undefined,
    size: 'md',
    icon: undefined,
    autofocus: false,
    disabled: false,
    clearable: false,
    min: undefined,
    max: undefined,
    step: undefined,
    loading: false,
  },
)

const emit = defineEmits<{
  (e: 'update:modelValue', val: string): void
  (e: 'clear'): void
}>()

function stepUp() {
  if (props.disabled) return
  const stepVal = props.step ?? 1
  const current = Number(props.modelValue) || 0
  let next = current + stepVal
  if (props.max !== undefined && next > props.max) {
    next = props.max
  }
  emit('update:modelValue', String(next))
}

function stepDown() {
  if (props.disabled) return
  const stepVal = props.step ?? 1
  const current = Number(props.modelValue) || 0
  let next = current - stepVal
  if (props.min !== undefined && next < props.min) {
    next = props.min
  }
  emit('update:modelValue', String(next))
}

const inputRef = ref<HTMLInputElement | null>(null)

onMounted(() => {
  if (props.autofocus && inputRef.value) {
    inputRef.value.focus()
  }
})

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
  <div class="flex flex-col gap-1 w-full">
    <label v-if="label" class="text-xs font-medium text-fg">{{ label }}</label>
    <div
      class="relative inline-flex items-center w-full rounded-lg bg-sunken border transition-colors"
      :class="[
        size === 'sm' ? 'h-8 text-xs' : 'h-10 text-sm',
        error
          ? 'border-danger focus-within:border-danger focus-within:ring-2 focus-within:ring-danger/25'
          : 'border-line-strong focus-within:border-primary focus-within:ring-2 focus-within:ring-primary/25',
        disabled ? 'opacity-50 cursor-not-allowed' : '',
      ]"
    >
      <span v-if="$slots.prefix" class="pl-3 flex items-center text-fg-subtle shrink-0">
        <slot name="prefix" />
      </span>
      <span
        v-else-if="icon"
        :class="icon"
        class="ml-3 w-4 h-4 text-fg-subtle shrink-0 pointer-events-none"
      />
      <input
        ref="inputRef"
        :type="type"
        :value="modelValue"
        :placeholder="placeholder"
        :disabled="disabled"
        :min="min"
        :max="max"
        :step="step"
        class="w-full h-full bg-transparent text-fg placeholder:text-fg-subtle outline-none focus:outline-none focus-visible:outline-none ring-0 focus:ring-0 px-3 disabled:cursor-not-allowed m-input-inner"
        :class="[
          $slots.prefix || icon ? 'pl-2' : '',
          (clearable && modelValue) || $slots.suffix || type === 'number' ? 'pr-1' : '',
        ]"
        @input="onInput"
      />
      <div
        v-if="type === 'number' && !disabled"
        class="flex flex-col justify-center items-center h-full mr-1.5 shrink-0 select-none py-0.5"
      >
        <button
          type="button"
          tabindex="-1"
          class="flex items-center justify-center w-4.5 h-3.5 text-fg-subtle hover:text-fg hover:bg-fg/10 active:bg-fg/15 rounded cursor-pointer transition-colors"
          @click="stepUp"
        >
          <span class="i-lucide-chevron-up w-3.5 h-3.5 block" />
        </button>
        <button
          type="button"
          tabindex="-1"
          class="flex items-center justify-center w-4.5 h-3.5 text-fg-subtle hover:text-fg hover:bg-fg/10 active:bg-fg/15 rounded cursor-pointer transition-colors"
          @click="stepDown"
        >
          <span class="i-lucide-chevron-down w-3.5 h-3.5 block" />
        </button>
      </div>
      <span
        v-if="loading"
        class="mr-2 inline-block w-4 h-4 border-2 border-current border-t-transparent rounded-full animate-spin text-fg-subtle shrink-0"
      />
      <button
        :aria-label="t('common.clear')"
        v-else-if="clearable && modelValue && !disabled"
        type="button"
        class="mr-2 text-fg-subtle hover:text-fg cursor-pointer p-0.5 rounded shrink-0"
        @click="onClear"
      >
        <span class="i-lucide-x w-4 h-4 block" />
      </button>
      <span v-if="$slots.suffix" class="pr-3 flex items-center text-fg-subtle shrink-0">
        <slot name="suffix" />
      </span>
    </div>
    <p v-if="error" class="text-xs text-danger-fg">{{ error }}</p>
    <p v-else-if="hint" class="text-xs text-fg-muted">{{ hint }}</p>
  </div>
</template>

<style scoped>
.m-input-inner[type='number']::-webkit-inner-spin-button,
.m-input-inner[type='number']::-webkit-outer-spin-button {
  -webkit-appearance: none;
  margin: 0;
}
.m-input-inner[type='number'] {
  -moz-appearance: textfield;
}
</style>

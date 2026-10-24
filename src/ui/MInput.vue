<script setup lang="ts">
import { onMounted, ref } from 'vue'

const props = withDefaults(
  defineProps<{
    modelValue: string
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
  },
)

const emit = defineEmits<{
  (e: 'update:modelValue', val: string): void
  (e: 'clear'): void
}>()

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
        class="w-full h-full bg-transparent text-fg placeholder:text-fg-subtle outline-none px-3 disabled:cursor-not-allowed"
        :class="[
          $slots.prefix || icon ? 'pl-2' : '',
          (clearable && modelValue) || $slots.suffix ? 'pr-8' : '',
        ]"
        @input="onInput"
      />
      <button
        v-if="clearable && modelValue && !disabled"
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

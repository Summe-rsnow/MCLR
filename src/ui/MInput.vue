<script setup lang="ts">
import { computed, onMounted, ref, useAttrs } from 'vue'
import { useI18n } from 'vue-i18n'
import { Input } from '@miuui/vue'

let t = (key: string) => key
try {
  const i18n = useI18n()
  t = i18n.t
} catch {
  // Mounted without vue-i18n (unit tests)
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

const attrs = useAttrs()
const hasExplicitWidth = computed(() => {
  const cls = typeof attrs.class === 'string' ? attrs.class : ''
  return /(?:^|\s)(?:[a-z0-9-]+:)*w-(?!auto\b)\S+/.test(cls)
})

const inputInst = ref<{ focus?: () => void } | null>(null)
onMounted(() => {
  if (props.autofocus) {
    inputInst.value?.focus?.()
  }
})

const stringValue = computed(() =>
  props.modelValue === undefined || props.modelValue === null ? '' : String(props.modelValue),
)

const inputType = computed(() => {
  if (
    props.type === 'password' ||
    props.type === 'email' ||
    props.type === 'search' ||
    props.type === 'number' ||
    props.type === 'url'
  ) {
    return props.type
  }
  return 'text'
})

function stepUp() {
  if (props.disabled) return
  const stepVal = props.step ?? 1
  const current = Number(props.modelValue) || 0
  let next = current + stepVal
  if (props.max !== undefined && next > props.max) next = props.max
  emit('update:modelValue', String(next))
}

function stepDown() {
  if (props.disabled) return
  const stepVal = props.step ?? 1
  const current = Number(props.modelValue) || 0
  let next = current - stepVal
  if (props.min !== undefined && next < props.min) next = props.min
  emit('update:modelValue', String(next))
}

function onUpdate(val: string) {
  emit('update:modelValue', val)
}

function onClear() {
  emit('clear')
}
</script>

<template>
  <div class="flex flex-col gap-1" :class="hasExplicitWidth ? undefined : 'w-full'">
    <label v-if="label" class="text-xs font-medium text-fg">{{ label }}</label>
    <Input
      ref="inputInst"
      :model-value="stringValue"
      :type="inputType"
      :placeholder="placeholder"
      :size="size"
      :disabled="disabled"
      :invalid="Boolean(error)"
      :clearable="clearable && !loading && type !== 'number'"
      :autofocus="autofocus"
      :class-names="{
        root: 'w-full',
        input: type === 'number' ? 'm-num-input' : undefined,
      }"
      @update:model-value="onUpdate"
      @clear="onClear"
    >
      <template v-if="icon || $slots.prefix" #prefix>
        <slot name="prefix">
          <span v-if="icon" :class="icon" class="w-4 h-4 text-fg-subtle shrink-0" />
        </slot>
      </template>
      <template v-if="loading || type === 'number' || $slots.suffix" #suffix>
        <slot name="suffix">
          <span
            v-if="loading"
            class="w-4 h-4 border-2 border-primary border-t-transparent rounded-full animate-spin shrink-0"
          />
          <div v-else-if="type === 'number'" class="flex flex-col shrink-0">
            <button
              type="button"
              tabindex="-1"
              :disabled="disabled"
              class="h-3.5 px-1 flex items-center justify-center text-fg-muted hover:text-fg hover:bg-sunken rounded-t cursor-pointer disabled:opacity-45"
              :aria-label="t('common.increment')"
              @click="stepUp"
            >
              <span class="i-lucide-chevron-up w-3 h-3" />
            </button>
            <button
              type="button"
              tabindex="-1"
              :disabled="disabled"
              class="h-3.5 px-1 flex items-center justify-center text-fg-muted hover:text-fg hover:bg-sunken rounded-b cursor-pointer disabled:opacity-45"
              :aria-label="t('common.decrement')"
              @click="stepDown"
            >
              <span class="i-lucide-chevron-down w-3 h-3" />
            </button>
          </div>
        </slot>
      </template>
    </Input>
    <p v-if="error" class="text-xs text-danger-fg">{{ error }}</p>
    <p v-else-if="hint" class="text-xs text-fg-muted">{{ hint }}</p>
  </div>
</template>

<style scoped>
:deep(.m-num-input::-webkit-inner-spin-button),
:deep(.m-num-input::-webkit-outer-spin-button) {
  -webkit-appearance: none;
  margin: 0;
}
</style>

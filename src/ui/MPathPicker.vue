<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { open as openFileDialog, save as saveFileDialog } from '@tauri-apps/plugin-dialog'
import MButton from './MButton.vue'

export interface PathPickerFilter {
  name: string
  extensions: string[]
}

const props = withDefaults(
  defineProps<{
    modelValue: string
    placeholder?: string
    type?: 'file' | 'directory' | 'saveFile'
    filters?: PathPickerFilter[]
    disabled?: boolean
    clearable?: boolean
    size?: 'sm' | 'md'
    buttonText?: string
    defaultPath?: string
  }>(),
  {
    placeholder: '',
    type: 'file',
    filters: undefined,
    disabled: false,
    clearable: false,
    size: 'md',
    buttonText: undefined,
    defaultPath: undefined,
  },
)

const emit = defineEmits<{
  (e: 'update:modelValue', val: string): void
  (e: 'clear'): void
}>()

const { t } = useI18n()

const leftIcon = computed(() => {
  if (props.type === 'directory') return 'i-lucide-folder'
  return 'i-lucide-file-text'
})

const resolvedButtonText = computed(() => {
  return props.buttonText || t('common.browse')
})

async function handleBrowse() {
  if (props.disabled) return
  try {
    if (props.type === 'saveFile') {
      const selected = await saveFileDialog({
        defaultPath: props.defaultPath || props.modelValue || undefined,
        filters: props.filters,
      })
      if (typeof selected === 'string' && selected) {
        emit('update:modelValue', selected)
      }
    } else {
      const selected = await openFileDialog({
        directory: props.type === 'directory',
        multiple: false,
        defaultPath: props.defaultPath || props.modelValue || undefined,
        filters: props.filters,
      })
      if (typeof selected === 'string' && selected) {
        emit('update:modelValue', selected)
      }
    }
  } catch {
    // Graceful fallback in browser / test environments or when dialog is cancelled
  }
}

function handleClear() {
  emit('update:modelValue', '')
  emit('clear')
}

function handleContainerClick() {
  if (props.disabled) return
  handleBrowse()
}

function onInput(e: Event) {
  const target = e.target as HTMLInputElement
  emit('update:modelValue', target.value)
}
</script>

<template>
  <div
    class="relative inline-flex items-center w-full rounded-lg bg-sunken border border-line-strong transition-all gap-2 px-2.5 overflow-hidden select-none"
    :class="[
      size === 'sm' ? 'h-8 text-xs' : 'h-10 text-sm',
      disabled ? 'opacity-50 cursor-not-allowed' : 'hover:border-primary/50 cursor-pointer',
    ]"
    :title="modelValue || placeholder"
    @click="handleContainerClick"
  >
    <!-- Left Icon -->
    <span
      class="shrink-0 text-fg-subtle pointer-events-none"
      :class="[leftIcon, size === 'sm' ? 'w-3.5 h-3.5' : 'w-4 h-4']"
    />

    <!-- Readonly Path Display / Input -->
    <input
      type="text"
      :value="modelValue"
      :placeholder="placeholder"
      readonly
      tabindex="-1"
      :disabled="disabled"
      :title="modelValue || placeholder"
      class="flex-1 min-w-0 bg-transparent text-fg placeholder:text-fg-subtle outline-none select-none cursor-pointer truncate font-mono"
      :class="size === 'sm' ? 'text-xs' : 'text-sm'"
      @keydown.prevent
      @input="onInput"
    />

    <!-- Right Actions -->
    <div class="flex items-center gap-1.5 shrink-0" @click.stop>
      <!-- Clear Button -->
      <button
        v-if="clearable && modelValue && !disabled"
        type="button"
        class="text-fg-subtle hover:text-fg cursor-pointer p-0.5 rounded transition-colors"
        :title="t('common.clear')"
        @click.stop="handleClear"
      >
        <span class="i-lucide-x block" :class="size === 'sm' ? 'w-3.5 h-3.5' : 'w-4 h-4'" />
      </button>

      <!-- Browse Button -->
      <MButton
        variant="tonal"
        :size="size === 'sm' ? 'xs' : 'sm'"
        icon="i-lucide-folder-open"
        :disabled="disabled"
        @click.stop="handleBrowse"
      >
        {{ resolvedButtonText }}
      </MButton>
    </div>
  </div>
</template>

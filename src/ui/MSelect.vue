<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'

export interface SelectOption {
  label: string
  value: string | number
  icon?: string
  avatar?: string
  badge?: string
  disabled?: boolean
}

const props = withDefaults(
  defineProps<{
    modelValue: string | number
    options: SelectOption[]
    placeholder?: string
    size?: 'sm' | 'md'
    disabled?: boolean
    block?: boolean
    loading?: boolean
  }>(),
  {
    placeholder: '',
    size: 'md',
    disabled: false,
    block: true,
    loading: false,
  },
)

let t = (key: string) => key
try {
  const i18n = useI18n()
  t = i18n.t
} catch {
  // safe fallback in unit tests mounted without vue-i18n
}

const emit = defineEmits<{
  (e: 'update:modelValue', val: string | number): void
}>()

const isOpen = ref(false)
const activeIndex = ref(-1)
const triggerRef = ref<HTMLButtonElement | null>(null)
const listRef = ref<HTMLDivElement | null>(null)
const popupStyle = ref<Record<string, string>>({})

const selectedOption = computed(() =>
  props.options.find((o) => String(o.value) === String(props.modelValue)),
)

function updatePopupPosition() {
  const el = triggerRef.value
  if (!el) return
  const rect = el.getBoundingClientRect()
  const vh = typeof window !== 'undefined' ? window.innerHeight : 800
  const spaceBelow = vh - rect.bottom
  const openAbove = spaceBelow < 200 && rect.top > spaceBelow

  const triggerWidth = rect.width
  popupStyle.value = {
    left: `${Math.max(8, rect.left)}px`,
    minWidth: `${Math.max(120, triggerWidth)}px`,
    maxWidth: '380px',
    width: 'max-content',
    ...(openAbove
      ? { bottom: `${Math.max(8, vh - rect.top + 4)}px` }
      : { top: `${rect.bottom + 4}px` }),
  }
}
async function openListbox() {
  if (props.disabled || props.options.length === 0) return
  isOpen.value = true
  const idx = props.options.findIndex((o) => String(o.value) === String(props.modelValue))
  activeIndex.value = idx >= 0 ? idx : props.options.findIndex((o) => !o.disabled)
  await nextTick()
  updatePopupPosition()
}

function closeListbox() {
  isOpen.value = false
}

function toggleListbox() {
  if (props.disabled || props.options.length === 0) return
  if (isOpen.value) {
    closeListbox()
  } else {
    openListbox()
  }
}

function selectOption(opt: SelectOption) {
  if (opt.disabled) return
  emit('update:modelValue', opt.value)
  closeListbox()
  triggerRef.value?.focus()
}

function moveActive(delta: number) {
  const len = props.options.length
  if (len === 0) return
  let idx = activeIndex.value
  for (let i = 0; i < len; i++) {
    idx = (idx + delta + len) % len
    if (!props.options[idx].disabled) {
      activeIndex.value = idx
      return
    }
  }
}

function handleKeyDown(event: KeyboardEvent) {
  if (props.disabled || props.options.length === 0) return
  if (event.key === 'ArrowDown') {
    event.preventDefault()
    if (!isOpen.value) {
      openListbox()
    } else {
      moveActive(1)
    }
  } else if (event.key === 'ArrowUp') {
    event.preventDefault()
    if (!isOpen.value) {
      openListbox()
    } else {
      moveActive(-1)
    }
  } else if (event.key === 'Enter' || event.key === ' ') {
    event.preventDefault()
    if (!isOpen.value) {
      openListbox()
    } else if (activeIndex.value >= 0 && activeIndex.value < props.options.length) {
      selectOption(props.options[activeIndex.value])
    }
  } else if (event.key === 'Escape') {
    if (isOpen.value) {
      event.preventDefault()
      closeListbox()
    }
  } else if (event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
    const ch = event.key.toLowerCase()
    const matchIdx = props.options.findIndex(
      (o) => !o.disabled && o.label.trim().toLowerCase().startsWith(ch),
    )
    if (matchIdx >= 0) {
      activeIndex.value = matchIdx
      if (!isOpen.value) {
        emit('update:modelValue', props.options[matchIdx].value)
      }
    }
  }
}

function handleOutsideMouseDown(event: MouseEvent) {
  if (!isOpen.value) return
  const target = event.target as Node | null
  if (!target) return
  if (triggerRef.value?.contains(target)) return
  if (listRef.value?.contains(target)) return
  closeListbox()
}

onMounted(() => {
  if (typeof window !== 'undefined') {
    window.addEventListener('mousedown', handleOutsideMouseDown)
    window.addEventListener('resize', updatePopupPosition)
  }
})

onUnmounted(() => {
  if (typeof window !== 'undefined') {
    window.removeEventListener('mousedown', handleOutsideMouseDown)
    window.removeEventListener('resize', updatePopupPosition)
  }
})
</script>

<template>
  <div
    class="relative inline-flex items-center"
    :class="block ? 'w-full max-w-full' : 'w-auto max-w-[360px] min-w-[140px]'"
  >
    <button
      ref="triggerRef"
      type="button"
      role="combobox"
      :aria-expanded="isOpen"
      :disabled="disabled || loading || options.length === 0"
      class="px-3 inline-flex items-center justify-between gap-2 rounded-lg text-fg border outline-none select-none text-left transition-colors"
      :class="[
        block ? 'w-full max-w-full' : 'w-auto min-w-[140px] max-w-[360px]',
        size === 'sm' ? 'h-8 text-xs' : 'h-10 text-sm',
        disabled || options.length === 0
          ? 'opacity-60 bg-sunken/40 border-line text-fg-subtle cursor-not-allowed'
          : 'bg-sunken border-line-strong cursor-pointer focus:border-primary focus:ring-2 focus:ring-primary/25',
      ]"
      @click="toggleListbox"
      @keydown="handleKeyDown"
    >
      <span class="inline-flex items-center gap-2 min-w-0 truncate">
        <img
          v-if="selectedOption?.avatar"
          :src="selectedOption.avatar"
          alt=""
          loading="lazy"
          decoding="async"
          class="w-5 h-5 rounded-full object-cover shrink-0"
        />
        <span
          v-else-if="selectedOption?.icon"
          :class="selectedOption.icon"
          class="w-4 h-4 text-fg-muted shrink-0"
        />
        <span v-if="selectedOption" class="truncate">{{ selectedOption.label }}</span>
        <span
          v-if="selectedOption?.badge"
          class="text-xs px-1.5 py-0.5 rounded bg-primary/10 text-primary shrink-0 font-normal"
        >
          {{ selectedOption.badge }}
        </span>
        <span v-else-if="!selectedOption" class="truncate text-fg-subtle">
          {{
            placeholder || (options.length === 0 ? t('common.noOptions') : t('common.unselected'))
          }}
        </span>
      </span>
      <span
        v-if="loading"
        class="inline-block w-4 h-4 border-2 border-current border-t-transparent rounded-full animate-spin text-fg-muted shrink-0"
      />
      <span
        v-else
        class="i-lucide-chevron-down w-4 h-4 text-fg-muted shrink-0 transition-transform duration-150"
        :class="{ 'rotate-180': isOpen }"
      />
    </button>
    <select
      :value="modelValue"
      :disabled="disabled"
      tabindex="-1"
      aria-hidden="true"
      class="sr-only pointer-events-none"
      @change="emit('update:modelValue', ($event.target as HTMLSelectElement).value)"
    >
      <option
        v-for="opt in options"
        :key="String(opt.value)"
        :value="opt.value"
        :disabled="opt.disabled"
      >
        {{ opt.label }}
      </option>
    </select>

    <Teleport to="body">
      <Transition name="m-pop">
        <div
          v-if="isOpen"
          class="fixed z-50 m-overlay m-refract rounded-xl overflow-hidden select-none"
          :style="popupStyle"
        >
          <div
            ref="listRef"
            role="listbox"
            data-testid="m-select-listbox"
            class="m-select-list max-h-[280px] overflow-y-auto p-1 pr-1.5 flex flex-col gap-0.5"
          >
            <button
              v-for="(opt, idx) in options"
              :key="String(opt.value)"
              type="button"
              role="option"
              :aria-selected="String(opt.value) === String(modelValue)"
              :disabled="opt.disabled"
              class="w-full flex items-center justify-between gap-2 px-3 py-2 text-sm rounded-lg transition-colors text-left cursor-pointer disabled:opacity-45 disabled:cursor-not-allowed"
              :class="[
                String(opt.value) === String(modelValue)
                  ? 'bg-primary-soft text-primary-soft-fg font-medium'
                  : idx === activeIndex
                    ? 'bg-sunken text-fg'
                    : 'text-fg hover:bg-sunken',
              ]"
              @mouseenter="activeIndex = idx"
              @click="selectOption(opt)"
            >
              <span class="inline-flex items-center gap-2 min-w-0 truncate">
                <img
                  v-if="opt.avatar"
                  :src="opt.avatar"
                  alt=""
                  loading="lazy"
                  decoding="async"
                  class="w-5 h-5 rounded-full object-cover shrink-0"
                />
                <span v-else-if="opt.icon" :class="opt.icon" class="w-4 h-4 shrink-0" />
                <span class="truncate">{{ opt.label }}</span>
                <span
                  v-if="opt.badge"
                  class="text-xs px-1.5 py-0.5 rounded bg-primary/10 text-primary shrink-0 font-normal"
                >
                  {{ opt.badge }}
                </span>
              </span>
              <span
                v-if="String(opt.value) === String(modelValue)"
                class="i-lucide-check w-4 h-4 shrink-0 ml-auto text-primary"
              />
            </button>
          </div>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<style scoped>
.m-select-list::-webkit-scrollbar-track {
  margin: 6px 0;
}
</style>

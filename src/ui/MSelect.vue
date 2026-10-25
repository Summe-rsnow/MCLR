<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref } from 'vue'

export interface SelectOption {
  label: string
  value: string | number
  icon?: string
  disabled?: boolean
}

const props = withDefaults(
  defineProps<{
    modelValue: string | number
    options: SelectOption[]
    placeholder?: string
    size?: 'sm' | 'md'
    disabled?: boolean
  }>(),
  {
    placeholder: '',
    size: 'md',
    disabled: false,
  },
)

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

  popupStyle.value = {
    left: `${Math.max(8, rect.left)}px`,
    width: `${Math.max(120, rect.width)}px`,
    ...(openAbove
      ? { bottom: `${Math.max(8, vh - rect.top + 4)}px` }
      : { top: `${rect.bottom + 4}px` }),
  }
}

async function openListbox() {
  if (props.disabled) return
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
  if (props.disabled) return
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
  <div class="relative inline-flex items-center w-full">
    <button
      ref="triggerRef"
      type="button"
      role="combobox"
      :aria-expanded="isOpen"
      :disabled="disabled"
      class="w-full px-3 inline-flex items-center justify-between gap-2 rounded-lg bg-sunken text-fg border border-line-strong outline-none cursor-pointer transition-colors focus:border-primary focus:ring-2 focus:ring-primary/25 disabled:cursor-not-allowed disabled:opacity-50 select-none text-left"
      :class="size === 'sm' ? 'h-8 text-xs' : 'h-10 text-sm'"
      @click="toggleListbox"
      @keydown="handleKeyDown"
    >
      <span class="inline-flex items-center gap-2 min-w-0 truncate">
        <span
          v-if="selectedOption?.icon"
          :class="selectedOption.icon"
          class="w-4 h-4 text-fg-muted shrink-0"
        />
        <span v-if="selectedOption" class="truncate">{{ selectedOption.label }}</span>
        <span v-else class="truncate text-fg-subtle">{{ placeholder }}</span>
      </span>
      <span
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
      <option v-for="opt in options" :key="String(opt.value)" :value="opt.value" :disabled="opt.disabled">
        {{ opt.label }}
      </option>
    </select>

    <Teleport to="body">
      <Transition name="m-pop">
      <div
        v-if="isOpen"
        ref="listRef"
        role="listbox"
        data-testid="m-select-listbox"
        class="fixed z-50 m-overlay m-refract rounded-xl p-1 max-h-[280px] overflow-y-auto flex flex-col gap-0.5 select-none"
        :style="popupStyle"
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
            <span v-if="opt.icon" :class="opt.icon" class="w-4 h-4 shrink-0" />
            <span class="truncate">{{ opt.label }}</span>
          </span>
          <span
            v-if="String(opt.value) === String(modelValue)"
            class="i-lucide-check w-4 h-4 shrink-0"
          />
        </button>
      </div>
      </Transition>
    </Teleport>
  </div>
</template>

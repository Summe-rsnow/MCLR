<script setup lang="ts" generic="T extends string | number = string | number">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { SegmentedControl } from '@miuui/vue'
import type { SegmentedOption } from '@miuui/vue'

export interface MSegmentedOption<V extends string | number = string | number> {
  label: string
  value: V
  icon?: string
}

const props = withDefaults(
  defineProps<{
    modelValue: T
    options: MSegmentedOption<T>[]
    size?: 'sm' | 'md'
    disabled?: boolean
    block?: boolean
  }>(),
  {
    size: 'md',
    disabled: false,
    block: false,
  },
)

const emit = defineEmits<{
  (e: 'update:modelValue', val: T): void
}>()

const hostRef = ref<HTMLElement | null>(null)
const measureRef = ref<HTMLElement | null>(null)
const compact = ref(false)
let observer: ResizeObserver | null = null

const optionMap = computed(() => {
  const m = new Map<string, MSegmentedOption<T>>()
  for (const opt of props.options) m.set(String(opt.value), opt)
  return m
})

const miuOptions = computed<SegmentedOption[]>(() =>
  props.options.map((opt) => ({
    value: String(opt.value),
    label: opt.label,
    title: opt.label,
    ariaLabel: opt.label,
  })),
)

function checkFit() {
  const host = hostRef.value
  const measure = measureRef.value
  if (!host || !measure) return
  if (host.clientWidth === 0 || measure.scrollWidth === 0) return
  compact.value = measure.scrollWidth > host.clientWidth
}

onMounted(() => {
  checkFit()
  if (typeof ResizeObserver !== 'undefined' && hostRef.value) {
    observer = new ResizeObserver(checkFit)
    observer.observe(hostRef.value)
  }
})

onUnmounted(() => {
  observer?.disconnect()
})

watch(() => props.options, checkFit, { deep: true })

function onUpdate(value: string) {
  const orig = optionMap.value.get(value)
  emit('update:modelValue', (orig ? orig.value : value) as T)
}
</script>

<template>
  <div
    ref="hostRef"
    class="relative min-w-0 max-w-full"
    :class="block ? 'flex w-full justify-end' : 'inline-flex w-fit'"
  >
    <SegmentedControl
      :model-value="String(modelValue)"
      :options="miuOptions"
      :size="size"
      :disabled="disabled"
      :icon-only="compact && options.every((o) => Boolean(o.icon))"
      :class-names="{ item: 'data-seg-btn' }"
      @update:model-value="onUpdate"
    >
      <template #item="{ option }">
        <span
          v-if="optionMap.get(option.value)?.icon"
          :class="optionMap.get(option.value)?.icon"
          class="w-3.5 h-3.5 shrink-0"
        />
        <span
          v-if="option.label && !(compact && optionMap.get(option.value)?.icon)"
          class="whitespace-nowrap"
        >
          {{ option.label }}
        </span>
      </template>
    </SegmentedControl>

    <div
      ref="measureRef"
      aria-hidden="true"
      class="invisible pointer-events-none absolute top-0 left-0 flex items-center gap-0 whitespace-nowrap p-1"
    >
      <span
        v-for="opt in options"
        :key="`m-${opt.value}`"
        class="inline-flex items-center gap-1.5"
        :class="size === 'sm' ? 'px-2.5 py-1 text-xs' : 'px-3 py-1.5 text-xs'"
      >
        <span v-if="opt.icon" class="w-3.5 h-3.5 shrink-0" />
        <span v-if="opt.label">{{ opt.label }}</span>
      </span>
    </div>
  </div>
</template>

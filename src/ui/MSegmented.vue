<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, watch } from 'vue'

const props = withDefaults(
  defineProps<{
    modelValue: string
    options: Array<{ label: string; value: string; icon?: string }>
    size?: 'sm' | 'md'
    disabled?: boolean
  }>(),
  {
    size: 'md',
    disabled: false,
  },
)

const emit = defineEmits<{
  (e: 'update:modelValue', val: string): void
}>()

const containerRef = ref<HTMLElement | null>(null)
const indicatorStyle = ref<Record<string, string>>({ opacity: '0' })
let observer: ResizeObserver | null = null

function updateIndicator() {
  const root = containerRef.value
  if (!root) return
  const idx = props.options.findIndex((o) => o.value === props.modelValue)
  const buttons = root.querySelectorAll<HTMLButtonElement>('button[data-seg-btn]')
  const btn = idx >= 0 ? buttons[idx] : undefined
  if (!btn || btn.offsetWidth === 0) {
    indicatorStyle.value = { opacity: '0' }
    return
  }
  indicatorStyle.value = {
    opacity: '1',
    width: `${btn.offsetWidth}px`,
    transform: `translateX(${btn.offsetLeft - 4}px)`,
  }
}

watch(
  () => [props.modelValue, props.options.length, props.size],
  async () => {
    await nextTick()
    updateIndicator()
  },
)

onMounted(() => {
  updateIndicator()
  if (typeof ResizeObserver !== 'undefined' && containerRef.value) {
    observer = new ResizeObserver(() => updateIndicator())
    observer.observe(containerRef.value)
  }
})

onUnmounted(() => {
  observer?.disconnect()
})
</script>

<template>
  <div
    ref="containerRef"
    class="relative inline-flex items-center p-1 rounded-xl bg-sunken border border-line select-none"
  >
    <div
      class="pointer-events-none absolute top-1 bottom-1 left-1 rounded-lg bg-raised shadow-1 transition-all duration-200"
      :style="indicatorStyle"
    />
    <button
      v-for="opt in options"
      :key="opt.value"
      data-seg-btn
      type="button"
      :disabled="disabled"
      class="relative z-1 inline-flex items-center justify-center gap-1.5 rounded-lg font-medium transition-colors cursor-pointer disabled:cursor-not-allowed disabled:opacity-45"
      :class="[
        size === 'sm' ? 'px-2.5 py-1 text-xs' : 'px-3 py-1.5 text-xs',
        modelValue === opt.value
          ? 'bg-raised text-fg shadow-1'
          : 'text-fg-muted hover:text-fg',
      ]"
      @click="emit('update:modelValue', opt.value)"
    >
      <span v-if="opt.icon" :class="opt.icon" class="w-3.5 h-3.5 shrink-0" />
      <span v-if="opt.label">{{ opt.label }}</span>
    </button>
  </div>
</template>

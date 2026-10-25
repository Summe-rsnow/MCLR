<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, watch } from 'vue'

const props = defineProps<{
  modelValue: string
  tabs: Array<{ key: string; label: string; icon?: string }>
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', val: string): void
}>()

const containerRef = ref<HTMLElement | null>(null)
const indicatorStyle = ref<Record<string, string>>({ opacity: '0' })
let observer: ResizeObserver | null = null

function updateIndicator() {
  const root = containerRef.value
  if (!root) return
  const idx = props.tabs.findIndex((t) => t.key === props.modelValue)
  const buttons = root.querySelectorAll<HTMLButtonElement>('button[data-tab-btn]')
  const btn = idx >= 0 ? buttons[idx] : undefined
  if (!btn || btn.offsetWidth === 0) {
    indicatorStyle.value = { opacity: '0' }
    return
  }
  indicatorStyle.value = {
    opacity: '1',
    width: `${btn.offsetWidth}px`,
    transform: `translateX(${btn.offsetLeft}px)`,
  }
}

function onWheel(e: WheelEvent) {
  const root = containerRef.value
  if (!root) return
  if (Math.abs(e.deltaY) > Math.abs(e.deltaX) && e.deltaY !== 0) {
    root.scrollLeft += e.deltaY
  }
}

watch(
  () => [props.modelValue, props.tabs.length],
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
    class="relative flex items-center gap-1 border-b border-line select-none overflow-x-auto [scrollbar-width:none] [&::-webkit-scrollbar]:hidden"
    @wheel.passive="onWheel"
  >
    <button
      v-for="tab in tabs"
      :key="tab.key"
      data-tab-btn
      type="button"
      class="relative inline-flex items-center gap-2 px-4 py-2.5 text-sm font-medium transition-colors cursor-pointer shrink-0"
      :class="modelValue === tab.key ? 'text-primary' : 'text-fg-muted hover:text-fg'"
      @click="emit('update:modelValue', tab.key)"
    >
      <span v-if="tab.icon" :class="tab.icon" class="w-4 h-4 shrink-0" />
      <span>{{ tab.label }}</span>
      <div
        v-if="modelValue === tab.key"
        class="absolute bottom-0 left-0 right-0 h-0.5 bg-primary rounded-t"
      />
    </button>
    <div
      class="m-tabs-pill pointer-events-none absolute bottom-0 left-0 h-0.5 bg-primary rounded-t transition-all duration-200"
      :style="indicatorStyle"
    />
  </div>
</template>

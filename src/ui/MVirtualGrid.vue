<script setup lang="ts" generic="T">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, shallowRef, watch } from 'vue'
import { useVirtualizer } from '@tanstack/vue-virtual'
import { findScrollParent, offsetWithin } from '../composables/useScrollParent'

const props = withDefaults(
  defineProps<{
    items: T[]
    itemKey: string | ((item: T, index: number) => string | number)
    /** Row height estimate (px); rows are measured after render. */
    estimateSize?: number
    minColumnWidth?: number
    gap?: number
    overscan?: number
    threshold?: number
    scrollEl?: HTMLElement | null
    maxHeight?: string
  }>(),
  {
    estimateSize: 200,
    minColumnWidth: 220,
    gap: 12,
    overscan: 3,
    threshold: 60,
    scrollEl: null,
    maxHeight: '70vh',
  },
)

defineSlots<{ item(props: { item: T; index: number }): unknown }>()

function keyOf(item: T, index: number): string | number {
  if (typeof props.itemKey === 'function') return props.itemKey(item, index)
  return (item as Record<string, string | number>)[props.itemKey] ?? index
}

const virtual = computed(() => props.items.length > props.threshold)
const outerRef = ref<HTMLElement | null>(null)
const innerRef = ref<HTMLElement | null>(null)
const scroller = shallowRef<HTMLElement | null>(null)
const selfScroll = ref(false)
const scrollMargin = ref(0)
const width = ref(0)

const columns = computed(() =>
  Math.max(1, Math.floor((width.value + props.gap) / (props.minColumnWidth + props.gap))),
)
const rowCount = computed(() => Math.ceil(props.items.length / columns.value))

function measure() {
  const outer = outerRef.value
  if (!outer) return
  const computedPad =
    typeof window !== 'undefined'
      ? (parseFloat(getComputedStyle(outer).paddingLeft) || 0) +
        (parseFloat(getComputedStyle(outer).paddingRight) || 0)
      : 0
  width.value = Math.max(0, outer.clientWidth - computedPad)
  if (!virtual.value) return
  const target = props.scrollEl ?? findScrollParent(outer)
  selfScroll.value = target === null
  scroller.value = target ?? outer
  scrollMargin.value = target && innerRef.value ? offsetWithin(innerRef.value, target) : 0
}

let ro: ResizeObserver | null = null
onMounted(async () => {
  await nextTick()
  measure()
  if (typeof ResizeObserver !== 'undefined' && outerRef.value) {
    ro = new ResizeObserver(() => measure())
    ro.observe(outerRef.value)
  }
})
onBeforeUnmount(() => ro?.disconnect())
watch(virtual, async () => {
  await nextTick()
  measure()
})

const virtualizer = useVirtualizer(
  computed(() => ({
    count: virtual.value ? rowCount.value : 0,
    getScrollElement: () => scroller.value,
    estimateSize: () => props.estimateSize,
    overscan: props.overscan,
    gap: props.gap,
    scrollMargin: scrollMargin.value,
  })),
)

const rows = computed(() => virtualizer.value.getVirtualItems())
const totalSize = computed(() => virtualizer.value.getTotalSize())

function rowItems(rowIndex: number) {
  const start = rowIndex * columns.value
  return props.items
    .slice(start, start + columns.value)
    .map((item, i) => ({ item, index: start + i, key: keyOf(item, start + i) }))
}

function measureEl(el: unknown) {
  if (el instanceof HTMLElement) virtualizer.value.measureElement(el)
}

const gridStyle = computed(() => ({
  display: 'grid',
  gap: props.gap + 'px',
  gridTemplateColumns: `repeat(${columns.value}, minmax(0, 1fr))`,
}))
</script>

<template>
  <div
    ref="outerRef"
    class="w-full overflow-x-hidden"
    :class="virtual && selfScroll ? 'overflow-y-auto' : ''"
    :style="virtual && selfScroll ? { maxHeight } : undefined"
  >
    <div v-if="!virtual" :style="gridStyle">
      <template v-for="(item, index) in items" :key="keyOf(item, index)">
        <slot name="item" :item="item" :index="index" />
      </template>
    </div>
    <div
      v-else
      ref="innerRef"
      data-testid="m-virtual-grid"
      class="relative w-full"
      :style="{ height: totalSize - scrollMargin + 'px' }"
    >
      <div
        v-for="row in rows"
        :key="String(row.key)"
        :ref="measureEl"
        :data-index="row.index"
        class="absolute top-0 left-0 w-full"
        :style="{ ...gridStyle, transform: `translateY(${row.start - scrollMargin}px)` }"
      >
        <template v-for="cell in rowItems(row.index)" :key="cell.key">
          <slot name="item" :item="cell.item" :index="cell.index" />
        </template>
      </div>
    </div>
  </div>
</template>

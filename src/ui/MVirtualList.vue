<script setup lang="ts" generic="T">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, shallowRef, watch } from 'vue'
import { useVirtualizer } from '@tanstack/vue-virtual'
import { findScrollParent, offsetWithin } from '../composables/useScrollParent'

const props = withDefaults(
  defineProps<{
    items: T[]
    /** Property name or function producing a stable key per item. */
    itemKey: string | ((item: T, index: number) => string | number)
    estimateSize?: number
    gap?: number
    overscan?: number
    /** Below this many items the list renders plainly (no virtualization). */
    threshold?: number
    scrollEl?: HTMLElement | null
    /** Height of the self-scrolling fallback when no scrollable ancestor exists. */
    maxHeight?: string
  }>(),
  {
    estimateSize: 64,
    gap: 0,
    overscan: 6,
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

function measure() {
  if (!virtual.value) return
  const outer = outerRef.value
  if (!outer) return
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
    count: virtual.value ? props.items.length : 0,
    getScrollElement: () => scroller.value,
    estimateSize: () => props.estimateSize,
    overscan: props.overscan,
    gap: props.gap,
    scrollMargin: scrollMargin.value,
    getItemKey: (i: number) => keyOf(props.items[i] as T, i),
  })),
)

const rows = computed(() => virtualizer.value.getVirtualItems())
const totalSize = computed(() => virtualizer.value.getTotalSize())

function measureEl(el: unknown) {
  if (el instanceof HTMLElement) virtualizer.value.measureElement(el)
}
</script>

<template>
  <div
    ref="outerRef"
    class="w-full overflow-x-clip"
    :class="virtual && selfScroll ? 'overflow-y-auto' : ''"
    :style="virtual && selfScroll ? { maxHeight } : undefined"
  >
    <div v-if="!virtual" class="flex flex-col" :style="{ gap: gap + 'px' }">
      <template v-for="(item, index) in items" :key="keyOf(item, index)">
        <slot name="item" :item="item" :index="index" />
      </template>
    </div>
    <div
      v-else
      ref="innerRef"
      data-testid="m-virtual-list"
      class="relative w-full"
      :style="{ height: totalSize - scrollMargin + 'px' }"
    >
      <div
        v-for="row in rows"
        :key="String(row.key)"
        :ref="measureEl"
        :data-index="row.index"
        class="absolute top-0 left-0 w-full"
        :style="{ transform: `translateY(${row.start - scrollMargin}px)` }"
      >
        <slot name="item" :item="items[row.index] as T" :index="row.index" />
      </div>
    </div>
  </div>
</template>

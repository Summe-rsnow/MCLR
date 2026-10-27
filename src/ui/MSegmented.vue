<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, watch } from 'vue'

const props = withDefaults(
  defineProps<{
    modelValue: string
    options: Array<{ label: string; value: string; icon?: string }>
    size?: 'sm' | 'md'
    disabled?: boolean
    /**
     * Fill the parent slot instead of hugging the content. Required for reliable
     * label-to-icon collapsing: only a fixed-width root can tell "the labels do
     * not fit" apart from "I am as small as my labels".
     */
    block?: boolean
  }>(),
  {
    size: 'md',
    disabled: false,
    block: false,
  },
)

const emit = defineEmits<{
  (e: 'update:modelValue', val: string): void
}>()

const containerRef = ref<HTMLElement | null>(null)
const indicatorStyle = ref<Record<string, string>>({ opacity: '0' })
let observer: ResizeObserver | null = null

/**
 * True when the labels do not fit the width the parent allows.
 *
 * The control sits in settings rows and cards that can be much narrower than
 * its natural width; without this the labels paint outside the card. Icons stay
 * visible, and each button keeps its label as `title`/`aria-label`.
 */
const compact = ref(false)
const measureRef = ref<HTMLElement | null>(null)
const hostRef = ref<HTMLElement | null>(null)

function measureFit() {
  const root = containerRef.value
  const host = hostRef.value
  if (!root || !host) return
  const available = host.clientWidth
  if (available === 0) return
  // `required` comes from a hidden copy of the full labels living outside the
  // control, so it never depends on the current compact state (no flip-flop).
  const required = (measureRef.value?.offsetWidth ?? root.scrollWidth) + 2
  const next = required > available + 1
  if (next !== compact.value) compact.value = next
}

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
    refresh()
    // A label change can push the control past its allocated width.
    await nextTick()
    refresh()
  },
)

function refresh() {
  measureFit()
  updateIndicator()
}

onMounted(async () => {
  refresh()
  await nextTick()
  refresh()
  if (typeof ResizeObserver !== 'undefined') {
    observer = new ResizeObserver(() => refresh())
    if (hostRef.value) observer.observe(hostRef.value)
    if (containerRef.value) observer.observe(containerRef.value)
  }
})

onUnmounted(() => {
  observer?.disconnect()
})
</script>

<template>
  <div
    ref="hostRef"
    class="relative min-w-0 max-w-full"
    :class="block ? 'flex w-full justify-end' : 'inline-flex w-fit'"
  >
    <div
      ref="containerRef"
      class="relative items-center p-1 rounded-xl bg-sunken border border-line select-none max-w-full min-w-0"
      :class="'inline-flex w-fit'"
    >
      <div
        class="m-segmented-pill pointer-events-none absolute top-1 bottom-1 left-1 rounded-lg bg-raised shadow-1 transition-all duration-200"
        :style="indicatorStyle"
      />
      <button
        v-for="opt in options"
        :key="opt.value"
        data-seg-btn
        type="button"
        :disabled="disabled"
        class="relative z-1 inline-flex items-center justify-center gap-1.5 rounded-lg font-medium transition-colors cursor-pointer disabled:cursor-not-allowed disabled:opacity-45 min-w-0"
        :class="[
          compact
            ? size === 'sm'
              ? 'px-2 py-1'
              : 'px-2.5 py-1.5'
            : size === 'sm'
              ? 'px-2.5 py-1 text-xs'
              : 'px-3 py-1.5 text-xs',
          compact ? 'shrink' : 'shrink-0',
          modelValue === opt.value ? 'text-fg font-semibold' : 'text-fg-muted hover:text-fg',
        ]"
        :title="opt.label"
        :aria-label="opt.label"
        @click="emit('update:modelValue', opt.value)"
      >
        <span v-if="opt.icon" :class="opt.icon" class="w-3.5 h-3.5 shrink-0" />
        <!-- Separate `v-if`: chaining `v-else-if` here would bind the label to the
             icon branch and silently drop every label whenever an icon exists. -->
        <span v-if="opt.label && !(compact && opt.icon)" class="whitespace-nowrap">
          {{ opt.label }}
        </span>
      </button>
    </div>

    <!-- Hidden full-label copy: the width the labels actually need. -->
    <div
      ref="measureRef"
      aria-hidden="true"
      class="invisible pointer-events-none absolute top-0 left-0 flex items-center gap-0 whitespace-nowrap p-1"
    >
      <span
        v-for="opt in options"
        :key="`m-${opt.value}`"
        class="inline-flex items-center gap-1.5 font-semibold"
        :class="size === 'sm' ? 'px-2.5 py-1 text-xs' : 'px-3 py-1.5 text-xs'"
      >
        <i v-if="opt.icon" :class="opt.icon" class="w-3.5 h-3.5" />
        {{ opt.label }}
      </span>
    </div>
  </div>
</template>

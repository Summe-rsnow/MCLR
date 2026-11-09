<script setup lang="ts">
import { ref, watch, computed } from 'vue'

export type MImageSize = 'xs' | 'sm' | 'md' | 'lg' | 'xl' | '2xl' | 'full' | 'auto'
export type MImageRadius = 'none' | 'xs' | 'sm' | 'md' | 'lg' | 'xl' | 'full' | 'auto'

const props = withDefaults(
  defineProps<{
    src?: string | null
    alt?: string
    fallbackSrc?: string | null
    fallbackIcon?: string
    loading?: 'lazy' | 'eager'
    decoding?: 'async' | 'sync' | 'auto'
    fit?: 'cover' | 'contain' | 'fill' | 'none' | 'scale-down'
    pixelated?: boolean
    /** Preset box size. `auto` (default) keeps caller-supplied dimensions. */
    size?: MImageSize
    /** Corner radius. `auto` (default) follows the `size` tier. */
    radius?: MImageRadius
  }>(),
  {
    src: null,
    alt: '',
    fallbackSrc: null,
    fallbackIcon: 'i-lucide-image',
    loading: 'lazy',
    decoding: 'async',
    fit: 'cover',
    pixelated: false,
    size: 'auto',
    radius: 'auto',
  },
)

const SIZE_CLASSES: Record<MImageSize, string> = {
  xs: 'w-5 h-5',
  sm: 'w-8 h-8',
  md: 'w-10 h-10',
  lg: 'w-14 h-14',
  xl: 'w-20 h-20',
  '2xl': 'w-44 h-44',
  full: 'w-full h-full',
  auto: '',
}

// Proportional corners: 20 px → 4 px, 32 px → 6 px, 40 px → 8 px,
// 56 px → 10 px, 80 px+ → 12 px. A tile that fills its parent (`full`)
// inherits the wrapper's shape instead of rounding twice.
const DEFAULT_RADIUS: Record<MImageSize, Exclude<MImageRadius, 'auto'>> = {
  xs: 'xs',
  sm: 'sm',
  md: 'md',
  lg: 'lg',
  xl: 'xl',
  '2xl': 'xl',
  full: 'none',
  auto: 'sm',
}

const RADIUS_CLASSES: Record<Exclude<MImageRadius, 'auto'>, string> = {
  none: 'rounded-none',
  xs: 'rounded-[4px]',
  sm: 'rounded-[6px]',
  md: 'rounded-sm',
  lg: 'rounded-md',
  xl: 'rounded-[12px]',
  full: 'rounded-full',
}

const sizeClass = computed(() => SIZE_CLASSES[props.size])
const radius = computed<Exclude<MImageRadius, 'auto'>>(() =>
  props.radius === 'auto' ? DEFAULT_RADIUS[props.size] : props.radius,
)
const radiusClass = computed(() => RADIUS_CLASSES[radius.value])

const emit = defineEmits<{
  (e: 'load', ev: Event): void
  (e: 'error', ev: Event): void
}>()

const currentSrc = ref<string | null>(props.src || props.fallbackSrc || null)
const hasError = ref(false)
const isLoaded = ref(false)

const fitClass = computed(() => {
  switch (props.fit) {
    case 'contain':
      return 'object-contain'
    case 'fill':
      return 'object-fill'
    case 'none':
      return 'object-none'
    case 'scale-down':
      return 'object-scale-down'
    case 'cover':
    default:
      return 'object-cover'
  }
})

watch(
  () => [props.src, props.fallbackSrc],
  () => {
    currentSrc.value = props.src || props.fallbackSrc || null
    hasError.value = false
    isLoaded.value = false
  },
)

function onLoad(ev: Event) {
  isLoaded.value = true
  emit('load', ev)
}

function onError(ev: Event) {
  if (currentSrc.value && props.fallbackSrc && currentSrc.value !== props.fallbackSrc) {
    currentSrc.value = props.fallbackSrc
  } else {
    currentSrc.value = null
    hasError.value = true
  }
  emit('error', ev)
}
</script>

<template>
  <div
    class="relative inline-flex items-center justify-center overflow-hidden shrink-0 select-none"
    :class="[sizeClass, radiusClass, pixelated ? '[image-rendering:pixelated]' : '']"
  >
    <img
      v-if="currentSrc"
      :src="currentSrc"
      :alt="alt"
      :loading="loading"
      :decoding="decoding"
      class="w-full h-full transition-opacity duration-200"
      :class="[fitClass, isLoaded ? 'opacity-100' : 'opacity-0']"
      @load="onLoad"
      @error="onError"
    />
    <div v-else class="w-full h-full flex items-center justify-center text-fg-subtle">
      <slot name="fallback">
        <span :class="[fallbackIcon, 'w-1/2 h-1/2 max-w-6 max-h-6 shrink-0']" />
      </slot>
    </div>
  </div>
</template>

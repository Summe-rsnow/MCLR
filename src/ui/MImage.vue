<script setup lang="ts">
import { computed } from 'vue'
import { Image } from '@miuui/vue'

export type MImageSize = 'xs' | 'sm' | 'md' | 'lg' | 'xl' | '2xl' | 'full' | 'auto'
export type MImageRadius = 'none' | 'xs' | 'sm' | 'md' | 'lg' | 'xl' | 'full'

const props = withDefaults(
  defineProps<{
    src?: string | null
    fallbackSrc?: string | null
    fallbackIcon?: string
    alt?: string
    size?: MImageSize
    radius?: MImageRadius
    fit?: 'cover' | 'contain'
    pixelated?: boolean
    loading?: 'lazy' | 'eager'
    decoding?: 'async' | 'sync' | 'auto'
  }>(),
  {
    src: null,
    fallbackSrc: null,
    fallbackIcon: 'i-lucide-image',
    alt: '',
    size: 'auto',
    radius: undefined,
    fit: 'cover',
    pixelated: false,
    loading: 'lazy',
    decoding: 'async',
  },
)

const sizeClass = computed(() => {
  switch (props.size) {
    case 'xs':
      return 'w-5 h-5'
    case 'sm':
      return 'w-8 h-8'
    case 'md':
      return 'w-10 h-10'
    case 'lg':
      return 'w-14 h-14'
    case 'xl':
      return 'w-20 h-20'
    case '2xl':
      return 'w-44 h-44'
    case 'full':
      return 'w-full h-full'
    case 'auto':
    default:
      return ''
  }
})

const effectiveRadius = computed<MImageRadius>(() => {
  if (props.radius) return props.radius
  switch (props.size) {
    case 'xs':
      return 'xs'
    case 'sm':
      return 'sm'
    case 'md':
      return 'md'
    case 'lg':
      return 'lg'
    case 'xl':
    case '2xl':
      return 'xl'
    case 'full':
      return 'none'
    case 'auto':
    default:
      return 'sm'
  }
})

const radiusClass = computed(() => {
  switch (effectiveRadius.value) {
    case 'none':
      return 'rounded-none'
    case 'xs':
      return 'rounded-[4px]'
    case 'sm':
      return 'rounded-[6px]'
    case 'md':
      return 'rounded-sm'
    case 'lg':
      return 'rounded-md'
    case 'xl':
      return 'rounded-[12px]'
    case 'full':
      return 'rounded-full'
    default:
      return 'rounded-[6px]'
  }
})

const fitClass = computed(() => (props.fit === 'contain' ? 'object-contain' : 'object-cover'))
</script>

<template>
  <Image
    :src="src"
    :fallback-src="fallbackSrc"
    :alt="alt"
    :fit="fit"
    :pixelated="pixelated"
    :lazy="loading === 'lazy'"
    radius="none"
    :class-names="{
      root: [
        'shrink-0',
        sizeClass,
        radiusClass,
        fitClass,
        pixelated ? '[image-rendering:pixelated]' : '',
      ]
        .filter(Boolean)
        .join(' '),
    }"
    :styles="{ root: { background: 'transparent' } }"
  >
    <template #fallback>
      <slot name="fallback">
        <span :class="[fallbackIcon, 'w-1/2 h-1/2 max-w-6 max-h-6 shrink-0']" />
      </slot>
    </template>
  </Image>
</template>

<script setup lang="ts">
import { ref, watch, computed } from 'vue'

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
  },
)

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
    :class="pixelated ? '[image-rendering:pixelated]' : ''"
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

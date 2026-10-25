<script setup lang="ts">
import { computed, ref, watch } from 'vue'

const props = withDefaults(
  defineProps<{
    src?: string
    name?: string
    size?: number
  }>(),
  {
    src: undefined,
    name: '',
    size: 36,
  },
)

const imgError = ref(false)
watch(
  () => props.src,
  () => {
    imgError.value = false
  },
)

const initials = computed(() => {
  if (!props.name) return '?'
  return props.name.trim().slice(0, 2).toUpperCase()
})
</script>

<template>
  <div
    class="relative inline-flex items-center justify-center rounded-lg overflow-hidden bg-primary-soft text-primary-soft-fg font-semibold select-none shrink-0"
    :style="{ width: `${size}px`, height: `${size}px`, fontSize: `${Math.round(size * 0.4)}px` }"
  >
    <img
      v-if="src && !imgError"
      :src="src"
      :alt="name"
      class="w-full h-full object-cover"
      style="image-rendering: pixelated"
      @error="imgError = true"
     loading="lazy" decoding="async"/>
    <span v-else>{{ initials }}</span>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import MImage from './MImage.vue'

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

const initials = computed(() => {
  if (!props.name) return '?'
  return props.name.trim().slice(0, 2).toUpperCase()
})

// Corner radius stays proportional to the avatar box: 24 px → 4 px,
// 36 px → 6 px, 48 px → 8 px, larger → 10 px.
const radiusClass = computed(() => {
  if (props.size <= 24) return 'rounded-[4px]'
  if (props.size <= 36) return 'rounded-[6px]'
  if (props.size <= 48) return 'rounded-sm'
  return 'rounded-md'
})
</script>

<template>
  <div
    class="relative inline-flex items-center justify-center overflow-hidden bg-primary-soft text-primary-soft-fg font-semibold select-none shrink-0"
    :class="radiusClass"
    :style="{ width: `${size}px`, height: `${size}px`, fontSize: `${Math.round(size * 0.4)}px` }"
  >
    <MImage v-if="src" :src="src" :alt="name" pixelated size="full" radius="none">
      <template #fallback>
        <span>{{ initials }}</span>
      </template>
    </MImage>
    <span v-else>{{ initials }}</span>
  </div>
</template>

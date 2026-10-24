<script setup lang="ts">
import { computed } from 'vue'

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
</script>

<template>
  <div
    class="relative inline-flex items-center justify-center rounded-lg overflow-hidden bg-primaryContainer text-primary font-semibold select-none shrink-0"
    :style="{ width: `${size}px`, height: `${size}px`, fontSize: `${Math.round(size * 0.4)}px` }"
  >
    <img
      v-if="src"
      :src="src"
      :alt="name"
      class="w-full h-full object-cover"
    />
    <span v-else>{{ initials }}</span>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(
  defineProps<{
    glass?: boolean
    padding?: 'none' | 'sm' | 'md' | 'lg'
  }>(),
  {
    glass: undefined,
    padding: 'md',
  },
)

const paddingClass = computed(() => {
  switch (props.padding) {
    case 'none':
      return 'p-0'
    case 'sm':
      return 'p-3'
    case 'lg':
      return 'p-6'
    case 'md':
    default:
      return 'p-4'
  }
})
</script>

<template>
  <div
    class="m-card rounded-xl transition-all"
    :class="[
      paddingClass,
      glass === true ? 'm-card--force-glass' : '',
      glass === false ? 'm-card--force-flat' : '',
    ]"
  >
    <slot />
  </div>
</template>

<style scoped>
.m-card--force-glass {
  background-color: rgb(var(--c-surface) / 0.65) !important;
  border: 1px solid rgb(255 255 255 / 0.25) !important;
  backdrop-filter: blur(24px) saturate(1.6) !important;
  -webkit-backdrop-filter: blur(24px) saturate(1.6) !important;
}

.m-card--force-flat {
  background-color: rgb(var(--c-surface)) !important;
  border: 1px solid rgb(var(--c-outline) / 0.7) !important;
  backdrop-filter: none !important;
  -webkit-backdrop-filter: none !important;
}
</style>

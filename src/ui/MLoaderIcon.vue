<script setup lang="ts">
import { computed } from 'vue'
import { getInstanceIconMeta } from './icons'

const props = withDefaults(
  defineProps<{
    kind?: string | null
    icon?: string | null
    size?: 24 | 32 | 40 | 56
  }>(),
  {
    kind: 'vanilla',
    icon: null,
    size: 32,
  },
)

const meta = computed(() => getInstanceIconMeta(props.icon, props.kind))

const sizeClasses = computed(() => {
  switch (props.size) {
    case 24:
      return { tile: 'w-6 h-6 rounded-md', icon: 'w-3.5 h-3.5' }
    case 40:
      return { tile: 'w-10 h-10 rounded-xl', icon: 'w-5 h-5' }
    case 56:
      return { tile: 'w-14 h-14 rounded-2xl', icon: 'w-7 h-7' }
    case 32:
    default:
      return { tile: 'w-8 h-8 rounded-lg', icon: 'w-4 h-4' }
  }
})
</script>

<template>
  <div
    class="m-loader-tile inline-flex items-center justify-center shrink-0 select-none"
    :class="sizeClasses.tile"
    :style="{ '--loader-hue': String(meta.hue) }"
    :title="meta.label"
  >
    <span :class="[meta.icon, sizeClasses.icon]" />
  </div>
</template>

<style scoped>
.m-loader-tile {
  background-color: oklch(0.92 0.06 var(--loader-hue, 140));
  color: oklch(0.42 0.13 var(--loader-hue, 140));
}

:global(:root[data-mode='dark']) .m-loader-tile {
  background-color: oklch(0.32 0.07 var(--loader-hue, 140));
  color: oklch(0.85 0.09 var(--loader-hue, 140));
}
</style>

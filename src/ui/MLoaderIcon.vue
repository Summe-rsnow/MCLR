<script setup lang="ts">
import { computed } from 'vue'
import { getInstanceIconMeta } from './icons'
import MImage from './MImage.vue'

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
  // `image` is deliberately larger than `icon`: official loader artwork is
  // detailed and unreadable at lucide-sized (16px) dimensions.
  switch (props.size) {
    case 24:
      return { tile: 'w-6 h-6 rounded-md', icon: 'w-3.5 h-3.5', image: 'w-[18px] h-[18px]' }
    case 40:
      return { tile: 'w-10 h-10 rounded-xl', icon: 'w-5 h-5', image: 'w-[30px] h-[30px]' }
    case 56:
      return { tile: 'w-14 h-14 rounded-2xl', icon: 'w-7 h-7', image: 'w-[44px] h-[44px]' }
    case 32:
    default:
      return { tile: 'w-8 h-8 rounded-lg', icon: 'w-4 h-4', image: 'w-6 h-6' }
  }
})
</script>

<template>
  <div
    class="m-loader-tile inline-flex items-center justify-center shrink-0 select-none overflow-hidden"
    :class="sizeClasses.tile"
    :style="{ '--loader-hue': String(meta.hue) }"
    :title="meta.label"
  >
    <MImage
      v-if="meta.image"
      :src="meta.image"
      :alt="meta.label"
      pixelated
      fit="contain"
      :class="sizeClasses.image"
    >
      <template #fallback>
        <span :class="[meta.icon, sizeClasses.icon]" />
      </template>
    </MImage>
    <span v-else :class="[meta.icon, sizeClasses.icon]" />
  </div>
</template>

<style scoped>
.m-loader-tile {
  /* Light chips in both themes: loader logos (Forge's grey anvil, Fabric's woven
     swatch) are dark artwork and disappear on a dark tile. */
  background-color: oklch(0.9 0.045 var(--loader-hue, 140));
  color: oklch(0.38 0.11 var(--loader-hue, 140));
  box-shadow: inset 0 0 0 1px oklch(0.82 0.05 var(--loader-hue, 140));
}

:global(:root[data-mode='dark']) .m-loader-tile {
  background-color: oklch(0.93 0.03 var(--loader-hue, 140));
  color: oklch(0.4 0.1 var(--loader-hue, 140));
  box-shadow: inset 0 0 0 1px oklch(0.88 0.035 var(--loader-hue, 140));
}
</style>

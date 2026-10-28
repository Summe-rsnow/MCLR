<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(
  defineProps<{
    width?: string
    height?: string
    rounded?: boolean | string
  }>(),
  {
    width: undefined,
    height: undefined,
    rounded: true,
  },
)

const inlineStyle = computed(() => {
  const s: Record<string, string> = {}
  if (props.width) s.width = props.width
  if (props.height) s.height = props.height
  return s
})

const roundedClass = computed(() => {
  if (props.rounded === false) return ''
  if (typeof props.rounded === 'string') {
    return `rounded-${props.rounded}`
  }
  return 'rounded-xl'
})
</script>

<template>
  <div
    class="m-skeleton"
    :class="[roundedClass, !width ? 'w-full' : '', !height ? 'h-4' : '']"
    :style="inlineStyle"
  />
</template>

<style scoped>
.m-skeleton {
  position: relative;
  overflow: hidden;
  background-color: rgb(var(--c-sunken, 236 239 243));
}

/*
 * Geometrically identical to the old background-position animation: that used
 * background-size: 220% with background-position 120% -> -120%, i.e. an offset of
 * (W - 2.2W)*P/100 = -1.44W -> +1.44W. Translating a 2.2W-wide layer by
 * -1.44W/+1.44W is -65.454545%/+65.454545%. The frozen state (reduced motion)
 * keeps the original 120% position, i.e. -65.454545%.
 */
.m-skeleton::after {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  width: 220%;
  height: 100%;
  background-image: linear-gradient(
    100deg,
    transparent 30%,
    rgb(var(--c-fg, 19 25 36) / 0.06) 50%,
    transparent 70%
  );
  transform: translateX(-65.454545%);
  will-change: transform;
  animation: m-shimmer-slide 1.4s var(--ease, ease-in-out) infinite;
}

@keyframes m-shimmer-slide {
  from {
    transform: translateX(-65.454545%);
  }
  to {
    transform: translateX(65.454545%);
  }
}
</style>

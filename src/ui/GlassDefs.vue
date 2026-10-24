<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { buildDisplacementMap } from './theme/glass'

const displacementMap = ref<string | null>(null)

onMounted(() => {
  displacementMap.value = buildDisplacementMap()
})
</script>

<template>
  <svg class="pointer-events-none fixed w-0 h-0 overflow-hidden" aria-hidden="true">
    <defs>
      <filter
        id="mclr-refract"
        x="-10%"
        y="-10%"
        width="120%"
        height="120%"
        color-interpolation-filters="sRGB"
      >
        <feGaussianBlur in="SourceGraphic" stdDeviation="1.5" result="blurred" />
        <feImage
          v-if="displacementMap"
          :href="displacementMap"
          result="map"
          preserveAspectRatio="none"
        />
        <feDisplacementMap
          v-if="displacementMap"
          in="blurred"
          in2="map"
          scale="18"
          xChannelSelector="R"
          yChannelSelector="G"
        />
      </filter>
    </defs>
  </svg>
</template>

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
  background-color: rgb(var(--c-sunken, 236 239 243));
  background-image: linear-gradient(
    100deg,
    transparent 30%,
    rgb(var(--c-fg, 19 25 36) / 0.06) 50%,
    transparent 70%
  );
  background-size: 220% 100%;
  background-repeat: no-repeat;
  background-position: 120% 0;
  animation: m-shimmer 1.4s var(--ease, ease-in-out) infinite;
}

@keyframes m-shimmer {
  to {
    background-position: -120% 0;
  }
}
</style>

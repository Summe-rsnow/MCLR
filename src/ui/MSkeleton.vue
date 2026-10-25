<script setup lang="ts">
withDefaults(
  defineProps<{
    width?: string
    height?: string
    rounded?: boolean
  }>(),
  {
    width: '100%',
    height: '1rem',
    rounded: true,
  },
)
</script>

<template>
  <div
    class="m-skeleton"
    :class="rounded ? 'rounded-lg' : ''"
    :style="{ width, height }"
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

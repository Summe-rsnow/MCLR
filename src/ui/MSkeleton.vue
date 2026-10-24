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
    class="bg-surfaceVariant animate-pulse"
    :class="rounded ? 'rounded-lg' : ''"
    :style="{ width, height }"
  />
</template>

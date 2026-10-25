<script setup lang="ts">
withDefaults(
  defineProps<{
    maxHeight?: string
  }>(),
  {
    maxHeight: '100%',
  },
)
</script>

<template>
  <div class="m-scroll overflow-y-auto p-1 -m-1" :style="{ maxHeight }">
    <slot />
  </div>
</template>

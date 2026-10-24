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
  <div
    class="overflow-y-auto overflow-x-hidden"
    :style="{ maxHeight }"
  >
    <slot />
  </div>
</template>

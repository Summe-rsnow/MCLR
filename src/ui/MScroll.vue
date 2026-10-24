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
    class="overflow-y-auto overflow-x-hidden scrollbar-thin scrollbar-thumb-outline/50 hover:scrollbar-thumb-outline"
    :style="{ maxHeight }"
  >
    <slot />
  </div>
</template>

<style scoped>
.scrollbar-thin {
  scrollbar-width: thin;
  scrollbar-color: rgb(var(--c-outline) / 0.5) transparent;
}
</style>

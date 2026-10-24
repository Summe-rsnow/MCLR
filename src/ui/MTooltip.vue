<script setup lang="ts">
import { ref } from 'vue'

defineProps<{
  text: string
}>()

const visible = ref(false)
</script>

<template>
  <div
    class="relative inline-flex items-center"
    @mouseenter="visible = true"
    @mouseleave="visible = false"
  >
    <slot />
    <Transition name="fade">
      <div
        v-if="visible"
        class="m-overlay absolute bottom-full left-1/2 -translate-x-1/2 mb-1.5 px-2.5 py-1 text-xs rounded-lg text-fg pointer-events-none whitespace-nowrap z-50"
      >
        {{ text }}
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.14s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>

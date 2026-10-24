<script setup lang="ts">
import { ref } from 'vue'

withDefaults(
  defineProps<{
    text: string
  }>(),
  {},
)

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
        class="absolute bottom-full left-1/2 -translate-x-1/2 mb-1.5 px-2 py-1 text-xs rounded-md bg-neutral-900 text-white shadow-md pointer-events-none whitespace-nowrap z-50"
      >
        {{ text }}
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.15s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>

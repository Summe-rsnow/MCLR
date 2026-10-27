<script setup lang="ts">
import { useToast } from './useToast'

const { toasts, remove } = useToast()
</script>

<template>
  <div class="fixed top-12 right-6 z-50 flex flex-col gap-2 pointer-events-none">
    <TransitionGroup name="toast">
      <div
        v-for="item in toasts"
        :key="item.id"
        class="m-overlay m-refract pointer-events-auto flex items-center gap-3 px-4 py-3 rounded-xl text-sm max-w-[min(24rem,calc(100vw-3rem))] text-fg"
      >
        <span
          class="w-4 h-4 shrink-0"
          :class="{
            'i-lucide-info text-primary': item.type === 'info',
            'i-lucide-circle-check text-success': item.type === 'success',
            'i-lucide-triangle-alert text-warning': item.type === 'warning',
            'i-lucide-circle-alert text-danger': item.type === 'danger',
          }"
        />
        <span class="flex-1 text-fg select-text">{{ item.message }}</span>
        <button
          type="button"
          class="text-fg-muted hover:text-fg cursor-pointer p-0.5 rounded transition-colors"
          @click="remove(item.id)"
        >
          <span class="i-lucide-x w-3.5 h-3.5 block" />
        </button>
      </div>
    </TransitionGroup>
  </div>
</template>

<style scoped>
.toast-enter-active,
.toast-leave-active {
  transition: all 0.22s cubic-bezier(0.2, 0.8, 0.2, 1);
}
.toast-enter-from {
  opacity: 0;
  transform: translateY(-10px) scale(0.96);
}
.toast-leave-to {
  opacity: 0;
  transform: translateY(-10px) scale(0.96);
}
</style>

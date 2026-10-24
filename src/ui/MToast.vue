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
        class="m-card pointer-events-auto flex items-center gap-3 px-4 py-3 rounded-xl shadow-lg border text-sm max-w-sm"
        :class="{
          'border-outline/60 text-text bg-surface': item.type === 'info',
          'border-success/40 text-success bg-surface': item.type === 'success',
          'border-warning/40 text-warning bg-surface': item.type === 'warning',
          'border-danger/40 text-danger bg-surface': item.type === 'danger',
        }"
      >
        <span
          class="w-4 h-4 shrink-0"
          :class="{
            'i-lucide-info text-primary': item.type === 'info',
            'i-lucide-check-circle text-success': item.type === 'success',
            'i-lucide-alert-triangle text-warning': item.type === 'warning',
            'i-lucide-alert-circle text-danger': item.type === 'danger',
          }"
        />
        <span class="flex-1 text-text">{{ item.message }}</span>
        <button
          type="button"
          class="text-textMuted hover:text-text cursor-pointer p-0.5 rounded"
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
  transition: all 0.25s ease;
}
.toast-enter-from {
  opacity: 0;
  transform: translateY(-12px) scale(0.95);
}
.toast-leave-to {
  opacity: 0;
  transform: translateY(-12px) scale(0.95);
}
</style>

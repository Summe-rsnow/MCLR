<script setup lang="ts">
import MSkeleton from './MSkeleton.vue'

withDefaults(
  defineProps<{
    rows?: number
    variant?: 'row' | 'card' | 'tile'
  }>(),
  { rows: 6, variant: 'row' },
)
</script>

<template>
  <div
    data-testid="m-skeleton-list"
    :class="
      variant === 'tile'
        ? 'grid gap-3 grid-cols-[repeat(auto-fill,minmax(200px,1fr))]'
        : 'flex flex-col gap-3'
    "
    aria-busy="true"
  >
    <template v-for="n in rows" :key="n">
      <div v-if="variant === 'row'" class="flex items-center gap-3 px-3 py-2.5">
        <MSkeleton width="2.25rem" height="2.25rem" />
        <div class="flex-1 flex flex-col gap-2">
          <MSkeleton :width="n % 2 ? '45%' : '60%'" height="0.9rem" />
          <MSkeleton width="30%" height="0.7rem" />
        </div>
      </div>
      <div v-else-if="variant === 'card'" class="m-card rounded-2xl p-4 flex items-center gap-3">
        <MSkeleton width="3rem" height="3rem" />
        <div class="flex-1 flex flex-col gap-2">
          <MSkeleton width="40%" height="1rem" />
          <MSkeleton width="65%" height="0.75rem" />
        </div>
      </div>
      <div v-else class="m-card rounded-2xl overflow-hidden">
        <MSkeleton height="7.5rem" :rounded="false" />
        <div class="p-3 flex flex-col gap-2">
          <MSkeleton width="55%" height="0.85rem" />
          <MSkeleton width="35%" height="0.7rem" />
        </div>
      </div>
    </template>
  </div>
</template>

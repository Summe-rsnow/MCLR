<script setup lang="ts">
import { PageHeader } from '@miuui/vue'

withDefaults(
  defineProps<{
    title: string
    description?: string
    icon?: string
  }>(),
  {
    description: undefined,
    icon: undefined,
  },
)
</script>

<template>
  <PageHeader :title="title" :description="description">
    <template v-if="$slots.icon || icon" #lead>
      <slot name="icon">
        <div
          v-if="icon"
          class="w-12 h-12 rounded-xl bg-primary-soft text-primary-soft-fg flex items-center justify-center shrink-0 shadow-sm"
        >
          <span :class="icon" class="w-6 h-6" />
        </div>
      </slot>
    </template>
    <template v-if="$slots.badge" #badge>
      <slot name="badge" />
    </template>
    <template v-if="$slots.actions" #actions>
      <slot name="actions" />
    </template>
  </PageHeader>
</template>

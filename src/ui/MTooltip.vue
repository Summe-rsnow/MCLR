<script setup lang="ts">
import { Tooltip } from '@miuui/vue'

withDefaults(
  defineProps<{
    text: string
    placement?: 'top' | 'bottom'
  }>(),
  {
    placement: 'top',
  },
)
</script>

<template>
  <Tooltip :side="placement" :delay-duration="140" max-width="280px">
    <template #trigger>
      <span class="inline-flex items-center">
        <slot />
      </span>
    </template>
    <span class="select-text">{{ text }}</span>
  </Tooltip>
</template>

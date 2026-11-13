<script setup lang="ts">
import { Collapse } from '@miuui/vue'

defineProps<{ open: boolean }>()
</script>

<template>
  <Collapse :open="open" :class-names="{ root: 'm-collapse', content: 'm-collapse-inner' }">
    <slot />
  </Collapse>
</template>

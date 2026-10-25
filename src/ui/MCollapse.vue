<script setup lang="ts">
defineProps<{ open: boolean }>()
</script>

<template>
  <div class="m-collapse" :data-open="open" :aria-hidden="!open">
    <div class="m-collapse-inner">
      <slot />
    </div>
  </div>
</template>

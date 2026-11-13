<script setup lang="ts">
import { ToastViewport } from '@miuui/vue'
</script>

<template>
  <ToastViewport
    position="top-right"
    :class-names="{
      toast: 'm-overlay m-refract',
      title: 'select-text',
      description: 'select-text',
    }"
  />
</template>

<script setup lang="ts">
import { ref } from 'vue'
import AppShell from './features/shell/AppShell.vue'
import StartupSplash from './features/shell/StartupSplash.vue'
import { useThemeStore } from './stores/theme'

const theme = useThemeStore()
const showSplash = ref(theme.startupSplash && theme.motion !== 'off')
</script>

<template>
  <AppShell />
  <StartupSplash v-if="showSplash" @done="showSplash = false" />
</template>

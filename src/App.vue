<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { ThemeProvider } from '@miuui/vue'
import AppShell from './features/shell/AppShell.vue'
import StartupSplash from './features/shell/StartupSplash.vue'
import { useThemeStore } from './stores/theme'
import { resolveEffectiveMode } from './ui/theme'

const { locale } = useI18n()
const theme = useThemeStore()
const showSplash = ref(theme.startupSplash && theme.motion !== 'off')
const systemDark = ref(
  typeof window !== 'undefined'
    ? (window.matchMedia?.('(prefers-color-scheme: dark)').matches ?? false)
    : false,
)
let mediaQuery: MediaQueryList | undefined

function onMediaChange(e: MediaQueryListEvent) {
  systemDark.value = e.matches
}

onMounted(() => {
  if (typeof window === 'undefined' || !window.matchMedia) return
  mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')
  systemDark.value = mediaQuery.matches
  mediaQuery.addEventListener('change', onMediaChange)
})

onBeforeUnmount(() => {
  mediaQuery?.removeEventListener('change', onMediaChange)
})

const resolvedThemeMode = computed<'light' | 'dark'>(() => {
  if (theme.mode === 'system') return systemDark.value ? 'dark' : 'light'
  return resolveEffectiveMode(theme.mode)
})

const miuLocale = computed(() => (locale.value === 'zh-CN' ? 'zh-CN' : 'en-US'))
</script>

<template>
  <ThemeProvider
    :theme="resolvedThemeMode"
    :accent-seed="theme.seed"
    :material="theme.material === 'glass' ? 'glass' : 'solid'"
    :motion-tier="theme.motion === 'off' ? 'none' : theme.motion"
    :locale="miuLocale"
    toast-position="top-right"
    as="div"
    style="display: contents"
  >
    <AppShell />
    <StartupSplash v-if="showSplash" @done="showSplash = false" />
  </ThemeProvider>
</template>

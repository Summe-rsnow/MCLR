<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { ipcClient } from './ipc/client'

const { t } = useI18n()
const version = ref('0.0.1')

onMounted(async () => {
  try {
    const info = await ipcClient.appInfo()
    version.value = info.version
  } catch {
    // Fallback in non-Tauri test environments
  }
})
</script>

<template>
  <main class="w-screen h-screen flex flex-col items-center justify-center bg-surface text-text">
    <h1 class="text-3xl font-bold text-primary">{{ t('app.name') }}</h1>
    <p class="mt-2 text-sm text-textMuted">{{ t('app.tagline') }} · v{{ version }}</p>
  </main>
</template>

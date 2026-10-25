<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useInstancesStore } from '../instances/store'
import { MPageHeader, MSkeleton } from '../../ui'
import ToolSkinSection from './components/ToolSkinSection.vue'
import ToolCacheSection from './components/ToolCacheSection.vue'
import ToolDownloadSection from './components/ToolDownloadSection.vue'
import ToolShortcutSection from './components/ToolShortcutSection.vue'

const { t } = useI18n()
const instancesStore = useInstancesStore()

const initialized = ref(false)

onMounted(async () => {
  try {
    await instancesStore.fetchInstances()
  } finally {
    initialized.value = true
  }
})
</script>

<template>
  <div class="m-page">
    <!-- Header -->
    <MPageHeader
      :title="t('tools.title')"
      :description="t('tools.subtitle')"
      icon="i-lucide-wrench"
    />

    <div v-if="!initialized" class="grid grid-cols-1 lg:grid-cols-2 gap-5 items-start">
      <!-- Left Column Skeletons -->
      <div class="flex flex-col gap-5">
        <div v-for="n in 2" :key="n" class="m-card p-5 flex flex-col space-y-4">
          <div class="flex items-center gap-3">
            <MSkeleton class="w-10 h-10 rounded-xl shrink-0" />
            <div class="flex-1 min-w-0 space-y-2">
              <MSkeleton class="h-4 w-32" />
              <MSkeleton class="h-3 w-48" />
            </div>
          </div>
          <div class="space-y-3 pt-2">
            <MSkeleton class="h-9 w-full rounded-lg" />
            <MSkeleton class="h-9 w-full rounded-lg" />
          </div>
        </div>
      </div>
      <!-- Right Column Skeletons -->
      <div class="flex flex-col gap-5">
        <div v-for="n in 2" :key="n" class="m-card p-5 flex flex-col space-y-4">
          <div class="flex items-center gap-3">
            <MSkeleton class="w-10 h-10 rounded-xl shrink-0" />
            <div class="flex-1 min-w-0 space-y-2">
              <MSkeleton class="h-4 w-32" />
              <MSkeleton class="h-3 w-48" />
            </div>
          </div>
          <div class="space-y-3 pt-2">
            <MSkeleton class="h-9 w-full rounded-lg" />
            <MSkeleton class="h-9 w-full rounded-lg" />
          </div>
        </div>
      </div>
    </div>

    <template v-else>
      <div class="grid grid-cols-1 lg:grid-cols-2 gap-5 items-start">
        <!-- Left Column: Card 1 (Skin) & Card 3 (Downloader) -->
        <div class="flex flex-col gap-5">
          <ToolSkinSection />
          <ToolDownloadSection />
        </div>

        <!-- Right Column: Card 2 (Cache) & Card 4 (Shortcut) -->
        <div class="flex flex-col gap-5">
          <ToolCacheSection />
          <ToolShortcutSection />
        </div>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import type { ScreenshotInfo } from '../../../ipc/client'
import { ipcClient } from '../../../ipc/client'
import {
  MCard,
  MButton,
  MIconButton,
  MDialog,
  MEmpty,
  useToast,
  useDialog,
} from '../../../ui'

const props = defineProps<{
  instanceId: string
}>()

const { t } = useI18n()
const toast = useToast()
const { confirm } = useDialog()

const screenshots = ref<ScreenshotInfo[]>([])
const loading = ref(false)
const previewTarget = ref<ScreenshotInfo | null>(null)

async function fetchScreenshots() {
  loading.value = true
  try {
    screenshots.value = await ipcClient.screenshotsList(props.instanceId)
  } catch (e: unknown) {
    toast.danger(String(e))
  } finally {
    loading.value = false
  }
}

async function handleOpenFolder() {
  try {
    await ipcClient.instanceOpenFolder(props.instanceId)
  } catch (e: unknown) {
    toast.danger(String(e))
  }
}

async function handleDelete(item: ScreenshotInfo) {
  const ok = await confirm({
    title: t('instanceDetail.screenshots.deleteConfirmTitle'),
    content: t('instanceDetail.screenshots.deleteConfirm', { name: item.fileName }),
  })
  if (!ok) return

  try {
    await ipcClient.screenshotDelete(props.instanceId, item.fileName)
    screenshots.value = screenshots.value.filter((s) => s.fileName !== item.fileName)
    toast.success(t('instanceDetail.screenshots.deleteSuccess'))
    if (previewTarget.value?.fileName === item.fileName) {
      previewTarget.value = null
    }
  } catch (e: unknown) {
    toast.danger(String(e))
  }
}

function formatBytes(bytes: number): string {
  if (bytes <= 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(1024))
  return `${(bytes / Math.pow(1024, i)).toFixed(1)} ${units[i]}`
}

onMounted(() => {
  fetchScreenshots()
})
</script>

<template>
  <div class="max-w-4xl space-y-4">
    <!-- Top Bar -->
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-2">
        <h2 class="text-base font-bold text-fg">
          {{ t('instanceDetail.tabs.screenshots') }}
        </h2>
        <span class="text-xs text-fg-muted font-mono">
          ({{ screenshots.length }})
        </span>
      </div>
      <div class="flex items-center gap-2">
        <MButton variant="tonal" size="sm" @click="handleOpenFolder">
          <span class="w-4 h-4 i-lucide-folder-open mr-1" />
          {{ t('instanceDetail.screenshots.openFolder') }}
        </MButton>
        <MButton variant="tonal" size="sm" :loading="loading" @click="fetchScreenshots">
          <span class="w-4 h-4 i-lucide-refresh-cw mr-1" />
          {{ t('instanceDetail.screenshots.refresh') }}
        </MButton>
      </div>
    </div>

    <!-- Empty State -->
    <div v-if="screenshots.length === 0" class="py-16">
      <MEmpty
        :title="t('instanceDetail.screenshots.empty')"
        icon="i-lucide-image"
      />
    </div>

    <!-- Screenshots Grid -->
    <div v-else class="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-4">
      <MCard
        v-for="item in screenshots"
        :key="item.fileName"
        variant="well"
        hoverable
        padding="none"
        class="overflow-hidden group flex flex-col justify-between transition-all"
      >
        <!-- Thumbnail / Image -->
        <div
          class="aspect-video bg-sunken/50 relative overflow-hidden cursor-pointer flex items-center justify-center"
          @click="previewTarget = item"
        >
          <img
            v-if="item.dataUrl"
            :src="item.dataUrl"
            :alt="item.fileName"
            class="w-full h-full object-cover group-hover:scale-105 transition-transform duration-300"
          />
          <span v-else class="w-10 h-10 i-lucide-image text-fg-muted/40" />
          <div class="absolute inset-0 bg-black/40 opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center text-white">
            <span class="w-6 h-6 i-lucide-maximize-2" />
          </div>
        </div>

        <!-- Info Bar -->
        <div class="p-3 flex items-center justify-between gap-2 border-t border-line">
          <div class="min-w-0">
            <div class="text-xs font-semibold text-fg truncate">
              {{ item.fileName }}
            </div>
            <div class="text-[11px] text-fg-muted mt-0.5">
              {{ formatBytes(item.size) }}
            </div>
          </div>

          <MIconButton
            icon="i-lucide-trash-2"
            size="sm"
            class="text-danger hover:bg-danger/10 shrink-0"
            :title="t('instanceDetail.screenshots.delete')"
            @click="handleDelete(item)"
          />
        </div>
      </MCard>
    </div>

    <!-- Preview Modal -->
    <MDialog
      :open="!!previewTarget"
      :title="previewTarget?.fileName || t('instanceDetail.screenshots.preview')"
      @update:open="!$event && (previewTarget = null)"
    >
      <div class="py-2 flex items-center justify-center max-h-[70vh] overflow-hidden bg-sunken/30 rounded-xl">
        <img
          v-if="previewTarget?.dataUrl"
          :src="previewTarget.dataUrl"
          :alt="previewTarget.fileName"
          class="max-w-full max-h-[65vh] object-contain rounded-lg"
        />
        <div v-else class="py-20 text-fg-muted">
          {{ previewTarget?.path }}
        </div>
      </div>
      <template #footer>
        <div class="flex items-center justify-between w-full">
          <div class="text-xs text-fg-muted">
            {{ previewTarget ? formatBytes(previewTarget.size) : '' }}
          </div>
          <div class="flex gap-2">
            <MButton
              variant="danger"
              size="sm"
              @click="previewTarget && handleDelete(previewTarget)"
            >
              <span class="w-4 h-4 i-lucide-trash-2 mr-1" />
              {{ t('instanceDetail.screenshots.delete') }}
            </MButton>
            <MButton variant="tonal" size="sm" @click="previewTarget = null">
              {{ t('common.close') }}
            </MButton>
          </div>
        </div>
      </template>
    </MDialog>
  </div>
</template>

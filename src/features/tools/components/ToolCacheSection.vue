<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { ipcClient, type CacheSizeInfo } from '../../../ipc/client'
import { useToast } from '../../../ui/useToast'
import { useErrorText } from '../../../composables/useErrorText'
import { useDialog } from '../../../ui/useDialog'
import { MCard, MButton, MCheckbox, MHelpTooltip } from '../../../ui'

const { t } = useI18n()
const toast = useToast()
const dialog = useDialog()
const { formatError } = useErrorText()

const cacheInfo = ref<CacheSizeInfo>({
  cacheBytes: 0,
  logsBytes: 0,
  skinCacheBytes: 0,
  failedTaskBytes: 0,
  totalBytes: 0,
})
const cleanCacheChecked = ref(true)
const cleanLogsChecked = ref(false)
const cleanSkinsChecked = ref(false)
// Leftovers of failed tasks are the one category that is always safe to delete.
const cleanFailedTasksChecked = ref(true)
const cacheCleaning = ref(false)
const cacheCleanedNotice = ref(false)

async function loadCacheInfo() {
  try {
    cacheInfo.value = await ipcClient.toolsCacheInfo()
  } catch (err: unknown) {
    console.error('Failed to load cache info:', err)
  }
}

function formatBytes(bytes: number): string {
  if (bytes <= 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`
}

async function cleanCache() {
  const selectedCategories: string[] = []
  let selectedBytes = 0
  if (cleanCacheChecked.value) {
    selectedCategories.push(t('tools.cache.itemCache'))
    selectedBytes += cacheInfo.value.cacheBytes
  }
  if (cleanLogsChecked.value) {
    selectedCategories.push(t('tools.cache.itemLogs'))
    selectedBytes += cacheInfo.value.logsBytes
  }
  if (cleanSkinsChecked.value) {
    selectedCategories.push(t('tools.cache.itemSkins'))
    selectedBytes += cacheInfo.value.skinCacheBytes
  }
  if (cleanFailedTasksChecked.value) {
    selectedCategories.push(t('tools.cache.failedTasks'))
    // A full cache wipe already removes these files; counting them twice would overstate it.
    if (!cleanCacheChecked.value) {
      selectedBytes += cacheInfo.value.failedTaskBytes
    }
  }

  if (selectedCategories.length === 0) {
    toast.warning(t('tools.cache.noneSelected'))
    return
  }

  const confirmed = await dialog.confirmDanger({
    title: t('tools.cache.confirmTitle'),
    content: t('tools.cache.confirmMessage', {
      categories: selectedCategories.join('、'),
      size: formatBytes(selectedBytes),
    }),
    detail: t('tools.cache.confirmDetail'),
    confirmText: t('tools.cache.cleanNow'),
    cancelText: t('common.cancel'),
  })

  if (!confirmed) return

  cacheCleaning.value = true
  try {
    // A full cache wipe already removes the leftovers, so the narrow pass only runs on its own.
    if (cleanCacheChecked.value || cleanLogsChecked.value || cleanSkinsChecked.value) {
      cacheInfo.value = await ipcClient.toolsCacheClear(
        cleanCacheChecked.value,
        cleanLogsChecked.value,
        cleanSkinsChecked.value,
      )
    }
    if (cleanFailedTasksChecked.value && !cleanCacheChecked.value) {
      cacheInfo.value = await ipcClient.toolsCleanupFailedTasks()
    }
    toast.success(t('tools.cache.cleaned'))
    cacheCleanedNotice.value = true
    setTimeout(() => {
      cacheCleanedNotice.value = false
    }, 5000)
  } catch (err: unknown) {
    toast.danger(formatError(err))
  } finally {
    cacheCleaning.value = false
  }
}

onMounted(() => {
  loadCacheInfo()
})

defineExpose({
  loadCacheInfo,
})
</script>

<template>
  <MCard hoverable class="p-5 flex flex-col space-y-4 m-stagger" :style="{ '--i': 1 }">
    <div>
      <div class="flex items-center gap-3">
        <div
          class="w-10 h-10 rounded-xl bg-primary-soft text-primary-soft-fg flex items-center justify-center shrink-0"
        >
          <span class="i-lucide-trash w-5 h-5 inline-block" />
        </div>
        <div class="flex-1 min-w-0">
          <div class="flex items-center gap-1.5">
            <h2 class="text-base font-semibold text-fg">{{ t('tools.cache.title') }}</h2>
            <MHelpTooltip :text="t('tools.cache.desc')" />
          </div>
          <p class="text-xs text-fg-muted mt-0.5 truncate">{{ t('tools.cache.desc') }}</p>
        </div>
      </div>
      <div class="mt-4 space-y-3">
        <div class="flex items-center justify-between p-3 rounded-xl m-well">
          <MCheckbox v-model="cleanCacheChecked" :label="t('tools.cache.itemCache')" />
          <span class="text-xs font-mono font-semibold text-fg tabular-nums">
            {{ formatBytes(cacheInfo.cacheBytes) }}
          </span>
        </div>

        <div class="flex items-center justify-between p-3 rounded-xl m-well">
          <MCheckbox v-model="cleanLogsChecked" :label="t('tools.cache.itemLogs')" />
          <span class="text-xs font-mono font-semibold text-fg tabular-nums">
            {{ formatBytes(cacheInfo.logsBytes) }}
          </span>
        </div>

        <div class="flex items-center justify-between p-3 rounded-xl m-well">
          <MCheckbox v-model="cleanSkinsChecked" :label="t('tools.cache.itemSkins')" />
          <span class="text-xs font-mono font-semibold text-fg tabular-nums">
            {{ formatBytes(cacheInfo.skinCacheBytes) }}
          </span>
        </div>

        <div class="flex items-center justify-between p-3 rounded-xl m-well">
          <MCheckbox v-model="cleanFailedTasksChecked" :label="t('tools.cache.failedTasks')" />
          <span class="text-xs font-mono font-semibold text-fg tabular-nums">
            {{ formatBytes(cacheInfo.failedTaskBytes) }}
          </span>
        </div>
      </div>
    </div>
    <div v-if="cacheCleanedNotice" class="text-xs text-primary font-medium">
      {{ t('tools.cache.cleaned') }}
    </div>
    <div class="pt-2 border-t border-line flex items-center justify-between">
      <div class="text-xs text-fg-muted">
        {{ t('tools.cache.totalSize') }}:
        <span class="font-mono font-bold text-sm text-fg ml-1">
          {{ formatBytes(cacheInfo.totalBytes) }}
        </span>
      </div>

      <MButton
        variant="danger"
        icon="i-lucide-trash"
        :loading="cacheCleaning"
        data-testid="clean-cache-btn"
        @click="cleanCache"
      >
        {{ t('tools.cache.cleanNow') }}
      </MButton>
    </div>
  </MCard>
</template>

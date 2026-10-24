<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import type { InstanceFileItem } from '../../../ipc/client'
import { ipcClient } from '../../../ipc/client'
import {
  MCard,
  MButton,
  MIconButton,
  MSwitch,
  MEmpty,
  useToast,
  useDialog,
} from '../../../ui'

const props = defineProps<{
  instanceId: string
  folderKind: string
  title: string
}>()

const { t } = useI18n()
const toast = useToast()
const { confirm } = useDialog()

const files = ref<InstanceFileItem[]>([])
const loading = ref(false)

async function fetchFiles() {
  loading.value = true
  try {
    files.value = await ipcClient.instanceFilesList(props.instanceId, props.folderKind)
  } catch (e: unknown) {
    toast.danger(String(e))
  } finally {
    loading.value = false
  }
}

async function handleToggle(item: InstanceFileItem, nextVal: boolean) {
  try {
    const updated = await ipcClient.instanceFileToggle(
      props.instanceId,
      props.folderKind,
      item.fileName,
      nextVal,
    )
    const idx = files.value.findIndex((f) => f.path === item.path || f.fileName === item.fileName)
    if (idx !== -1) {
      files.value[idx] = updated
    }
    toast.success(t('instanceDetail.files.toggleSuccess'))
  } catch (e: unknown) {
    toast.danger(String(e))
  }
}

async function handleDelete(item: InstanceFileItem) {
  const ok = await confirm({
    title: t('instanceDetail.files.deleteConfirmTitle'),
    content: t('instanceDetail.files.deleteConfirm', { name: item.fileName }),
  })
  if (!ok) return

  try {
    await ipcClient.instanceFileDelete(props.instanceId, props.folderKind, item.fileName)
    files.value = files.value.filter((f) => f.fileName !== item.fileName)
    toast.success(t('instanceDetail.files.deleteSuccess'))
  } catch (e: unknown) {
    toast.danger(String(e))
  }
}

async function handleOpenFolder() {
  try {
    await ipcClient.instanceOpenFolder(props.instanceId)
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
  fetchFiles()
})
</script>

<template>
  <div class="max-w-4xl space-y-4">
    <!-- Top Bar -->
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-2">
        <h2 class="text-base font-bold text-fg">
          {{ title }}
        </h2>
        <span class="text-xs text-fg-muted font-mono">
          ({{ files.length }})
        </span>
      </div>
      <div class="flex items-center gap-2">
        <MButton variant="tonal" size="sm" @click="handleOpenFolder">
          <span class="w-4 h-4 i-lucide-folder-open mr-1" />
          {{ t('instanceDetail.files.openFolder') }}
        </MButton>
        <MButton variant="tonal" size="sm" :loading="loading" @click="fetchFiles">
          <span class="w-4 h-4 i-lucide-refresh-cw mr-1" />
          {{ t('instanceDetail.files.refresh') }}
        </MButton>
      </div>
    </div>

    <!-- Empty State -->
    <div v-if="files.length === 0" class="py-16">
      <MEmpty
        :title="t('instanceDetail.files.empty')"
        :description="t('instanceDetail.files.emptyHint')"
        icon="i-lucide-file-archive"
      />
    </div>

    <!-- Files List -->
    <div v-else class="space-y-2">
      <MCard
        v-for="file in files"
        :key="file.fileName"
        variant="well"
        hoverable
        padding="sm"
        class="flex items-center justify-between gap-4 transition-all"
        :class="{ 'opacity-60': !file.enabled }"
      >
        <div class="flex items-center gap-3.5 min-w-0">
          <div
            class="w-9 h-9 rounded-lg flex items-center justify-center shrink-0 border border-line"
            :class="[
              file.enabled
                ? 'bg-primary/10 text-primary border-primary/30'
                : 'bg-sunken text-fg-muted',
            ]"
          >
            <span class="w-5 h-5 i-lucide-file-box" />
          </div>

          <div class="min-w-0">
            <div class="text-sm font-semibold text-fg truncate">
              {{ file.fileName }}
            </div>
            <div class="flex items-center gap-3 text-xs text-fg-muted mt-0.5">
              <span>{{ formatBytes(file.size) }}</span>
              <span v-if="file.modifiedAt">{{ file.modifiedAt.split('T')[0] }}</span>
            </div>
          </div>
        </div>

        <div class="flex items-center gap-3 shrink-0">
          <MSwitch
            :model-value="file.enabled"
            @update:model-value="handleToggle(file, $event)"
          />
          <MIconButton
            icon="i-lucide-trash-2"
            size="sm"
            class="text-danger hover:bg-danger/10"
            :title="t('instanceDetail.files.delete')"
            @click="handleDelete(file)"
          />
        </div>
      </MCard>
    </div>
  </div>
</template>

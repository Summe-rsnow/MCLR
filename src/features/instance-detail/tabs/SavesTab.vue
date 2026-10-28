<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { convertFileSrc } from '@tauri-apps/api/core'
import type { SaveInfo, DatapackInfo } from '../../../ipc/client'
import { ipcClient } from '../../../ipc/client'
import {
  MCard,
  MButton,
  MIconButton,
  MBadge,
  MDialog,
  MInput,
  MEmpty,
  MSkeleton,
  MImage,
  useToast,
  useDialog,
} from '../../../ui'
import SaveDatapacksDrawer from '../components/SaveDatapacksDrawer.vue'
const props = defineProps<{
  instanceId: string
}>()

const { t } = useI18n()
const toast = useToast()
const { confirmDanger } = useDialog()

const saves = ref<SaveInfo[]>([])
const loading = ref(false)

// Datapack modal state
const activeSaveForDatapacks = ref<SaveInfo | null>(null)
const datapacksList = ref<DatapackInfo[]>([])
const loadingDatapacks = ref(false)

// Import dialog state
const showImportDialog = ref(false)
const importZipPath = ref('')
const importing = ref(false)

// Export dialog state
const showExportDialog = ref(false)
const exportSaveTarget = ref<SaveInfo | null>(null)
const exportDestPath = ref('')
const exporting = ref(false)

async function fetchSaves() {
  loading.value = true
  try {
    saves.value = await ipcClient.savesList(props.instanceId)
  } catch (e: unknown) {
    toast.danger(String(e))
  } finally {
    loading.value = false
  }
}

async function handleOpenFolder() {
  try {
    await ipcClient.instanceOpenPath(props.instanceId, 'saves')
  } catch (e: unknown) {
    toast.danger(String(e))
  }
}

async function handleBackup(save: SaveInfo) {
  try {
    const backupPath = await ipcClient.saveBackup(props.instanceId, save.folderName)
    toast.success(t('instanceDetail.saves.backupSuccess', { path: backupPath }))
  } catch (e: unknown) {
    toast.danger(String(e))
  }
}

async function handleDelete(save: SaveInfo) {
  const ok = await confirmDanger({
    title: t('instanceDetail.saves.deleteConfirmTitle'),
    content: t('instanceDetail.saves.deleteConfirm', { name: save.name || save.folderName }),
    requireCheckbox: t('instanceDetail.saves.deleteCheckbox'),
  })
  if (!ok) return

  try {
    await ipcClient.saveDelete(props.instanceId, save.folderName)
    saves.value = saves.value.filter((s) => s.folderName !== save.folderName)
    toast.success(t('instanceDetail.saves.deleteSuccess'))
  } catch (e: unknown) {
    toast.danger(String(e))
  }
}

function openExport(save: SaveInfo) {
  exportSaveTarget.value = save
  exportDestPath.value = `${save.folderName}.zip`
  showExportDialog.value = true
}

async function handleExport() {
  if (!exportSaveTarget.value || !exportDestPath.value.trim()) return
  exporting.value = true
  try {
    await ipcClient.saveExport(
      props.instanceId,
      exportSaveTarget.value.folderName,
      exportDestPath.value.trim(),
    )
    toast.success(t('instanceDetail.saves.exportSuccess'))
    showExportDialog.value = false
  } catch (e: unknown) {
    toast.danger(String(e))
  } finally {
    exporting.value = false
  }
}

function openImport() {
  importZipPath.value = ''
  showImportDialog.value = true
}

async function handleImport() {
  if (!importZipPath.value.trim()) return
  importing.value = true
  try {
    const imported = await ipcClient.saveImport(props.instanceId, importZipPath.value.trim())
    toast.success(t('instanceDetail.saves.importSuccess', { name: imported.name }))
    showImportDialog.value = false
    await fetchSaves()
  } catch (e: unknown) {
    toast.danger(String(e))
  } finally {
    importing.value = false
  }
}

async function openDatapacks(save: SaveInfo) {
  activeSaveForDatapacks.value = save
  loadingDatapacks.value = true
  try {
    datapacksList.value = await ipcClient.saveDatapacksList(props.instanceId, save.folderName)
  } catch (e: unknown) {
    toast.danger(String(e))
  } finally {
    loadingDatapacks.value = false
  }
}

async function handleDatapackToggle(pack: DatapackInfo, nextVal: boolean) {
  if (!activeSaveForDatapacks.value) return
  try {
    const updated = await ipcClient.saveDatapackToggle(
      props.instanceId,
      activeSaveForDatapacks.value.folderName,
      pack.id,
      nextVal,
    )
    datapacksList.value = updated
    toast.success(t('instanceDetail.files.toggleSuccess'))
  } catch (e: unknown) {
    toast.danger(String(e))
  }
}

function getGameModeText(mode: number): string {
  switch (mode) {
    case 0:
      return t('instanceDetail.saves.modes.survival')
    case 1:
      return t('instanceDetail.saves.modes.creative')
    case 2:
      return t('instanceDetail.saves.modes.adventure')
    case 3:
      return t('instanceDetail.saves.modes.spectator')
    default:
      return `${mode}`
  }
}

function getDifficultyText(diff: number): string {
  switch (diff) {
    case 0:
      return t('instanceDetail.saves.difficulties.peaceful')
    case 1:
      return t('instanceDetail.saves.difficulties.easy')
    case 2:
      return t('instanceDetail.saves.difficulties.normal')
    case 3:
      return t('instanceDetail.saves.difficulties.hard')
    default:
      return `${diff}`
  }
}

function formatPlayTime(ms: number): string {
  const totalSeconds = Math.floor(ms / 1000)
  const hours = Math.floor(totalSeconds / 3600)
  const minutes = Math.floor((totalSeconds % 3600) / 60)
  if (hours > 0) return `${hours}h ${minutes}m`
  return `${minutes}m`
}

onMounted(() => {
  fetchSaves()
})
</script>

<template>
  <div class="w-full space-y-4">
    <!-- Top Bar -->
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-2">
        <h2 class="text-base font-bold text-fg">
          {{ t('instanceDetail.tabs.saves') }}
        </h2>
        <span class="text-xs text-fg-muted font-mono"> ({{ saves.length }}) </span>
      </div>
      <div class="flex items-center gap-2">
        <MButton variant="primary" size="sm" @click="openImport">
          <span class="w-4 h-4 i-lucide-upload mr-1" />
          {{ t('instanceDetail.saves.importZip') }}
        </MButton>
        <MButton variant="tonal" size="sm" @click="handleOpenFolder">
          <span class="w-4 h-4 i-lucide-folder-open mr-1" />
          {{ t('instanceDetail.saves.openFolder') }}
        </MButton>
        <MButton variant="tonal" size="sm" :loading="loading" @click="fetchSaves">
          <span class="w-4 h-4 i-lucide-refresh-cw mr-1" />
          {{ t('instanceDetail.saves.refresh') }}
        </MButton>
      </div>
    </div>

    <!-- Skeleton Loading -->
    <div v-if="loading && saves.length === 0" class="space-y-3 py-1">
      <MCard
        v-for="n in 3"
        :key="n"
        variant="well"
        padding="md"
        class="flex flex-col md:flex-row items-start md:items-center justify-between gap-4"
      >
        <div class="flex items-start gap-4 min-w-0">
          <MSkeleton width="3.5rem" height="3.5rem" class="rounded-xl shrink-0" />
          <div class="space-y-2 min-w-0">
            <div class="flex items-center gap-2">
              <MSkeleton width="9rem" height="1.125rem" />
              <MSkeleton width="3rem" height="1.25rem" class="rounded-md" />
            </div>
            <div class="flex items-center gap-3">
              <MSkeleton width="5rem" height="0.75rem" />
              <MSkeleton width="5rem" height="0.75rem" />
              <MSkeleton width="6rem" height="0.75rem" />
            </div>
          </div>
        </div>
        <div class="flex items-center gap-1.5 self-end md:self-center shrink-0">
          <MSkeleton width="2rem" height="2rem" class="rounded-lg" />
          <MSkeleton width="2rem" height="2rem" class="rounded-lg" />
          <MSkeleton width="2rem" height="2rem" class="rounded-lg" />
        </div>
      </MCard>
    </div>

    <!-- Empty State -->
    <div v-else-if="saves.length === 0" class="py-16">
      <MEmpty :title="t('instanceDetail.saves.empty')" icon="i-lucide-globe" />
    </div>
    <!-- Saves List -->
    <div v-else class="space-y-3">
      <MCard
        v-for="save in saves"
        :key="save.folderName"
        variant="well"
        hoverable
        padding="md"
        class="flex flex-col md:flex-row items-start md:items-center justify-between gap-4 transition-all"
      >
        <div class="flex items-start gap-4 min-w-0">
          <!-- Icon / Thumbnail -->
          <div
            class="w-14 h-14 rounded-xl bg-sunken/60 flex items-center justify-center shrink-0 border border-line overflow-hidden text-primary"
          >
            <MImage
              v-if="save.iconPath"
              :src="convertFileSrc(save.iconPath)"
              alt="World Icon"
              fallback-icon="i-lucide-compass"
              class="w-full h-full"
            />
            <span v-else class="w-7 h-7 i-lucide-compass" />
          </div>

          <!-- Metadata -->
          <div class="min-w-0 space-y-1">
            <div class="flex items-center gap-2.5">
              <span class="text-base font-bold text-fg truncate">
                {{ save.name || save.folderName }}
              </span>
              <MBadge v-if="save.dataVersion" variant="primary" size="sm">
                v{{ save.dataVersion }}
              </MBadge>
            </div>

            <div class="flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-fg-muted">
              <span
                >{{ t('instanceDetail.saves.gameMode') }}:
                {{ getGameModeText(save.gameMode) }}</span
              >
              <span
                >{{ t('instanceDetail.saves.difficulty') }}:
                {{ getDifficultyText(save.difficulty) }}</span
              >
              <span v-if="save.playTime > 0"
                >{{ t('instanceDetail.saves.playTime') }}: {{ formatPlayTime(save.playTime) }}</span
              >
              <span
                >{{ t('instanceDetail.saves.seed') }}:
                <span class="font-mono">{{ save.seed }}</span></span
              >
              <span
                >{{ t('instanceDetail.saves.spawn') }}:
                <span class="font-mono"
                  >({{ save.spawn.x }}, {{ save.spawn.y }}, {{ save.spawn.z }})</span
                ></span
              >
            </div>
          </div>
        </div>

        <!-- Action Buttons -->
        <div class="flex items-center gap-1.5 self-end md:self-center shrink-0">
          <MButton variant="tonal" size="sm" @click="openDatapacks(save)">
            <span class="w-4 h-4 i-lucide-layers mr-1" />
            {{ t('instanceDetail.saves.actions.datapacks') }}
          </MButton>

          <MIconButton
            icon="i-lucide-archive"
            size="sm"
            :title="t('instanceDetail.saves.actions.backup')"
            @click="handleBackup(save)"
          />

          <MIconButton
            icon="i-lucide-download"
            size="sm"
            :title="t('instanceDetail.saves.actions.export')"
            @click="openExport(save)"
          />

          <MIconButton
            icon="i-lucide-trash"
            size="sm"
            class="text-danger hover:bg-danger/10"
            :title="t('instanceDetail.saves.actions.delete')"
            @click="handleDelete(save)"
          />
        </div>
      </MCard>
    </div>

    <!-- Datapacks Drawer -->
    <SaveDatapacksDrawer
      :open="!!activeSaveForDatapacks"
      :save="activeSaveForDatapacks"
      :datapacks="datapacksList"
      :loading="loadingDatapacks"
      @update:open="!$event && (activeSaveForDatapacks = null)"
      @toggle="handleDatapackToggle"
    />

    <!-- Import Dialog -->
    <MDialog
      :open="showImportDialog"
      :title="t('instanceDetail.saves.importDialogTitle')"
      @update:open="showImportDialog = $event"
    >
      <div class="py-2 space-y-3">
        <MInput
          v-model="importZipPath"
          :label="t('instanceDetail.saves.importPath')"
          placeholder="C:\Downloads\world.zip"
          autofocus
          @keydown.enter="handleImport"
        />
      </div>
      <template #footer>
        <div class="flex justify-end gap-2">
          <MButton variant="text" @click="showImportDialog = false">
            {{ t('common.cancel') }}
          </MButton>
          <MButton variant="primary" :loading="importing" @click="handleImport">
            {{ t('common.confirm') }}
          </MButton>
        </div>
      </template>
    </MDialog>

    <!-- Export Dialog -->
    <MDialog
      :open="showExportDialog"
      :title="t('instanceDetail.saves.exportDialogTitle')"
      @update:open="showExportDialog = $event"
    >
      <div class="py-2 space-y-3">
        <MInput
          v-model="exportDestPath"
          :label="t('instanceDetail.saves.importPath')"
          placeholder="C:\Exports\world.zip"
          autofocus
          @keydown.enter="handleExport"
        />
      </div>
      <template #footer>
        <div class="flex justify-end gap-2">
          <MButton variant="text" @click="showExportDialog = false">
            {{ t('common.cancel') }}
          </MButton>
          <MButton variant="primary" :loading="exporting" @click="handleExport">
            {{ t('common.confirm') }}
          </MButton>
        </div>
      </template>
    </MDialog>
  </div>
</template>

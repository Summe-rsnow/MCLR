<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import type { SaveInfo, DatapackInfo } from '../../../ipc/client'
import { ipcClient } from '../../../ipc/client'
import {
  MCard,
  MButton,
  MIconButton,
  MBadge,
  MSwitch,
  MDialog,
  MInput,
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
    await ipcClient.instanceOpenFolder(props.instanceId)
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
  const ok = await confirm({
    title: t('instanceDetail.saves.deleteConfirmTitle'),
    content: t('instanceDetail.saves.deleteConfirm', { name: save.name || save.folderName }),
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
  <div class="max-w-4xl space-y-4">
    <!-- Top Bar -->
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-2">
        <h2 class="text-base font-bold text-text">
          {{ t('instanceDetail.tabs.saves') }}
        </h2>
        <span class="text-xs text-textMuted font-mono">
          ({{ saves.length }})
        </span>
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

    <!-- Empty State -->
    <div v-if="saves.length === 0" class="py-16">
      <MEmpty
        :title="t('instanceDetail.saves.empty')"
        icon="i-lucide-globe"
      />
    </div>

    <!-- Saves List -->
    <div v-else class="space-y-3">
      <MCard
        v-for="save in saves"
        :key="save.folderName"
        class="p-5 flex flex-col md:flex-row items-start md:items-center justify-between gap-4 transition-all"
      >
        <div class="flex items-start gap-4 min-w-0">
          <!-- Icon / Thumbnail -->
          <div class="w-14 h-14 rounded-xl bg-surfaceVariant/60 flex items-center justify-center shrink-0 border border-outline/30 overflow-hidden text-primary">
            <img
              v-if="save.iconDataUrl"
              :src="save.iconDataUrl"
              class="w-full h-full object-cover"
              alt="World Icon"
            />
            <span v-else class="w-7 h-7 i-lucide-compass" />
          </div>

          <!-- Metadata -->
          <div class="min-w-0 space-y-1">
            <div class="flex items-center gap-2.5">
              <span class="text-base font-bold text-text truncate">
                {{ save.name || save.folderName }}
              </span>
              <MBadge v-if="save.dataVersion" variant="primary" size="sm">
                v{{ save.dataVersion }}
              </MBadge>
            </div>

            <div class="flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-textMuted">
              <span>{{ t('instanceDetail.saves.gameMode') }}: {{ getGameModeText(save.gameMode) }}</span>
              <span>{{ t('instanceDetail.saves.difficulty') }}: {{ getDifficultyText(save.difficulty) }}</span>
              <span v-if="save.playTime > 0">{{ t('instanceDetail.saves.playTime') }}: {{ formatPlayTime(save.playTime) }}</span>
              <span>{{ t('instanceDetail.saves.seed') }}: <span class="font-mono">{{ save.seed }}</span></span>
              <span>{{ t('instanceDetail.saves.spawn') }}: <span class="font-mono">({{ save.spawn.x }}, {{ save.spawn.y }}, {{ save.spawn.z }})</span></span>
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
            icon="i-lucide-trash-2"
            size="sm"
            class="text-danger hover:bg-danger/10"
            :title="t('instanceDetail.saves.actions.delete')"
            @click="handleDelete(save)"
          />
        </div>
      </MCard>
    </div>

    <!-- Datapacks Modal -->
    <MDialog
      :open="!!activeSaveForDatapacks"
      :title="t('instanceDetail.saves.datapackModalTitle', { name: activeSaveForDatapacks?.name || '' })"
      @update:open="!$event && (activeSaveForDatapacks = null)"
    >
      <div class="py-2 space-y-3 min-w-80 max-h-96 overflow-y-auto">
        <div v-if="datapacksList.length === 0" class="py-8">
          <MEmpty :title="t('instanceDetail.saves.datapackEmpty')" icon="i-lucide-layers" />
        </div>
        <div v-else class="space-y-2">
          <div
            v-for="pack in datapacksList"
            :key="pack.id"
            class="p-3 rounded-xl border border-outline/20 bg-surface flex items-center justify-between gap-3"
          >
            <div class="min-w-0">
              <div class="text-sm font-semibold text-text flex items-center gap-2 truncate">
                <span>{{ pack.fileName }}</span>
                <MBadge v-if="pack.builtIn" variant="neutral" size="sm">
                  {{ t('instanceDetail.saves.builtIn') }}
                </MBadge>
              </div>
              <div v-if="pack.description" class="text-xs text-textMuted mt-0.5 truncate">
                {{ pack.description }}
              </div>
              <div class="text-[11px] text-textMuted/60 font-mono mt-0.5">
                {{ t('instanceDetail.saves.datapackFormat') }}: {{ pack.packFormat }}
              </div>
            </div>

            <div class="shrink-0">
              <MSwitch
                :model-value="pack.enabled"
                :disabled="pack.builtIn"
                @update:model-value="handleDatapackToggle(pack, $event)"
              />
            </div>
          </div>
        </div>
      </div>
      <template #footer>
        <div class="flex justify-end">
          <MButton variant="tonal" @click="activeSaveForDatapacks = null">
            {{ t('common.close') }}
          </MButton>
        </div>
      </template>
    </MDialog>

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
          <MButton
            variant="primary"
            :loading="importing"
            @click="handleImport"
          >
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
          <MButton
            variant="primary"
            :loading="exporting"
            @click="handleExport"
          >
            {{ t('common.confirm') }}
          </MButton>
        </div>
      </template>
    </MDialog>
  </div>
</template>

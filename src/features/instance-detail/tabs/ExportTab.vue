<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { save as saveFileDialog } from '@tauri-apps/plugin-dialog'
import type {
  InstanceInfo,
  ExportCandidateEntry,
  ModpackExportFormat,
  ModpackExportOptions,
} from '../../../ipc/client'
import { ipcClient } from '../../../ipc/client'
import {
  MCard,
  MButton,
  MInput,
  MTextarea,
  MSegmented,
  MCheckbox,
  MBadge,
  MEmpty,
  useToast,
} from '../../../ui'

const props = defineProps<{
  instanceId: string
  instance?: InstanceInfo
}>()

const { t } = useI18n()
const toast = useToast()

const format = ref<ModpackExportFormat>('modrinth')
const packName = ref(props.instance?.name || props.instanceId)
const packVersion = ref('1.0.0')
const author = ref('')
const summary = ref('')
const outputPath = ref('')
const exporting = ref(false)

const loadingCandidates = ref(false)
const candidates = ref<ExportCandidateEntry[]>([])
const selectedPaths = ref<string[]>([])

const formatOptions = computed(() => [
  { label: t('instanceDetail.export.formats.modrinth'), value: 'modrinth' },
  { label: t('instanceDetail.export.formats.curseforge'), value: 'curseforge' },
])

watch(
  () => props.instance,
  (inst) => {
    if (inst?.name && !packName.value) {
      packName.value = inst.name
    }
    updateDefaultOutputPath()
  },
  { immediate: true },
)

watch(format, () => {
  updateDefaultOutputPath()
})

function updateDefaultOutputPath() {
  const safeName = (packName.value || props.instanceId || 'modpack').trim().replace(/[^a-zA-Z0-9_\u4e00-\u9fa5.-]/g, '_')
  const ext = format.value === 'modrinth' ? 'mrpack' : 'zip'
  if (!outputPath.value || outputPath.value.endsWith('.mrpack') || outputPath.value.endsWith('.zip')) {
    outputPath.value = `${safeName}-${packVersion.value || '1.0.0'}.${ext}`
  }
}

async function loadCandidates() {
  loadingCandidates.value = true
  try {
    const list = await ipcClient.exportScanInstance(props.instanceId)
    candidates.value = list
    // Pre-populate with defaultSelected
    selectedPaths.value = list.filter((c) => c.defaultSelected).map((c) => c.relPath)
  } catch (e: unknown) {
    toast.danger(String(e))
  } finally {
    loadingCandidates.value = false
  }
}

onMounted(() => {
  loadCandidates()
  updateDefaultOutputPath()
})

function togglePath(relPath: string, checked: boolean) {
  if (checked) {
    if (!selectedPaths.value.includes(relPath)) {
      selectedPaths.value.push(relPath)
    }
  } else {
    selectedPaths.value = selectedPaths.value.filter((p) => p !== relPath)
  }
}

function selectAll() {
  selectedPaths.value = candidates.value.map((c) => c.relPath)
}

function deselectAll() {
  selectedPaths.value = []
}

function restoreDefault() {
  selectedPaths.value = candidates.value.filter((c) => c.defaultSelected).map((c) => c.relPath)
}

async function browseOutputFile() {
  try {
    const ext = format.value === 'modrinth' ? 'mrpack' : 'zip'
    const defaultName = outputPath.value || `${packName.value || 'modpack'}.${ext}`
    const selected = await saveFileDialog({
      defaultPath: defaultName,
      filters: [
        {
          name: t('instanceDetail.export.fileFilter'),
          extensions: [ext],
        },
      ],
    })
    if (selected && typeof selected === 'string') {
      outputPath.value = selected
    }
  } catch {
    // Non-tauri browser mode fallback
  }
}

function formatBytes(bytes: number): string {
  if (bytes <= 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`
}

async function handleStartExport() {
  if (!packName.value.trim()) {
    toast.warning(t('instanceDetail.export.validation.nameRequired'))
    return
  }
  if (!packVersion.value.trim()) {
    toast.warning(t('instanceDetail.export.validation.versionRequired'))
    return
  }
  if (!outputPath.value.trim()) {
    toast.warning(t('instanceDetail.export.validation.outputPathRequired'))
    return
  }
  if (selectedPaths.value.length === 0) {
    toast.warning(t('instanceDetail.export.validation.filesRequired'))
    return
  }

  exporting.value = true
  try {
    const options: ModpackExportOptions = {
      instanceId: props.instanceId,
      format: format.value,
      packName: packName.value.trim(),
      packVersion: packVersion.value.trim(),
      author: author.value.trim(),
      summary: summary.value.trim(),
      selectedPaths: [...selectedPaths.value],
      outputPath: outputPath.value.trim(),
    }
    await ipcClient.exportModpack(options)
    toast.success(t('instanceDetail.export.exportSuccess'))
  } catch (e: unknown) {
    toast.danger(String(e))
  } finally {
    exporting.value = false
  }
}
</script>

<template>
  <div class="space-y-6 max-w-4xl pb-10">
    <!-- Header Card -->
    <MCard class="p-6">
      <div class="flex items-center gap-3 mb-2">
        <span class="w-6 h-6 i-lucide-package-plus text-primary shrink-0" />
        <h2 class="text-xl font-bold text-text">
          {{ t('instanceDetail.export.title') }}
        </h2>
      </div>
      <p class="text-sm text-text-muted">
        {{ t('instanceDetail.export.description') }}
      </p>
    </MCard>

    <!-- Configuration Options -->
    <MCard class="p-6 space-y-5">
      <!-- Format Selector -->
      <div>
        <label class="block text-sm font-medium text-text mb-2">
          {{ t('instanceDetail.export.formatLabel') }}
        </label>
        <MSegmented
          v-model="format"
          :options="formatOptions"
          class="w-full sm:w-auto"
        />
      </div>

      <!-- Basic metadata -->
      <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
        <div>
          <label class="block text-sm font-medium text-text mb-1.5">
            {{ t('instanceDetail.export.packName') }}
          </label>
          <MInput
            v-model="packName"
            :placeholder="t('instanceDetail.export.packNamePlaceholder')"
            class="w-full"
          />
        </div>
        <div>
          <label class="block text-sm font-medium text-text mb-1.5">
            {{ t('instanceDetail.export.packVersion') }}
          </label>
          <MInput
            v-model="packVersion"
            :placeholder="t('instanceDetail.export.packVersionPlaceholder')"
            class="w-full"
          />
        </div>
      </div>

      <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
        <div>
          <label class="block text-sm font-medium text-text mb-1.5">
            {{ t('instanceDetail.export.author') }}
          </label>
          <MInput
            v-model="author"
            :placeholder="t('instanceDetail.export.authorPlaceholder')"
            class="w-full"
          />
        </div>
        <div>
          <label class="block text-sm font-medium text-text mb-1.5">
            {{ t('instanceDetail.export.outputPath') }}
          </label>
          <div class="flex gap-2">
            <MInput
              v-model="outputPath"
              :placeholder="t('instanceDetail.export.outputPathPlaceholder')"
              class="flex-1"
            />
            <MButton
              variant="tonal"
              icon="i-lucide-folder-open"
              @click="browseOutputFile"
            >
              {{ t('instanceDetail.export.browse') }}
            </MButton>
          </div>
        </div>
      </div>

      <div>
        <label class="block text-sm font-medium text-text mb-1.5">
          {{ t('instanceDetail.export.summary') }}
        </label>
        <MTextarea
          v-model="summary"
          :placeholder="t('instanceDetail.export.summaryPlaceholder')"
          :rows="2"
          class="w-full"
        />
      </div>
    </MCard>

    <!-- File selection checklist -->
    <MCard class="p-6">
      <div class="flex flex-wrap items-center justify-between gap-3 mb-4">
        <div>
          <h3 class="text-base font-semibold text-text">
            {{ t('instanceDetail.export.filesSelection') }}
          </h3>
          <p class="text-xs text-text-muted mt-0.5">
            {{ t('instanceDetail.export.filesSelectionSubtitle') }}
          </p>
        </div>
        <div class="flex items-center gap-2">
          <MButton
            variant="text"
            size="sm"
            @click="selectAll"
          >
            {{ t('instanceDetail.export.selectAll') }}
          </MButton>
          <MButton
            variant="text"
            size="sm"
            @click="deselectAll"
          >
            {{ t('instanceDetail.export.deselectAll') }}
          </MButton>
          <MButton
            variant="tonal"
            size="sm"
            @click="restoreDefault"
          >
            {{ t('instanceDetail.export.defaultSelection') }}
          </MButton>
        </div>
      </div>

      <div v-if="loadingCandidates" class="py-8 flex justify-center text-text-muted">
        <span class="w-6 h-6 i-lucide-loader-2 animate-spin" />
      </div>
      <div v-else-if="candidates.length === 0" class="py-8">
        <MEmpty
          :title="t('instanceDetail.export.emptyFiles')"
          icon="i-lucide-folder"
        />
      </div>
      <div
        v-else
        class="border border-outline/20 rounded-xl divide-y divide-outline/10 max-h-96 overflow-y-auto bg-surfaceVariant/20"
      >
        <div
          v-for="entry in candidates"
          :key="entry.relPath"
          class="flex items-center justify-between px-4 py-2.5 hover:bg-surfaceVariant/40 transition-colors cursor-pointer"
          @click="togglePath(entry.relPath, !selectedPaths.includes(entry.relPath))"
        >
          <div class="flex items-center gap-3 min-w-0" @click.stop>
            <MCheckbox
              :model-value="selectedPaths.includes(entry.relPath)"
              @update:model-value="(val) => togglePath(entry.relPath, Boolean(val))"
            />
            <div class="flex items-center gap-2 truncate">
              <span
                class="w-4 h-4 shrink-0 text-text-muted"
                :class="entry.isDir ? 'i-lucide-folder text-primary/80' : 'i-lucide-file-text'"
              />
              <span class="text-sm font-medium text-text truncate">
                {{ entry.relPath }}
              </span>
            </div>
          </div>
          <div class="flex items-center gap-2 shrink-0 ml-3">
            <span class="text-xs text-text-muted">
              {{ formatBytes(entry.size) }}
            </span>
            <MBadge variant="neutral" size="sm">
              {{ entry.isDir ? t('instanceDetail.export.isDir') : t('instanceDetail.export.isFile') }}
            </MBadge>
          </div>
        </div>
      </div>

      <!-- Action Button -->
      <div class="mt-6 flex justify-end">
        <MButton
          variant="primary"
          icon="i-lucide-download"
          :loading="exporting"
          @click="handleStartExport"
        >
          {{ exporting ? t('instanceDetail.export.exporting') : t('instanceDetail.export.startExport') }}
        </MButton>
      </div>
    </MCard>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type {
  InstanceInfo,
  ExportCandidateEntry,
  ModpackExportFormat,
  ModpackExportOptions,
} from '../../../ipc/client'
import { ipcClient } from '../../../ipc/client'
import {
  MSection,
  MSettingRow,
  MCard,
  MButton,
  MInput,
  MPathPicker,
  MTextarea,
  MSegmented,
  MCheckbox,
  MBadge,
  MEmpty,
  MSkeleton,
  MVirtualList,
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

const exportFilters = computed(() => [
  {
    name:
      format.value === 'modrinth' ? 'Modrinth Modpack (*.mrpack)' : 'CurseForge Modpack (*.zip)',
    extensions: format.value === 'modrinth' ? ['mrpack'] : ['zip'],
  },
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
  const ext = format.value === 'modrinth' ? 'mrpack' : 'zip'
  const safeName = (packName.value || 'modpack').replace(/[^a-zA-Z0-9_\u4e00-\u9fa5-]/g, '_')
  outputPath.value = `${safeName}-${packVersion.value || '1.0.0'}.${ext}`
}

async function fetchCandidates() {
  loadingCandidates.value = true
  try {
    const list = await ipcClient.exportScanInstance(props.instanceId)
    candidates.value = list
    // Pre-select defaults
    selectedPaths.value = list.filter((c) => c.defaultSelected).map((c) => c.relPath)
  } catch (e: unknown) {
    toast.danger(String(e))
  } finally {
    loadingCandidates.value = false
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

function togglePath(relPath: string, checked: boolean) {
  if (checked) {
    if (!selectedPaths.value.includes(relPath)) {
      selectedPaths.value.push(relPath)
    }
  } else {
    selectedPaths.value = selectedPaths.value.filter((p) => p !== relPath)
  }
}

function formatBytes(bytes: number): string {
  if (bytes <= 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(1024))
  return `${(bytes / Math.pow(1024, i)).toFixed(1)} ${units[i]}`
}

async function handleStartExport() {
  if (!outputPath.value.trim()) {
    toast.danger(t('instanceDetail.export.pathRequired'))
    return
  }

  exporting.value = true
  try {
    const options: ModpackExportOptions = {
      instanceId: props.instanceId,
      format: format.value,
      packName: packName.value,
      packVersion: packVersion.value,
      author: author.value,
      summary: summary.value,
      selectedPaths: selectedPaths.value,
      outputPath: outputPath.value,
    }
    await ipcClient.exportModpack(options)
    toast.success(t('instanceDetail.export.success', { path: outputPath.value }))
  } catch (e: unknown) {
    toast.danger(String(e))
  } finally {
    exporting.value = false
  }
}

onMounted(() => {
  fetchCandidates()
})
</script>

<template>
  <div class="space-y-6">
    <!-- Header Summary Card -->
    <MCard padding="md">
      <div class="flex items-center gap-3 mb-1">
        <span class="w-5 h-5 i-lucide-package-plus text-primary shrink-0" />
        <h2 class="text-lg font-bold text-fg">
          {{ t('instanceDetail.export.title') }}
        </h2>
      </div>
      <p class="text-xs text-fg-muted">
        {{ t('instanceDetail.export.description') }}
      </p>
    </MCard>

    <!-- Configuration Options Section -->
    <MSection :title="t('instanceDetail.export.title')" icon="i-lucide-settings-2">
      <div class="divide-y divide-line">
        <MSettingRow :label="t('instanceDetail.export.formatLabel')">
          <MSegmented v-model="format" :options="formatOptions" size="sm" />
        </MSettingRow>

        <MSettingRow :label="t('instanceDetail.export.packName')">
          <MInput
            v-model="packName"
            :placeholder="t('instanceDetail.export.packNamePlaceholder')"
            size="sm"
            class="w-72"
          />
        </MSettingRow>

        <MSettingRow :label="t('instanceDetail.export.packVersion')">
          <MInput
            v-model="packVersion"
            :placeholder="t('instanceDetail.export.packVersionPlaceholder')"
            size="sm"
            class="w-72"
          />
        </MSettingRow>

        <MSettingRow :label="t('instanceDetail.export.author')">
          <MInput
            v-model="author"
            :placeholder="t('instanceDetail.export.authorPlaceholder')"
            size="sm"
            class="w-72"
          />
        </MSettingRow>

        <MSettingRow :label="t('instanceDetail.export.outputPath')">
          <div class="w-80">
            <MPathPicker
              v-model="outputPath"
              type="saveFile"
              size="sm"
              :placeholder="t('instanceDetail.export.outputPathPlaceholder')"
              :button-text="t('instanceDetail.export.browse')"
              :filters="exportFilters"
              :default-path="outputPath"
            />
          </div>
        </MSettingRow>

        <MSettingRow :label="t('instanceDetail.export.summary')">
          <MTextarea
            v-model="summary"
            :placeholder="t('instanceDetail.export.summaryPlaceholder')"
            :rows="2"
            class="w-72"
          />
        </MSettingRow>
      </div>
    </MSection>

    <!-- File selection checklist Section -->
    <MSection
      :title="t('instanceDetail.export.filesSelection')"
      :description="t('instanceDetail.export.filesSelectionSubtitle')"
      icon="i-lucide-folder-tree"
    >
      <template #actions>
        <div class="flex items-center gap-2">
          <MButton variant="text" size="xs" @click="selectAll">
            {{ t('instanceDetail.export.selectAll') }}
          </MButton>
          <MButton variant="text" size="xs" @click="deselectAll">
            {{ t('instanceDetail.export.deselectAll') }}
          </MButton>
          <MButton variant="tonal" size="xs" @click="restoreDefault">
            {{ t('instanceDetail.export.defaultSelection') }}
          </MButton>
        </div>
      </template>

      <div
        v-if="loadingCandidates && candidates.length === 0"
        class="border border-line rounded-xl divide-y divide-line bg-sunken/40 overflow-hidden"
      >
        <div v-for="n in 5" :key="n" class="flex items-center justify-between px-4 py-2.5">
          <div class="flex items-center gap-3 min-w-0">
            <MSkeleton width="1.25rem" height="1.25rem" class="rounded shrink-0" />
            <MSkeleton width="1rem" height="1rem" class="shrink-0" />
            <MSkeleton :width="n % 2 === 0 ? '14rem' : '10rem'" height="1rem" />
          </div>
          <div class="flex items-center gap-2 shrink-0 ml-3">
            <MSkeleton width="3.5rem" height="0.875rem" />
            <MSkeleton width="2.5rem" height="1.25rem" class="rounded-md" />
          </div>
        </div>
      </div>
      <div v-else-if="candidates.length === 0" class="py-8">
        <MEmpty :title="t('instanceDetail.export.emptyFiles')" icon="i-lucide-folder" />
      </div>
      <div
        v-else
        class="border border-line rounded-xl divide-y divide-line max-h-96 overflow-y-auto overflow-x-hidden bg-sunken/40"
      >
        <MVirtualList
          :items="candidates"
          :item-key="(entry) => entry.relPath"
          :estimate-size="41"
          :threshold="60"
        >
          <template #item="{ item: entry }">
            <div
              class="flex items-center justify-between px-4 py-2 hover:bg-sunken/80 transition-colors cursor-pointer border-b border-line last:border-b-0"
              @click="togglePath(entry.relPath, !selectedPaths.includes(entry.relPath))"
            >
              <div class="flex items-center gap-3 min-w-0" @click.stop>
                <MCheckbox
                  :model-value="selectedPaths.includes(entry.relPath)"
                  @update:model-value="(val) => togglePath(entry.relPath, Boolean(val))"
                />
                <div class="flex items-center gap-2 truncate">
                  <span
                    class="w-4 h-4 shrink-0 text-fg-muted"
                    :class="entry.isDir ? 'i-lucide-folder text-primary/80' : 'i-lucide-file-text'"
                  />
                  <span class="text-sm font-medium text-fg truncate">
                    {{ entry.relPath }}
                  </span>
                </div>
              </div>
              <div class="flex items-center gap-2 shrink-0 ml-3">
                <span class="text-xs text-fg-muted font-mono">
                  {{ formatBytes(entry.size) }}
                </span>
                <MBadge variant="neutral" size="sm">
                  {{
                    entry.isDir
                      ? t('instanceDetail.export.isDir')
                      : t('instanceDetail.export.isFile')
                  }}
                </MBadge>
              </div>
            </div>
          </template>
        </MVirtualList>
      </div>

      <!-- Action Button -->
      <div class="pt-4 flex justify-end">
        <MButton
          variant="primary"
          icon="i-lucide-download"
          size="sm"
          :loading="exporting"
          @click="handleStartExport"
        >
          {{
            exporting
              ? t('instanceDetail.export.exporting')
              : t('instanceDetail.export.startExport')
          }}
        </MButton>
      </div>
    </MSection>
  </div>
</template>

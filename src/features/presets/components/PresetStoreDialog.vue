<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type { StoreInfo } from '../../../ipc/client'
import { ipcClient } from '../../../ipc/client'
import {
  MBadge,
  MButton,
  MCheckbox,
  MDialog,
  MEmpty,
  MHelpTooltip,
  MIconButton,
  MPathPicker,
  MSettingRow,
  MSwitch,
  useDialog,
  useToast,
} from '../../../ui'
import { useErrorText } from '../../../composables/useErrorText'
import { awaitTask } from '../../tasks/awaitTask'
import { usePresetsStore } from '../store'

const props = defineProps<{
  open: boolean
  presetId?: string | null
  presetName?: string | null
}>()

const emit = defineEmits<{
  (e: 'update:open', val: boolean): void
}>()

const { t } = useI18n()
const toast = useToast()
const dialog = useDialog()
const { formatError } = useErrorText()
const store = usePresetsStore()

const info = ref<StoreInfo | null>(null)
const dir = ref('')
const migrate = ref(true)
const useHardlink = ref(false)
const busy = ref(false)
const loading = ref(false)

const entries = computed(() => info.value?.entries ?? [])

function formatBytes(bytes: number): string {
  if (bytes >= 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`
  if (bytes >= 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
  if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${bytes} B`
}

function kindLabel(kind: string): string {
  switch (kind) {
    case 'mod':
      return t('presets.kinds.mod')
    case 'resourcepack':
      return t('presets.kinds.resourcepack')
    case 'shader':
      return t('presets.kinds.shader')
    case 'datapack':
      return t('presets.kinds.datapack')
    case 'world':
      return t('presets.kinds.world')
    default:
      return kind
  }
}

async function load(): Promise<void> {
  loading.value = true
  try {
    info.value = await store.loadStoreInfo()
    dir.value = info.value.dir
    const settings = await ipcClient.settingsGet()
    useHardlink.value = settings.comp?.useHardlink ?? false
  } catch (error) {
    toast.danger(formatError(error))
  } finally {
    loading.value = false
  }
}

watch(
  () => props.open,
  (open) => {
    if (open) void load()
  },
  { immediate: true },
)

async function applyDir(target: string): Promise<void> {
  if (!target.trim()) return
  busy.value = true
  try {
    info.value = await ipcClient.presetsStoreSetDir(target, migrate.value)
    dir.value = info.value.dir
    toast.success(t('presets.storeDialog.dirUpdated'))
  } catch (error) {
    toast.danger(formatError(error))
  } finally {
    busy.value = false
  }
}

async function resetDir(): Promise<void> {
  if (info.value) await applyDir(info.value.defaultDir)
}

async function setHardlink(enabled: boolean): Promise<void> {
  const previous = useHardlink.value
  useHardlink.value = enabled
  try {
    await ipcClient.settingsPatch({ comp: { useHardlink: enabled } })
  } catch (error) {
    useHardlink.value = previous
    toast.danger(formatError(error))
  }
}

async function prune(): Promise<void> {
  const confirmed = await dialog.confirmDanger({
    title: t('presets.storeDialog.pruneConfirmTitle'),
    content: t('presets.storeDialog.pruneConfirmContent'),
    confirmText: t('presets.storeDialog.prune'),
    cancelText: t('common.cancel'),
  })
  if (!confirmed) return
  busy.value = true
  try {
    const result = await ipcClient.presetsStorePrune()
    toast.success(
      t('presets.storeDialog.pruneDone', {
        count: result.removedCount,
        size: formatBytes(result.freedBytes),
      }),
    )
    await load()
  } catch (error) {
    toast.danger(formatError(error))
  } finally {
    busy.value = false
  }
}

async function removeEntry(sha1: string): Promise<void> {
  const confirmed = await dialog.confirmDanger({
    title: t('presets.storeDialog.removeConfirmTitle'),
    content: t('presets.storeDialog.removeConfirmContent'),
    confirmText: t('common.delete'),
    cancelText: t('common.cancel'),
  })
  if (!confirmed) return
  try {
    await ipcClient.presetsStoreRemove(sha1)
    toast.success(t('presets.storeDialog.removed'))
    await load()
  } catch (error) {
    toast.danger(formatError(error))
  }
}

async function cachePreset(): Promise<void> {
  if (!props.presetId) return
  busy.value = true
  try {
    const taskId = await ipcClient.presetsCache(props.presetId)
    await awaitTask(taskId)
    toast.success(t('presets.storeDialog.cacheStarted', { name: props.presetName ?? '' }))
    await load()
  } catch (error) {
    toast.danger(formatError(error))
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <MDialog
    :open="open"
    :title="t('presets.storeDialog.title')"
    :hide-footer="true"
    max-width="lg"
    @update:open="emit('update:open', $event)"
  >
    <div class="flex flex-col gap-4 max-h-[65vh] overflow-y-auto pr-1" data-testid="preset-store">
      <div class="grid grid-cols-3 gap-2">
        <div class="m-well rounded-xl p-3 text-center">
          <div class="text-lg font-bold text-fg">{{ info?.objectCount ?? 0 }}</div>
          <div class="text-xs text-fg-muted">{{ t('presets.storeDialog.objectCount') }}</div>
        </div>
        <div class="m-well rounded-xl p-3 text-center">
          <div class="text-lg font-bold text-fg">{{ formatBytes(info?.totalBytes ?? 0) }}</div>
          <div class="text-xs text-fg-muted">{{ t('presets.storeDialog.totalBytes') }}</div>
        </div>
        <div class="m-well rounded-xl p-3 text-center">
          <div class="text-lg font-bold text-success-fg">
            {{ formatBytes(info?.savedBytes ?? 0) }}
          </div>
          <div class="text-xs text-fg-muted">{{ t('presets.storeDialog.savedBytes') }}</div>
        </div>
      </div>

      <div class="flex flex-col gap-2">
        <span class="text-sm font-medium text-fg">{{ t('presets.storeDialog.dirLabel') }}</span>
        <div class="flex items-center gap-2">
          <MPathPicker
            v-model="dir"
            type="directory"
            :placeholder="t('presets.storeDialog.dirPlaceholder')"
          />
          <MButton
            variant="primary"
            size="sm"
            :loading="busy"
            data-testid="apply-store-dir-btn"
            @click="applyDir(dir)"
          >
            {{ t('presets.storeDialog.applyDir') }}
          </MButton>
          <MButton variant="text" size="sm" :disabled="busy" @click="resetDir">
            {{ t('presets.storeDialog.resetDefault') }}
          </MButton>
        </div>
        <MCheckbox v-model="migrate" :label="t('presets.storeDialog.migrate')" />
        <p class="text-xs text-fg-subtle">{{ t('presets.storeDialog.defaultHint') }}</p>
      </div>

      <MSettingRow
        :label="t('presets.storeDialog.useHardlink')"
        :hint="t('presets.storeDialog.useHardlinkHint')"
      >
        <template #label>
          <div class="flex items-center gap-1.5 flex-wrap">
            <span>{{ t('presets.storeDialog.useHardlink') }}</span>
            <MBadge variant="warning" size="sm">
              {{ t('presets.storeDialog.useHardlinkBeta') }}
            </MBadge>
            <MHelpTooltip :text="t('presets.storeDialog.hardlinkTooltip')" size="xs" />
          </div>
        </template>
        <MSwitch :model-value="useHardlink" @update:model-value="setHardlink" />
      </MSettingRow>

      <div class="flex items-center justify-between gap-2 flex-wrap">
        <div class="flex items-center gap-2">
          <MButton
            variant="tonal"
            size="sm"
            icon="i-lucide-eraser"
            :loading="busy"
            data-testid="prune-store-btn"
            @click="prune"
          >
            {{ t('presets.storeDialog.prune') }}
          </MButton>
          <MButton
            variant="tonal"
            size="sm"
            icon="i-lucide-download"
            :disabled="!presetId"
            :loading="busy"
            data-testid="cache-preset-btn"
            @click="cachePreset"
          >
            {{ t('presets.storeDialog.cachePreset') }}
          </MButton>
        </div>
      </div>

      <div class="flex flex-col gap-2">
        <span class="text-sm font-medium text-fg">
          {{ t('presets.storeDialog.entries') }}
        </span>
        <MEmpty
          v-if="entries.length === 0"
          :title="t('presets.storeDialog.entriesEmpty')"
          icon="i-lucide-database"
        />
        <div v-else class="flex flex-col divide-y divide-line/60">
          <div
            v-for="entry in entries"
            :key="entry.sha1"
            class="grid grid-cols-[minmax(0,1fr)_auto_auto_auto_auto] items-center gap-2 py-2"
            data-testid="store-entry-row"
          >
            <div class="min-w-0">
              <div class="text-sm text-fg truncate">{{ entry.fileName }}</div>
              <div class="text-xs text-fg-subtle">
                {{ kindLabel(entry.kind) }} • {{ entry.platform }}
              </div>
            </div>
            <span class="text-xs text-fg-muted whitespace-nowrap">
              {{ formatBytes(entry.size) }}
            </span>
            <MBadge variant="neutral" size="sm">
              {{ t('presets.storeDialog.liveRefs', { count: entry.liveRefs }) }}
            </MBadge>
            <MBadge v-if="entry.pinned" variant="primary" size="sm" icon="i-lucide-pin">
              {{ t('presets.storeDialog.pinned') }}
            </MBadge>
            <MBadge v-else variant="neutral" size="sm" dot>
              {{ t('presets.storeDialog.unreferenced') }}
            </MBadge>
            <MIconButton
              icon="i-lucide-trash-2"
              variant="text"
              size="sm"
              :title="t('presets.storeDialog.deleteEntry')"
              data-testid="delete-store-entry-btn"
              @click="removeEntry(entry.sha1)"
            />
          </div>
        </div>
      </div>
    </div>
  </MDialog>
</template>

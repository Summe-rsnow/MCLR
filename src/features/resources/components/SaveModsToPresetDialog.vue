<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { convertFileSrc } from '@tauri-apps/api/core'
import type {
  AddItemOutcome,
  AddStatus,
  PresetItem,
  ResourceEntry,
  ResourcePreset,
} from '../../../ipc/client'
import { ipcClient } from '../../../ipc/client'
import { MBadge, MButton, MCheckbox, MDialog, MImage, MInput, MSelect, MSwitch } from '../../../ui'

const props = defineProps<{
  open: boolean
  /** Mods to offer: the current selection when the user made one, otherwise every installed mod. */
  entries: ResourceEntry[]
}>()

const emit = defineEmits<{
  (e: 'update:open', value: boolean): void
}>()

const { t } = useI18n()

/** Sentinel value for the "create a new preset" option. */
const NEW_PRESET = '__new__'

interface ModRow {
  entry: ResourceEntry
  checked: boolean
  /** `true` pins the installed file id; the default follows the latest compatible file. */
  lockVersion: boolean
}

const presets = ref<ResourcePreset[]>([])
const selected = ref<string>(NEW_PRESET)
const newName = ref('')
const rows = ref<ModRow[]>([])
const loading = ref(false)
const submitting = ref(false)
const outcomes = ref<AddItemOutcome[]>([])
const error = ref<string | null>(null)
const savedTo = ref<ResourcePreset | null>(null)

/** Only mods resolved to a community project can be re-downloaded from a preset. */
const recognized = computed(() => props.entries.filter((entry) => Boolean(entry.projectId)))
const unrecognized = computed(() => props.entries.filter((entry) => !entry.projectId))

const creating = computed(
  () => selected.value === NEW_PRESET || !presets.value.some((p) => p.id === selected.value),
)

const options = computed(() => [
  ...presets.value.map((preset) => ({ label: preset.name, value: preset.id })),
  { label: t('resources.saveToPreset.newPreset'), value: NEW_PRESET },
])

const checkedRows = computed(() => rows.value.filter((row) => row.checked))
const lockableCount = computed(() => rows.value.filter((row) => Boolean(row.entry.fileId)).length)

const canSubmit = computed(
  () =>
    checkedRows.value.length > 0 &&
    recognized.value.length > 0 &&
    (!creating.value || newName.value.trim().length > 0),
)

watch(
  () => props.open,
  async (open) => {
    if (!open) return
    outcomes.value = []
    error.value = null
    savedTo.value = null
    newName.value = ''
    // Unlocked by default: locking a version is an explicit choice per mod.
    rows.value = recognized.value.map((entry) => ({
      entry,
      checked: true,
      lockVersion: false,
    }))
    loading.value = true
    try {
      presets.value = await ipcClient.presetsList()
      if (!presets.value.some((preset) => preset.id === selected.value)) {
        selected.value = presets.value[0]?.id ?? NEW_PRESET
      }
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
      selected.value = NEW_PRESET
    } finally {
      loading.value = false
    }
  },
  { immediate: true },
)

function close() {
  emit('update:open', false)
}

function lockAll() {
  for (const row of rows.value) {
    // A mod without a known file id has nothing to pin; its switch stays disabled.
    if (row.entry.fileId) row.lockVersion = true
  }
}

function unlockAll() {
  for (const row of rows.value) row.lockVersion = false
}

function outcomeVariant(status: AddStatus): 'success' | 'warning' | 'danger' | 'neutral' {
  switch (status) {
    case 'added':
      return 'success'
    case 'alreadyInPreset':
      return 'neutral'
    case 'alreadyInPresetOtherVersion':
      return 'warning'
    default:
      return 'danger'
  }
}

function outcomeIcon(status: AddStatus): string {
  switch (status) {
    case 'added':
      return 'i-lucide-circle-check text-success'
    case 'alreadyInPreset':
    case 'alreadyInPresetOtherVersion':
      return 'i-lucide-info text-fg-muted'
    default:
      return 'i-lucide-circle-alert text-danger'
  }
}

function outcomeText(status: AddStatus): string {
  return t(`resources.saveToPreset.outcome.${status}`)
}

function subtitle(entry: ResourceEntry): string {
  return entry.version || entry.fileName
}

/** Jar artwork lives on disk (asset protocol); project artwork is a plain URL. */
function iconSrc(entry: ResourceEntry): string | null {
  const icon = entry.icon
  if (!icon) return null
  return /^https?:\/\//i.test(icon) ? icon : convertFileSrc(icon)
}

async function submit() {
  if (!canSubmit.value || submitting.value) return
  submitting.value = true
  error.value = null
  outcomes.value = []

  try {
    let presetId = selected.value
    let preset = presets.value.find((item) => item.id === presetId) ?? null
    if (creating.value) {
      preset = await ipcClient.presetsSave({ name: newName.value.trim() })
      presets.value = [...presets.value, preset]
      presetId = preset.id
      selected.value = preset.id
    }

    const items: PresetItem[] = checkedRows.value.map((row) => {
      const pinned = row.lockVersion && Boolean(row.entry.fileId)
      return {
        platform: row.entry.platform ?? 'modrinth',
        projectId: row.entry.projectId as string,
        kind: 'mod',
        title: row.entry.zhName || row.entry.title || row.entry.fileName,
        slug: row.entry.modId ?? null,
        iconUrl: row.entry.icon?.startsWith('http') ? row.entry.icon : null,
        pinnedFileId: pinned ? row.entry.fileId : null,
        pinnedFileName: pinned ? row.entry.fileName : null,
      }
    })
    outcomes.value = await ipcClient.presetsAddItems(presetId, items)
    if (preset) savedTo.value = preset
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
  } finally {
    submitting.value = false
  }
}
</script>

<template>
  <MDialog
    :open="open"
    :title="t('resources.saveToPreset.title')"
    max-width="lg"
    hide-footer
    @update:open="emit('update:open', $event)"
  >
    <div class="space-y-4 py-1">
      <div class="flex flex-col sm:flex-row gap-3">
        <div class="space-y-1.5 flex-1 min-w-0">
          <label class="text-xs font-semibold text-fg-muted block">
            {{ t('resources.saveToPreset.selectPreset') }}
          </label>
          <MSelect v-model="selected" :options="options" :loading="loading" size="sm" />
        </div>
        <div v-if="creating" class="space-y-1.5 flex-1 min-w-0">
          <label class="text-xs font-semibold text-fg-muted block">
            {{ t('resources.saveToPreset.newPresetLabel') }}
          </label>
          <MInput
            v-model="newName"
            :placeholder="t('resources.saveToPreset.newPresetPlaceholder')"
            size="sm"
            clearable
          />
        </div>
      </div>

      <div v-if="recognized.length > 0" class="space-y-2">
        <div class="flex flex-wrap items-center justify-between gap-2">
          <span class="text-xs font-semibold text-fg-muted">
            {{ t('resources.saveToPreset.modsTitle', { count: recognized.length }) }}
          </span>
          <div class="flex items-center gap-1.5">
            <MButton
              variant="text"
              size="sm"
              :disabled="lockableCount === 0"
              data-testid="save-to-preset-lock-all"
              @click="lockAll"
            >
              {{ t('resources.saveToPreset.lockAll') }}
            </MButton>
            <MButton
              variant="text"
              size="sm"
              data-testid="save-to-preset-unlock-all"
              @click="unlockAll"
            >
              {{ t('resources.saveToPreset.unlockAll') }}
            </MButton>
          </div>
        </div>

        <div class="space-y-1.5 max-h-72 overflow-y-auto pr-1" data-testid="save-to-preset-rows">
          <div
            v-for="row in rows"
            :key="row.entry.path"
            class="flex items-center gap-3 p-2 rounded-xl border border-line bg-panel"
          >
            <MCheckbox v-model="row.checked" />
            <MImage :src="iconSrc(row.entry)" size="sm" fallback-icon="i-lucide-puzzle" />
            <div class="min-w-0 flex-1">
              <div class="text-sm text-fg truncate">
                {{ row.entry.zhName || row.entry.title || row.entry.fileName }}
              </div>
              <div class="text-xs text-fg-muted truncate">
                {{ subtitle(row.entry) }}
              </div>
            </div>
            <MSwitch
              v-model="row.lockVersion"
              size="sm"
              :disabled="!row.entry.fileId"
              :label="
                row.lockVersion
                  ? t('resources.saveToPreset.locked')
                  : t('resources.saveToPreset.unlocked')
              "
            />
          </div>
        </div>
      </div>

      <div v-else class="text-xs text-fg-muted">
        {{ t('resources.saveToPreset.noRecognised') }}
      </div>

      <div
        v-if="unrecognized.length > 0"
        class="text-xs text-fg-muted"
        data-testid="save-to-preset-unrecognized"
      >
        {{ t('resources.saveToPreset.unrecognisedHint', { count: unrecognized.length }) }}
      </div>

      <div v-if="error" class="text-xs text-danger break-words">{{ error }}</div>

      <div v-if="outcomes.length > 0" class="space-y-1.5" data-testid="save-to-preset-outcomes">
        <div class="text-xs font-semibold text-fg-muted">
          {{ t('resources.saveToPreset.outcomeTitle') }}
          <span v-if="savedTo" class="text-fg-muted">· {{ savedTo.name }}</span>
        </div>
        <div
          v-for="outcome in outcomes"
          :key="outcome.platform + outcome.projectId"
          class="flex items-center gap-2 p-2 rounded-lg border border-line bg-panel"
        >
          <span :class="outcomeIcon(outcome.status)" class="w-4 h-4 shrink-0" />
          <div class="min-w-0 flex-1">
            <div class="text-xs text-fg truncate">{{ outcome.title }}</div>
            <div v-if="outcome.latestFileName" class="text-[11px] text-fg-muted truncate">
              {{ outcome.latestFileName }}
            </div>
          </div>
          <MBadge :variant="outcomeVariant(outcome.status)" size="sm">
            {{ outcomeText(outcome.status) }}
          </MBadge>
        </div>
      </div>
    </div>

    <div class="flex items-center justify-end gap-2 pt-3">
      <MButton variant="tonal" size="sm" @click="close">
        {{ t('resources.saveToPreset.close') }}
      </MButton>
      <MButton
        variant="primary"
        size="sm"
        :loading="submitting"
        :disabled="!canSubmit"
        data-testid="save-to-preset-confirm"
        @click="submit"
      >
        {{ t('resources.saveToPreset.submit') }}
      </MButton>
    </div>
  </MDialog>
</template>

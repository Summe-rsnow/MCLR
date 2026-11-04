<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type {
  AddItemOutcome,
  AddStatus,
  CompProject,
  PresetItem,
  ResourcePreset,
} from '../../../ipc/client'
import { ipcClient } from '../../../ipc/client'
import { MDialog, MSelect, MInput, MButton, MBadge, MImage } from '../../../ui'

const props = defineProps<{
  open: boolean
  project: CompProject | null
  /** Set when the user pinned one concrete version from the file list. */
  pinnedFileId?: string | null
  pinnedFileName?: string | null
}>()

const emit = defineEmits<{
  (e: 'update:open', value: boolean): void
  (e: 'added', preset: ResourcePreset): void
}>()

const { t } = useI18n()

/** Sentinel value for the "create a new preset" option. */
const NEW_PRESET = '__new__'

const presets = ref<ResourcePreset[]>([])
const selected = ref<string>(NEW_PRESET)
const newName = ref('')
const loading = ref(false)
const submitting = ref(false)
const outcomes = ref<AddItemOutcome[]>([])
const error = ref<string | null>(null)
const addedTo = ref<ResourcePreset | null>(null)

const creating = computed(
  () => selected.value === NEW_PRESET || !presets.value.some((p) => p.id === selected.value),
)

const options = computed(() => [
  ...presets.value.map((preset) => ({ label: preset.name, value: preset.id })),
  { label: t('comp.addToPreset.newPreset'), value: NEW_PRESET },
])

const canSubmit = computed(
  () => Boolean(props.project) && (!creating.value || newName.value.trim().length > 0),
)

watch(
  () => props.open,
  async (open) => {
    if (!open) return
    outcomes.value = []
    error.value = null
    addedTo.value = null
    newName.value = ''
    loading.value = true
    try {
      presets.value = await ipcClient.presetsList()
      selected.value = presets.value[0]?.id ?? NEW_PRESET
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
  return t(`comp.addToPreset.outcome.${status}`)
}

async function submit() {
  if (!props.project || !canSubmit.value) return
  submitting.value = true
  error.value = null
  outcomes.value = []

  try {
    let presetId = selected.value
    let preset = presets.value.find((p) => p.id === presetId) ?? null
    if (creating.value) {
      preset = await ipcClient.presetsSave({ name: newName.value.trim() })
      presets.value = [...presets.value, preset]
      presetId = preset.id
      selected.value = preset.id
    }

    const item: PresetItem = {
      platform: props.project.platform,
      projectId: props.project.id,
      kind: props.project.compType,
      title: props.project.title,
      slug: props.project.slug,
      iconUrl: props.project.iconUrl ?? null,
      pinnedFileId: props.pinnedFileId ?? null,
      pinnedFileName: props.pinnedFileName ?? null,
    }
    outcomes.value = await ipcClient.presetsAddItems(presetId, [item])
    if (preset) {
      addedTo.value = preset
      emit('added', preset)
    }
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
    :title="t('comp.addToPreset.title')"
    max-width="md"
    hide-footer
    @update:open="emit('update:open', $event)"
  >
    <div class="space-y-4 py-1">
      <div v-if="project" class="flex items-center gap-3 p-2 rounded-xl bg-sunken/50">
        <MImage
          :src="project.iconUrl"
          fallback-icon="i-lucide-puzzle"
          class="w-9 h-9 rounded-lg shrink-0"
        />
        <div class="min-w-0">
          <div class="text-sm font-medium text-fg truncate">
            {{ project.zhName || project.title }}
          </div>
          <div class="text-xs text-fg-muted truncate">
            {{ project.platform }} · {{ project.compType }}
          </div>
        </div>
      </div>

      <div class="space-y-1.5">
        <label class="text-xs font-semibold text-fg-muted block">
          {{ t('comp.addToPreset.selectPreset') }}
        </label>
        <MSelect v-model="selected" :options="options" :loading="loading" size="sm" />
      </div>

      <div v-if="creating" class="space-y-1.5">
        <label class="text-xs font-semibold text-fg-muted block">
          {{ t('comp.addToPreset.newPresetName') }}
        </label>
        <MInput
          v-model="newName"
          :placeholder="t('comp.addToPreset.newPresetPlaceholder')"
          size="sm"
          clearable
        />
      </div>

      <div
        v-if="pinnedFileName"
        class="text-xs text-fg-muted flex items-center gap-1.5"
        data-testid="add-to-preset-pin-hint"
      >
        <span class="i-lucide-pin w-3.5 h-3.5 text-primary" />
        {{ t('comp.addToPreset.pinHint', { file: pinnedFileName }) }}
      </div>

      <div v-if="error" class="text-xs text-danger break-words">{{ error }}</div>

      <div v-if="outcomes.length > 0" class="space-y-1.5" data-testid="add-to-preset-outcomes">
        <div class="text-xs font-semibold text-fg-muted">
          {{ t('comp.addToPreset.outcomeTitle') }}
          <span v-if="addedTo" class="text-fg-muted">· {{ addedTo.name }}</span>
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
        {{ t('comp.addToPreset.close') }}
      </MButton>
      <MButton
        variant="primary"
        size="sm"
        :loading="submitting"
        :disabled="!canSubmit"
        data-testid="add-to-preset-confirm"
        @click="submit"
      >
        {{ t('comp.addToPreset.confirm') }}
      </MButton>
    </div>
  </MDialog>
</template>

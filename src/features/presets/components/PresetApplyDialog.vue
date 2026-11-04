<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type { ApplyReport, ResourcePreset, SaveInfo } from '../../../ipc/client'
import { ipcClient } from '../../../ipc/client'
import {
  MBadge,
  MCheckbox,
  MDialog,
  MEmpty,
  MSegmented,
  MSelect,
  MSwitch,
  compTypeMeta,
  useToast,
} from '../../../ui'
import { useErrorText } from '../../../composables/useErrorText'
import { usePresetsStore } from '../store'
import ApplyReportDialog from './ApplyReportDialog.vue'

const props = defineProps<{
  open: boolean
  instanceId: string
}>()

const emit = defineEmits<{
  (e: 'update:open', val: boolean): void
  (e: 'applied', report: ApplyReport): void
}>()

const { t } = useI18n()
const toast = useToast()
const { formatError } = useErrorText()
const store = usePresetsStore()

const selectedIds = ref<string[]>([])
const world = ref('')
const saves = ref<SaveInfo[]>([])
const savesLoading = ref(false)
const includeDependencies = ref(true)
const onOtherVersion = ref<string>('skip')
const submitting = ref(false)
const reportOpen = ref(false)
const report = ref<ApplyReport | null>(null)

const selectedPresets = computed<ResourcePreset[]>(() =>
  selectedIds.value
    .map((id) => store.presets.find((preset) => preset.id === id))
    .filter((preset): preset is ResourcePreset => Boolean(preset)),
)

const hasDatapack = computed(() =>
  selectedPresets.value.some((preset) => preset.items.some((item) => item.kind === 'datapack')),
)

const worldOptions = computed(() =>
  saves.value.map((save) => ({ label: save.name || save.folderName, value: save.folderName })),
)

const canConfirm = computed(
  () => selectedIds.value.length > 0 && (!hasDatapack.value || world.value.length > 0),
)

const otherVersionOptions = computed(() => [
  { label: t('presets.apply.skip'), value: 'skip' },
  { label: t('presets.apply.replace'), value: 'replace' },
])

async function loadSaves(): Promise<void> {
  if (!props.instanceId) {
    saves.value = []
    return
  }
  savesLoading.value = true
  try {
    saves.value = await ipcClient.savesList(props.instanceId)
  } catch {
    saves.value = []
  } finally {
    savesLoading.value = false
  }
}

watch(
  () => props.open,
  async (open) => {
    if (!open) return
    selectedIds.value = []
    world.value = ''
    includeDependencies.value = true
    onOtherVersion.value = 'skip'
    if (store.presets.length === 0) {
      try {
        await store.loadPresets()
      } catch (error) {
        toast.danger(formatError(error))
      }
    }
    await loadSaves()
  },
  { immediate: true },
)

function isSelected(id: string): boolean {
  return selectedIds.value.includes(id)
}

function orderIndex(id: string): number {
  return selectedIds.value.indexOf(id) + 1
}

function togglePreset(id: string): void {
  selectedIds.value = isSelected(id)
    ? selectedIds.value.filter((item) => item !== id)
    : [...selectedIds.value, id]
}

function presetSummary(preset: ResourcePreset): string {
  return t('presets.apply.itemCount', { count: preset.items.length })
}

async function confirm(): Promise<void> {
  if (!canConfirm.value || submitting.value) return
  submitting.value = true
  try {
    const result = await store.applyPresets({
      instanceId: props.instanceId,
      presetIds: [...selectedIds.value],
      datapackWorld: hasDatapack.value ? world.value : null,
      includeDependencies: includeDependencies.value,
      onOtherVersion: onOtherVersion.value as 'skip' | 'replace',
    })
    report.value = result
    reportOpen.value = true
    emit('applied', result)
    emit('update:open', false)
  } catch (error) {
    toast.danger(formatError(error))
  } finally {
    submitting.value = false
  }
}
</script>

<template>
  <MDialog
    :open="open"
    :title="t('presets.apply.title')"
    :confirm-text="t('presets.apply.apply')"
    :confirm-disabled="!canConfirm"
    :confirm-loading="submitting"
    max-width="lg"
    @update:open="emit('update:open', $event)"
    @confirm="confirm"
  >
    <div class="flex flex-col gap-4" data-testid="preset-apply">
      <p class="text-xs text-fg-muted">{{ t('presets.apply.subtitle') }}</p>

      <MEmpty
        v-if="store.presets.length === 0"
        :title="t('presets.apply.empty')"
        icon="i-lucide-package-open"
      />

      <div v-else class="flex flex-col gap-1.5 max-h-64 overflow-y-auto pr-1">
        <div
          v-for="preset in store.presets"
          :key="preset.id"
          class="flex items-center gap-3 rounded-xl border border-line px-3 py-2 transition-colors"
          :class="{ 'border-primary/50 bg-primary-soft/40': isSelected(preset.id) }"
          data-testid="apply-preset-row"
          @click="togglePreset(preset.id)"
        >
          <MCheckbox
            :model-value="isSelected(preset.id)"
            @click.stop
            @update:model-value="togglePreset(preset.id)"
          />
          <div class="min-w-0 flex-1">
            <div class="text-sm font-medium text-fg truncate">{{ preset.name }}</div>
            <div class="text-xs text-fg-subtle truncate">
              {{ presetSummary(preset) }}
            </div>
          </div>
          <MBadge
            v-if="isSelected(preset.id)"
            variant="primary"
            size="sm"
            :data-testid="`apply-order-${preset.id}`"
          >
            {{ t('presets.apply.order', { index: orderIndex(preset.id) }) }}
          </MBadge>
        </div>
      </div>

      <div v-if="hasDatapack" class="flex flex-col gap-2">
        <span class="text-sm font-medium text-fg">{{ t('presets.apply.world') }}</span>
        <MSelect
          v-model="world"
          :options="worldOptions"
          :placeholder="
            savesLoading ? t('presets.apply.loadingSaves') : t('presets.apply.worldPlaceholder')
          "
          :loading="savesLoading"
        />
        <p class="text-xs text-warning-fg">{{ t('presets.apply.worldRequired') }}</p>
      </div>

      <div class="flex items-center justify-between gap-3 py-1">
        <span class="text-sm text-fg">{{ t('presets.apply.includeDependencies') }}</span>
        <MSwitch v-model="includeDependencies" />
      </div>

      <div class="flex items-center justify-between gap-3 py-1">
        <span class="text-sm text-fg">{{ t('presets.apply.otherVersion') }}</span>
        <MSegmented v-model="onOtherVersion" :options="otherVersionOptions" size="sm" />
      </div>

      <div v-if="selectedPresets.length > 0" class="flex items-center gap-1.5 flex-wrap">
        <span class="text-xs text-fg-muted">{{ t('presets.apply.selected') }}</span>
        <MBadge
          v-for="item in selectedPresets.flatMap((preset) => preset.items)"
          :key="`${item.platform}:${item.projectId}`"
          variant="neutral"
          size="sm"
          :icon="compTypeMeta[item.kind]?.icon"
        >
          {{ item.title }}
        </MBadge>
      </div>
    </div>
  </MDialog>

  <ApplyReportDialog v-model:open="reportOpen" :report="report" />
</template>

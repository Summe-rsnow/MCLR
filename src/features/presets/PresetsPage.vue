<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type { CompType, PresetItem, ResourcePreset } from '../../ipc/client'
import {
  MBadge,
  MButton,
  MCard,
  MDialog,
  MEmpty,
  MIconButton,
  MImage,
  MInput,
  MBrandIcon,
  MListItem,
  MPageHeader,
  MSection,
  MTabs,
  MTextarea,
  compTypeMeta,
  useToast,
} from '../../ui'
import { useFormat } from '../../composables/useFormat'
import { useErrorText } from '../../composables/useErrorText'
import { usePresetsStore } from './store'
import PresetPinDrawer from './components/PresetPinDrawer.vue'
import PresetStoreDialog from './components/PresetStoreDialog.vue'

const { t } = useI18n()
const toast = useToast()
const { formatError } = useErrorText()
const { formatDate } = useFormat()
const store = usePresetsStore()

const KIND_ORDER: CompType[] = ['mod', 'resourcepack', 'shader', 'datapack', 'world']

const activePresetId = ref('')
const activeTab = ref<'items' | 'settings'>('items')
const createOpen = ref(false)
const newPresetName = ref('')
const pinOpen = ref(false)
const pinTarget = ref<{ platform: string; projectId: string } | null>(null)
const pinItem = computed<PresetItem | null>(() => {
  if (!pinTarget.value || !activePreset.value) return null
  return (
    activePreset.value.items.find(
      (i) => i.platform === pinTarget.value!.platform && i.projectId === pinTarget.value!.projectId,
    ) ?? null
  )
})
const storeOpen = ref(false)

const draftName = ref('')
const draftDescription = ref('')

const activePreset = computed<ResourcePreset | null>(
  () => store.presets.find((p) => p.id === activePresetId.value) ?? null,
)

const tabs = computed(() => [
  { key: 'items', label: t('presets.detail.tabs.items'), icon: 'i-lucide-package' },
  {
    key: 'settings',
    label: t('presets.detail.tabs.settings'),
    icon: 'i-lucide-sliders-horizontal',
  },
])

const groupedItems = computed(() =>
  KIND_ORDER.map((kind) => ({
    kind,
    items: (activePreset.value?.items ?? []).filter((item) => item.kind === kind),
  })).filter((group) => group.items.length > 0),
)

const canSaveSettings = computed(() => draftName.value.trim().length > 0)

watch(
  activePreset,
  (preset) => {
    if (!preset) return
    draftName.value = preset.name
    draftDescription.value = preset.description
  },
  { immediate: true },
)

function kindLabel(kind: CompType): string {
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

function presetSubtitle(preset: ResourcePreset): string {
  return t('presets.list.items', { count: preset.items.length })
}

function isCached(item: PresetItem): boolean {
  const entries = store.storeInfo?.entries
  if (!entries) return false
  return entries.some(
    (entry) =>
      entry.platform === item.platform &&
      entry.projectId === item.projectId &&
      (item.pinnedFileId ? entry.fileId === item.pinnedFileId : entry.kind === item.kind),
  )
}

function selectPreset(id: string): void {
  activePresetId.value = id
  activeTab.value = 'items'
}

async function loadAll(): Promise<void> {
  try {
    const presets = await store.loadPresets()
    if (!activePresetId.value || !presets.some((p) => p.id === activePresetId.value)) {
      activePresetId.value = presets[0]?.id ?? ''
    }
  } catch (error) {
    toast.danger(formatError(error))
  }
  // Cache badges are best-effort: a store that cannot be read must not blank the page.
  try {
    await store.loadStoreInfo()
  } catch {
    // ignore
  }
}

onMounted(loadAll)

async function createPreset(): Promise<void> {
  const name = newPresetName.value.trim()
  if (!name) return
  try {
    const created = await store.savePreset({
      name,
      description: '',
      gameVersions: [],
      loaders: [],
      items: [],
    })
    activePresetId.value = created.id
    draftName.value = created.name
    draftDescription.value = ''
    activeTab.value = 'settings'
    createOpen.value = false
    newPresetName.value = ''
  } catch (error) {
    toast.danger(formatError(error))
  }
}

async function saveSettings(): Promise<void> {
  const preset = activePreset.value
  if (!preset || !canSaveSettings.value) return
  try {
    await store.savePreset({
      ...preset,
      name: draftName.value.trim(),
      description: draftDescription.value.trim(),
      // Presets are universal lists; the legacy filter fields are cleared on every save.
      gameVersions: [],
      loaders: [],
    })
    toast.success(t('presets.settings.saved'))
  } catch (error) {
    toast.danger(formatError(error))
  }
}

async function deletePreset(): Promise<void> {
  const preset = activePreset.value
  if (!preset) return
  try {
    const deleted = await store.deletePreset(preset.id)
    if (!deleted) return
    activePresetId.value = store.presets[0]?.id ?? ''
    toast.success(t('presets.settings.deleted'))
  } catch (error) {
    toast.danger(formatError(error))
  }
}

function openPinDrawer(item: PresetItem): void {
  pinTarget.value = { platform: item.platform, projectId: item.projectId }
  pinOpen.value = true
}

async function unpin(item: PresetItem): Promise<void> {
  const preset = activePreset.value
  if (!preset) return
  try {
    await store.pin(preset.id, item.platform, item.projectId)
    toast.success(t('presets.item.unpinned'))
  } catch (error) {
    toast.danger(formatError(error))
  }
}

async function removeItem(item: PresetItem): Promise<void> {
  const preset = activePreset.value
  if (!preset) return
  try {
    await store.removeItem(preset.id, item.platform, item.projectId)
    toast.success(t('presets.item.removed'))
  } catch (error) {
    toast.danger(formatError(error))
  }
}
</script>

<template>
  <div class="m-page">
    <MPageHeader
      :title="t('presets.title')"
      :description="t('presets.subtitle')"
      icon="i-lucide-package-check"
    >
      <template #actions>
        <MButton
          variant="tonal"
          icon="i-lucide-hard-drive"
          data-testid="open-store-btn"
          @click="storeOpen = true"
        >
          {{ t('presets.actions.store') }}
        </MButton>
        <MButton
          variant="primary"
          icon="i-lucide-plus"
          data-testid="create-preset-btn"
          @click="createOpen = true"
        >
          {{ t('presets.actions.create') }}
        </MButton>
      </template>
    </MPageHeader>

    <div class="grid grid-cols-1 lg:grid-cols-[300px_minmax(0,1fr)] gap-4 items-start">
      <MCard padding="sm" class="lg:sticky lg:top-4">
        <div v-if="store.presets.length === 0" class="py-6">
          <MEmpty :title="t('presets.empty.title')" :description="t('presets.empty.description')">
            <MButton variant="primary" size="sm" icon="i-lucide-plus" @click="createOpen = true">
              {{ t('presets.actions.create') }}
            </MButton>
          </MEmpty>
        </div>
        <div v-else class="flex flex-col gap-1" data-testid="preset-list">
          <MListItem
            v-for="preset in store.presets"
            :key="preset.id"
            :title="preset.name"
            :active="preset.id === activePresetId"
            data-testid="preset-list-item"
            @click="selectPreset(preset.id)"
          >
            <template #subtitle>
              <span class="text-xs text-fg-muted truncate">{{ presetSubtitle(preset) }}</span>
            </template>
          </MListItem>
        </div>
      </MCard>

      <div v-if="activePreset" class="flex flex-col gap-4 min-w-0">
        <MCard padding="md">
          <div class="flex items-start justify-between gap-4 flex-wrap">
            <div class="min-w-0">
              <h2 class="text-xl font-bold text-fg truncate">{{ activePreset.name }}</h2>
              <p v-if="activePreset.description" class="text-xs text-fg-muted mt-1">
                {{ activePreset.description }}
              </p>
            </div>
            <div class="flex items-center gap-2 shrink-0">
              <MBadge variant="neutral" size="sm">
                {{
                  t('presets.list.updated', {
                    date: formatDate(new Date(activePreset.updatedAt).toISOString()),
                  })
                }}
              </MBadge>
            </div>
          </div>
          <MTabs v-model="activeTab" :tabs="tabs" class="mt-3" />
        </MCard>

        <template v-if="activeTab === 'items'">
          <MEmpty
            v-if="activePreset.items.length === 0"
            :title="t('presets.detail.noItems')"
            :description="t('presets.detail.noItemsHint')"
            icon="i-lucide-package-open"
          />
          <MSection
            v-for="group in groupedItems"
            :key="group.kind"
            :title="kindLabel(group.kind)"
            :icon="compTypeMeta[group.kind]?.icon"
          >
            <div class="flex flex-col divide-y divide-line/60">
              <div
                v-for="item in group.items"
                :key="`${item.platform}:${item.projectId}`"
                class="flex items-center gap-3 py-2.5 min-w-0"
                data-testid="preset-item-row"
              >
                <MImage :src="item.iconUrl ?? null" :alt="item.title" size="sm" />
                <div class="min-w-0 flex-1">
                  <div class="flex items-center gap-1.5 min-w-0">
                    <span class="text-sm font-medium text-fg truncate">{{ item.title }}</span>
                    <MBrandIcon :name="item.platform" class="w-3.5 h-3.5 text-fg-muted" />
                  </div>
                  <span v-if="item.slug" class="text-xs text-fg-subtle truncate block">
                    {{ item.slug }}
                  </span>
                </div>
                <MBadge v-if="isCached(item)" variant="success" size="sm" icon="i-lucide-database">
                  {{ t('presets.item.cached') }}
                </MBadge>
                <MBadge v-if="item.pinnedFileName" variant="primary" size="sm" icon="i-lucide-pin">
                  {{ t('presets.item.pinned', { name: item.pinnedFileName }) }}
                </MBadge>
                <MBadge v-else variant="neutral" size="sm" icon="i-lucide-arrow-down-to-line">
                  {{ t('presets.item.followingLatest') }}
                </MBadge>
                <div class="flex items-center gap-1 shrink-0">
                  <MButton
                    variant="text"
                    size="xs"
                    icon="i-lucide-pin"
                    data-testid="pin-item-btn"
                    @click="openPinDrawer(item)"
                  >
                    {{ t('presets.item.pin') }}
                  </MButton>
                  <MButton
                    v-if="item.pinnedFileName"
                    variant="text"
                    size="xs"
                    icon="i-lucide-pin-off"
                    @click="unpin(item)"
                  >
                    {{ t('presets.item.unpin') }}
                  </MButton>
                  <MIconButton
                    icon="i-lucide-trash-2"
                    variant="text"
                    size="sm"
                    :title="t('presets.item.remove')"
                    data-testid="remove-item-btn"
                    @click="removeItem(item)"
                  />
                </div>
              </div>
            </div>
          </MSection>
        </template>

        <MCard v-else-if="activeTab === 'settings'" padding="md" class="flex flex-col gap-5">
          <MInput
            v-model="draftName"
            :label="t('presets.settings.name')"
            :placeholder="t('presets.createDialog.namePlaceholder')"
          />
          <MTextarea
            v-model="draftDescription"
            :label="t('presets.settings.description')"
            :placeholder="t('presets.settings.descriptionPlaceholder')"
            :rows="2"
          />

          <div class="flex items-center justify-between gap-3 pt-2 border-t border-line">
            <MButton
              variant="danger"
              icon="i-lucide-trash-2"
              data-testid="delete-preset-btn"
              @click="deletePreset"
            >
              {{ t('presets.settings.delete') }}
            </MButton>
            <MButton
              variant="primary"
              icon="i-lucide-save"
              :disabled="!canSaveSettings"
              data-testid="save-preset-btn"
              @click="saveSettings"
            >
              {{ t('presets.settings.save') }}
            </MButton>
          </div>
        </MCard>
      </div>

      <MEmpty v-else :title="t('presets.empty.title')" icon="i-lucide-package-open" />
    </div>

    <MDialog
      v-model:open="createOpen"
      :title="t('presets.createDialog.title')"
      :confirm-text="t('presets.actions.create')"
      :confirm-disabled="!newPresetName.trim()"
      @confirm="createPreset"
    >
      <MInput
        v-model="newPresetName"
        :label="t('presets.createDialog.nameLabel')"
        :placeholder="t('presets.createDialog.namePlaceholder')"
        autofocus
        data-testid="new-preset-name"
      />
    </MDialog>

    <PresetPinDrawer v-model:open="pinOpen" :preset="activePreset" :item="pinItem" />

    <PresetStoreDialog
      v-model:open="storeOpen"
      :preset-id="activePreset?.id ?? null"
      :preset-name="activePreset?.name ?? null"
    />
  </div>
</template>

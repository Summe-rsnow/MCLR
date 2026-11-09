<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { compareVersionsDesc } from '@/shared/mcver'
import { compareLoaders } from '@/shared/loader'
import { useI18n } from 'vue-i18n'
import type { CompFile, PresetItem, ResourcePreset } from '../../../ipc/client'
import { ipcClient } from '../../../ipc/client'
import { MBadge, MButton, MDrawer, MEmpty, MSkeletonList, useToast } from '../../../ui'
import { useErrorText } from '../../../composables/useErrorText'
import { usePresetsStore } from '../store'

const props = defineProps<{
  open: boolean
  preset: ResourcePreset | null
  item: PresetItem | null
}>()

const emit = defineEmits<{
  (e: 'update:open', val: boolean): void
}>()

const { t } = useI18n()
const toast = useToast()
const { formatError } = useErrorText()
const store = usePresetsStore()

const files = ref<CompFile[]>([])
const loading = ref(false)
/** Browsing-only filters: they narrow the file list, they are never stored on the preset. */
const selectedVersionFilter = ref<string>('')
const selectedLoaderFilter = ref<string>('')

const title = computed(() => t('presets.pin.title'))
const subtitle = computed(() => props.item?.title ?? '')

const currentItem = computed<PresetItem | null>(() => {
  const item = props.item
  if (!item) return null
  return (
    props.preset?.items.find(
      (i) => i.platform === item.platform && i.projectId === item.projectId,
    ) ?? item
  )
})

const pinnedFileName = computed(() => currentItem.value?.pinnedFileName ?? null)

async function loadFiles(): Promise<void> {
  const item = props.item
  if (!item) {
    files.value = []
    return
  }
  loading.value = true
  try {
    files.value = await ipcClient.compFiles(
      item.platform,
      item.projectId,
      selectedVersionFilter.value || null,
      item.kind === 'mod' ? selectedLoaderFilter.value || null : null,
    )
  } catch (error) {
    files.value = []
    toast.danger(formatError(error))
  } finally {
    loading.value = false
  }
}

watch(
  () => [props.open, props.item?.platform, props.item?.projectId],
  ([open]) => {
    if (!open) return
    selectedVersionFilter.value = ''
    selectedLoaderFilter.value = ''
    void loadFiles()
  },
  { immediate: true },
)

watch([selectedVersionFilter, selectedLoaderFilter], () => {
  if (props.open) void loadFiles()
})

const availableVersions = computed(() => {
  const set = new Set<string>()
  for (const file of files.value) {
    for (const version of file.mcVersions) {
      if (version) set.add(version)
    }
  }
  return Array.from(set).sort(compareVersionsDesc)
})

const availableLoaders = computed(() => {
  const set = new Set<string>()
  for (const file of files.value) {
    for (const loader of file.loaders) {
      if (loader) set.add(loader)
    }
  }
  return Array.from(set).sort(compareLoaders)
})

const filteredFiles = computed(() => {
  return files.value.filter((file) => {
    if (selectedVersionFilter.value && !file.mcVersions.includes(selectedVersionFilter.value)) {
      return false
    }
    if (
      selectedLoaderFilter.value &&
      !file.loaders.some((l) => l.toLowerCase() === selectedLoaderFilter.value.toLowerCase())
    ) {
      return false
    }
    return true
  })
})

function isPinned(file: CompFile): boolean {
  return currentItem.value?.pinnedFileId === file.id
}

function fileSize(bytes: number): string {
  if (bytes >= 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
  return `${(bytes / 1024).toFixed(1)} KB`
}

/** One click pins the file; clicking the pinned file again restores "follow latest". */
async function togglePin(file: CompFile): Promise<void> {
  const preset = props.preset
  const item = props.item
  if (!preset || !item) return
  try {
    if (isPinned(file)) {
      await store.pin(preset.id, item.platform, item.projectId, null, null)
      toast.success(t('presets.item.unpinned'))
    } else {
      await store.pin(preset.id, item.platform, item.projectId, file.id, file.fileName)
      toast.success(t('presets.pin.picked'))
    }
  } catch (error) {
    toast.danger(formatError(error))
  }
}

async function followLatest(): Promise<void> {
  const preset = props.preset
  const item = props.item
  if (!preset || !item) return
  try {
    await store.pin(preset.id, item.platform, item.projectId, null, null)
    toast.success(t('presets.pin.followingLatest'))
  } catch (error) {
    toast.danger(formatError(error))
  }
}
</script>
<template>
  <MDrawer
    :open="open"
    :title="title"
    :subtitle="subtitle"
    size="md"
    @update:open="emit('update:open', $event)"
  >
    <div class="flex flex-col gap-4 h-full min-h-0">
      <!-- Lock status banner -->
      <div class="p-3 rounded-xl bg-sunken/40 border border-line space-y-2 shrink-0">
        <div class="flex items-center justify-between gap-2">
          <div class="flex items-center gap-1.5 min-w-0">
            <span
              class="w-4 h-4 shrink-0 text-primary"
              :class="pinnedFileName ? 'i-lucide-pin' : 'i-lucide-arrow-down-to-line'"
            />
            <span
              class="text-xs font-semibold text-fg truncate"
              data-testid="pin-status"
              :title="pinnedFileName ?? undefined"
            >
              {{
                pinnedFileName
                  ? t('presets.item.pinned', { name: pinnedFileName })
                  : t('presets.item.followingLatest')
              }}
            </span>
          </div>

          <MButton
            v-if="pinnedFileName"
            variant="text"
            size="xs"
            icon="i-lucide-pin-off"
            data-testid="unpin-btn"
            @click="followLatest"
          >
            {{ t('presets.pin.followLatest') }}
          </MButton>
        </div>

        <p class="text-[11px] text-fg-muted leading-relaxed flex items-start gap-1.5">
          <span class="i-lucide-info w-3.5 h-3.5 text-primary shrink-0 mt-0.5" />
          <span>{{ t('presets.pin.hint') }}</span>
        </p>
      </div>

      <!-- File picker -->
      <div class="flex flex-col gap-2.5 flex-1 min-h-0">
        <div class="text-xs font-semibold text-fg-muted flex items-center gap-1.5 shrink-0">
          <span class="i-lucide-file-search w-3.5 h-3.5" />
          <span>{{ t('presets.pin.pickFile') }}</span>
        </div>

        <!-- Browsing filters (game version & loader) -->
        <div class="p-2.5 rounded-xl bg-sunken/30 border border-line space-y-2 shrink-0 text-xs">
          <!-- Game Version Filter Chips -->
          <div class="flex items-start gap-1.5">
            <span class="font-semibold text-fg-muted shrink-0 pt-0.5">
              {{ t('presets.pin.filterVersion') }}:
            </span>
            <div class="flex flex-wrap items-center gap-1 flex-1 max-h-16 overflow-y-auto pr-1">
              <button
                type="button"
                class="px-2 py-0.5 rounded-md transition font-medium cursor-pointer"
                :class="
                  !selectedVersionFilter
                    ? 'bg-primary text-white shadow-xs'
                    : 'text-fg-muted hover:text-fg hover:bg-sunken'
                "
                @click="selectedVersionFilter = ''"
              >
                {{ t('presets.pin.allVersions') }}
              </button>
              <button
                v-for="ver in availableVersions"
                :key="ver"
                type="button"
                class="px-2 py-0.5 rounded-md transition font-mono cursor-pointer"
                :class="
                  selectedVersionFilter === ver
                    ? 'bg-primary text-white shadow-xs font-semibold'
                    : 'text-fg-muted hover:text-fg hover:bg-sunken'
                "
                @click="selectedVersionFilter = ver"
              >
                {{ ver }}
              </button>
            </div>
          </div>

          <!-- Loader Filter Chips -->
          <div
            v-if="item?.kind === 'mod' && availableLoaders.length > 0"
            class="flex items-center gap-1.5 pt-1.5 border-t border-line/40"
          >
            <span class="font-semibold text-fg-muted shrink-0">
              {{ t('presets.pin.filterLoader') }}:
            </span>
            <div class="flex flex-wrap items-center gap-1 flex-1">
              <button
                type="button"
                class="px-2 py-0.5 rounded-md transition font-medium cursor-pointer"
                :class="
                  !selectedLoaderFilter
                    ? 'bg-primary text-white shadow-xs'
                    : 'text-fg-muted hover:text-fg hover:bg-sunken'
                "
                @click="selectedLoaderFilter = ''"
              >
                {{ t('presets.pin.allLoaders') }}
              </button>
              <button
                v-for="ldr in availableLoaders"
                :key="ldr"
                type="button"
                class="px-2 py-0.5 rounded-md transition font-medium cursor-pointer capitalize"
                :class="
                  selectedLoaderFilter === ldr
                    ? 'bg-primary text-white shadow-xs font-semibold'
                    : 'text-fg-muted hover:text-fg hover:bg-sunken'
                "
                @click="selectedLoaderFilter = ldr"
              >
                {{ ldr }}
              </button>
            </div>
          </div>
        </div>

        <!-- Files List -->
        <MSkeletonList v-if="loading" :rows="6" variant="row" />

        <MEmpty
          v-else-if="filteredFiles.length === 0"
          :title="t('presets.pin.empty')"
          icon="i-lucide-file-x"
        />

        <div
          v-else
          class="flex flex-col divide-y divide-line/60 overflow-y-auto flex-1 min-h-0 pr-1"
        >
          <button
            v-for="file in filteredFiles"
            :key="file.id"
            type="button"
            class="flex items-center gap-3 py-2 text-left cursor-pointer hover:bg-sunken rounded-lg px-2 transition-colors"
            data-testid="pin-file-row"
            @click="togglePin(file)"
          >
            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-1.5 min-w-0">
                <span class="text-sm text-fg truncate">{{ file.fileName }}</span>
                <MBadge v-if="isPinned(file)" variant="primary" size="sm" icon="i-lucide-pin">
                  {{ t('presets.pin.current') }}
                </MBadge>
              </div>
              <div class="flex items-center gap-1.5 text-xs text-fg-subtle mt-0.5 flex-wrap">
                <span class="font-mono">{{ file.mcVersions.slice(0, 3).join(', ') }}</span>
                <span v-if="file.loaders.length > 0">• {{ file.loaders.join(', ') }}</span>
                <span>• {{ fileSize(file.size) }}</span>
              </div>
            </div>
            <span
              class="w-4 h-4 shrink-0"
              :class="
                isPinned(file) ? 'i-lucide-pin-off text-primary' : 'i-lucide-pin text-fg-subtle'
              "
            />
          </button>
        </div>
      </div>
    </div>
  </MDrawer>
</template>

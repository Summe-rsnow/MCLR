<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type { CompFile, PresetItem, PresetPinRule, ResourcePreset } from '../../../ipc/client'
import { ipcClient } from '../../../ipc/client'
import { MBadge, MButton, MDrawer, MEmpty, MIconButton, MSkeletonList, useToast } from '../../../ui'
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
const selectedVersionFilter = ref<string>('')
const selectedLoaderFilter = ref<string>('')

const title = computed(() => t('presets.pin.title'))
const subtitle = computed(() => props.item?.title ?? '')
async function loadFiles(): Promise<void> {
  const item = props.item
  const preset = props.preset
  if (!item) {
    files.value = []
    return
  }
  loading.value = true
  try {
    // If preset selected game version or loader, query ONLY the selected ones!
    const targetMcVersion =
      selectedVersionFilter.value ||
      (preset && preset.gameVersions.length > 0 ? preset.gameVersions[0] : null)
    const targetLoader =
      item.kind === 'mod'
        ? selectedLoaderFilter.value ||
          (preset && preset.loaders.length > 0 ? preset.loaders[0] : null)
        : null

    let list = await ipcClient.compFiles(
      item.platform,
      item.projectId,
      targetMcVersion,
      targetLoader,
    )

    // Strictly filter by preset game versions & loaders if preset has them configured
    if (preset && preset.gameVersions.length > 0) {
      list = list.filter((f) => f.mcVersions.some((v) => preset.gameVersions.includes(v)))
    }
    if (preset && preset.loaders.length > 0 && item.kind === 'mod') {
      const allowed = preset.loaders.map((l) => l.toLowerCase())
      list = list.filter((f) => f.loaders.some((l) => allowed.includes(l.toLowerCase())))
    }
    files.value = list
  } catch (error) {
    files.value = []
    toast.danger(formatError(error))
  } finally {
    loading.value = false
  }
}

watch(
  () => [props.open, props.preset?.id, props.item?.platform, props.item?.projectId],
  ([open]) => {
    if (open) {
      const preset = props.preset
      if (preset && preset.gameVersions.length > 0) {
        selectedVersionFilter.value = preset.gameVersions[0]
      } else {
        selectedVersionFilter.value = ''
      }
      if (preset && preset.loaders.length > 0) {
        selectedLoaderFilter.value = preset.loaders[0]
      } else {
        selectedLoaderFilter.value = ''
      }
      void loadFiles()
    }
  },
  { immediate: true },
)

watch([selectedVersionFilter, selectedLoaderFilter], () => {
  if (props.open) void loadFiles()
})
function compareMcVersionsDesc(a: string, b: string): number {
  const parseParts = (v: string) => {
    const parts = v
      .trim()
      .split('.')
      .map((p) => {
        const n = parseInt(p, 10)
        return isNaN(n) ? 0 : n
      })
    while (parts.length < 3 && parts[0] === 1) {
      parts.push(0)
    }
    return parts
  }

  const pa = parseParts(a)
  const pb = parseParts(b)
  const maxLen = Math.max(pa.length, pb.length)

  for (let i = 0; i < maxLen; i++) {
    const na = pa[i] ?? 0
    const nb = pb[i] ?? 0
    if (na !== nb) {
      return nb - na
    }
  }
  return b.localeCompare(a)
}

const LOADER_ORDER: Record<string, number> = {
  fabric: 1,
  quilt: 2,
  neoforge: 3,
  forge: 4,
}

function compareLoaders(a: string, b: string): number {
  const oa = LOADER_ORDER[a.toLowerCase()] ?? 99
  const ob = LOADER_ORDER[b.toLowerCase()] ?? 99
  if (oa !== ob) return oa - ob
  return a.localeCompare(b)
}

const availableVersions = computed(() => {
  const preset = props.preset
  if (preset && preset.gameVersions.length > 0) {
    return [...preset.gameVersions].sort(compareMcVersionsDesc)
  }
  const set = new Set<string>()
  for (const f of files.value) {
    for (const v of f.mcVersions) {
      if (v) set.add(v)
    }
  }
  return Array.from(set).sort(compareMcVersionsDesc)
})

const availableLoaders = computed(() => {
  const preset = props.preset
  if (preset && preset.loaders.length > 0 && props.item?.kind === 'mod') {
    return [...preset.loaders].sort(compareLoaders)
  }
  const set = new Set<string>()
  for (const f of files.value) {
    for (const l of f.loaders) {
      if (l) set.add(l)
    }
  }
  return Array.from(set).sort(compareLoaders)
})
const filteredFiles = computed(() => {
  return files.value.filter((f) => {
    if (selectedVersionFilter.value && !f.mcVersions.includes(selectedVersionFilter.value)) {
      return false
    }
    if (
      selectedLoaderFilter.value &&
      !f.loaders.some((l) => l.toLowerCase() === selectedLoaderFilter.value.toLowerCase())
    ) {
      return false
    }
    return true
  })
})

const currentItem = computed<PresetItem | null>(() => {
  if (!props.item) return null
  return (
    props.preset?.items.find(
      (i) => i.platform === props.item?.platform && i.projectId === props.item?.projectId,
    ) ?? props.item
  )
})

const currentRules = computed<PresetPinRule[]>(() => {
  const item = currentItem.value
  if (!item) return []
  // Never display empty wildcard rules without specific versions or loaders
  return (item.pinRules ?? []).filter(
    (r) => r.gameVersions.length > 0 || (r.loaders && r.loaders.length > 0),
  )
})

const hasRules = computed(() => currentRules.value.length > 0)

function isFilePinnedInRule(fileId: string): boolean {
  return currentRules.value.some((r) => r.fileId === fileId)
}

function supportsPresetFilters(file: CompFile): boolean {
  const preset = props.preset
  const item = props.item
  if (!preset || !item) return true
  const matchesVersion =
    preset.gameVersions.length === 0 ||
    file.mcVersions.some((version) => preset.gameVersions.includes(version))
  const matchesLoader =
    item.kind !== 'mod' ||
    preset.loaders.length === 0 ||
    file.loaders.some((loader) => preset.loaders.includes(loader.toLowerCase()))
  return matchesVersion && matchesLoader
}

function fileSize(bytes: number): string {
  if (bytes >= 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
  return `${(bytes / 1024).toFixed(1)} KB`
}

async function pinFile(file: CompFile): Promise<void> {
  const preset = props.preset
  const item = props.item
  if (!preset || !item) return
  try {
    let gameVersions = selectedVersionFilter.value ? [selectedVersionFilter.value] : []
    if (gameVersions.length === 0) {
      if (preset.gameVersions.length > 0) {
        gameVersions = file.mcVersions.filter((v) => preset.gameVersions.includes(v))
      }
      if (gameVersions.length === 0) {
        gameVersions = file.mcVersions.slice(0, 1)
      }
    }

    let loaders = selectedLoaderFilter.value ? [selectedLoaderFilter.value] : []
    if (loaders.length === 0 && item.kind === 'mod') {
      if (preset.loaders.length > 0) {
        loaders = file.loaders.filter((l) =>
          preset.loaders.some((pl) => pl.toLowerCase() === l.toLowerCase()),
        )
      }
      if (loaders.length === 0 && file.loaders.length > 0) {
        loaders = [file.loaders[0]]
      }
    }

    const rule: PresetPinRule = {
      gameVersions,
      loaders,
      fileId: file.id,
      fileName: file.fileName,
    }
    await store.pin(preset.id, item.platform, item.projectId, file.id, file.fileName)
    await store.pinRule(preset.id, item.platform, item.projectId, rule)
    toast.success(t('presets.pin.ruleAdded'))
  } catch (error) {
    toast.danger(formatError(error))
  }
}

async function removeRule(fileId: string): Promise<void> {
  const preset = props.preset
  const item = props.item
  if (!preset || !item) return
  try {
    await store.removePinRule(preset.id, item.platform, item.projectId, fileId)
    toast.success(t('presets.pin.ruleRemoved'))
  } catch (error) {
    toast.danger(formatError(error))
  }
}

async function followLatest(): Promise<void> {
  const preset = props.preset
  const item = props.item
  if (!preset || !item) return
  try {
    await store.pin(preset.id, item.platform, item.projectId)
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
      <!-- Active Rules Section -->
      <div class="p-3 rounded-xl bg-sunken/40 border border-line space-y-2.5 shrink-0">
        <div class="flex items-center justify-between gap-2">
          <div class="flex items-center gap-1.5">
            <span class="i-lucide-list-checks w-4 h-4 text-primary" />
            <span class="text-xs font-bold text-fg">{{ t('presets.pin.rulesTitle') }}</span>
            <MBadge variant="neutral" size="sm">{{ currentRules.length }}</MBadge>
          </div>

          <MButton
            variant="text"
            size="xs"
            icon="i-lucide-arrow-down-to-line"
            :disabled="!hasRules"
            @click="followLatest"
          >
            {{ t('presets.pin.followLatest') }}
          </MButton>
        </div>

        <!-- Rules List -->
        <div v-if="currentRules.length > 0" class="space-y-1.5 max-h-36 overflow-y-auto pr-1">
          <div
            v-for="rule in currentRules"
            :key="rule.fileId"
            class="flex items-center justify-between gap-2 p-2 rounded-lg bg-panel border border-line text-xs"
          >
            <div class="min-w-0 flex-1 space-y-0.5">
              <div class="flex items-center gap-1.5 flex-wrap">
                <MBadge variant="primary" size="sm">
                  {{
                    rule.gameVersions.length > 0
                      ? rule.gameVersions.join(', ')
                      : t('presets.pin.anyVersion')
                  }}
                </MBadge>
                <MBadge v-if="item?.kind === 'mod'" variant="neutral" size="sm">
                  {{
                    rule.loaders.length > 0 ? rule.loaders.join(', ') : t('presets.pin.anyLoader')
                  }}
                </MBadge>
              </div>
              <div class="font-mono text-fg-muted truncate" :title="rule.fileName">
                {{ rule.fileName }}
              </div>
            </div>

            <MIconButton
              icon="i-lucide-trash-2"
              variant="text"
              size="sm"
              class="text-danger hover:opacity-80 shrink-0"
              :tooltip="t('presets.pin.deleteRule')"
              @click="removeRule(rule.fileId)"
            />
          </div>
        </div>

        <!-- Rules Hint -->
        <p class="text-[11px] text-fg-muted leading-relaxed flex items-start gap-1.5">
          <span class="i-lucide-info w-3.5 h-3.5 text-primary shrink-0 mt-0.5" />
          <span>{{ t('presets.pin.rulesHint') }}</span>
        </p>
      </div>

      <!-- Add Rule / File Picker Section -->
      <div class="flex flex-col gap-2.5 flex-1 min-h-0">
        <div class="text-xs font-semibold text-fg-muted flex items-center gap-1.5 shrink-0">
          <span class="i-lucide-plus-circle w-3.5 h-3.5" />
          <span>{{ t('presets.pin.addRuleTitle') }}</span>
        </div>

        <!-- Filters (Game Version & Loader) -->
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
                    ? 'bg-primary text-white shadow-xs'
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
            @click="pinFile(file)"
          >
            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-1.5 min-w-0">
                <span class="text-sm text-fg truncate">{{ file.fileName }}</span>
                <MBadge
                  v-if="isFilePinnedInRule(file.id)"
                  variant="primary"
                  size="sm"
                  icon="i-lucide-pin"
                >
                  {{ t('presets.pin.current') }}
                </MBadge>
              </div>
              <div class="flex items-center gap-1.5 text-xs text-fg-subtle mt-0.5 flex-wrap">
                <span class="font-mono">{{ file.mcVersions.slice(0, 3).join(', ') }}</span>
                <span v-if="file.loaders.length > 0">• {{ file.loaders.join(', ') }}</span>
                <span>• {{ fileSize(file.size) }}</span>
              </div>
            </div>
            <MBadge :variant="supportsPresetFilters(file) ? 'success' : 'warning'" size="sm">
              {{
                supportsPresetFilters(file)
                  ? t('presets.pin.compatible')
                  : t('presets.pin.incompatible')
              }}
            </MBadge>
            <MIconButton
              icon="i-lucide-pin"
              variant="text"
              size="sm"
              :title="t('presets.pin.title')"
            />
          </button>
        </div>
      </div>
    </div>
  </MDrawer>
</template>

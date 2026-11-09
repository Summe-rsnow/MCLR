<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { compareVersionsDesc } from '@/shared/mcver'
import { compareLoaders } from '@/shared/loader'
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
/**
 * The `(game version, loader)` the next pin applies to — a file belongs to one release line, so
 * pinning always names its context instead of forcing one build on every instance.
 */
const selectedVersion = ref<string>('')
const selectedLoader = ref<string>('')

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

/** Rules stored on the item; the legacy single pin is mirrored by the backend. */
const currentRules = computed<PresetPinRule[]>(() => currentItem.value?.pinRules ?? [])

const contextGameVersions = computed(() => (selectedVersion.value ? [selectedVersion.value] : []))
const contextLoaders = computed(() =>
  props.item?.kind === 'mod' && selectedLoader.value ? [selectedLoader.value] : [],
)
/** Without a version or loader on either side the pin would be a wildcard, which the UI never
 * writes: an unpinned item already means "latest compatible file". */
const hasContext = computed(
  () => contextGameVersions.value.length > 0 || contextLoaders.value.length > 0,
)

function sameContext(rule: PresetPinRule): boolean {
  return (
    rule.gameVersions.join(',') === contextGameVersions.value.join(',') &&
    rule.loaders.join(',') === contextLoaders.value.join(',')
  )
}

const ruleForContext = computed<PresetPinRule | null>(
  () => currentRules.value.find((rule) => sameContext(rule)) ?? null,
)

function isPinnedForContext(file: CompFile): boolean {
  return ruleForContext.value?.fileId === file.id
}

async function loadFiles(): Promise<void> {
  const item = props.item
  const preset = props.preset
  if (!item) {
    files.value = []
    return
  }
  loading.value = true
  try {
    const targetMcVersion = selectedVersion.value || (preset && preset.gameVersions[0]) || null
    const targetLoader =
      item.kind === 'mod' ? selectedLoader.value || (preset && preset.loaders[0]) || null : null

    let list = await ipcClient.compFiles(
      item.platform,
      item.projectId,
      targetMcVersion,
      targetLoader,
    )

    // A preset only carries resources it was published for: keep the picker honest.
    if (preset && preset.gameVersions.length > 0) {
      list = list.filter((file) => file.mcVersions.some((v) => preset.gameVersions.includes(v)))
    }
    if (preset && preset.loaders.length > 0 && item.kind === 'mod') {
      const allowed = preset.loaders.map((loader) => loader.toLowerCase())
      list = list.filter((file) =>
        file.loaders.some((loader) => allowed.includes(loader.toLowerCase())),
      )
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
    if (!open) return
    const preset = props.preset
    selectedVersion.value = preset?.gameVersions[0] ?? ''
    selectedLoader.value = preset?.loaders[0] ?? ''
    void loadFiles()
  },
  { immediate: true },
)

watch([selectedVersion, selectedLoader], () => {
  if (props.open) void loadFiles()
})

const availableVersions = computed(() => {
  const preset = props.preset
  if (preset && preset.gameVersions.length > 0) {
    return [...preset.gameVersions].sort(compareVersionsDesc)
  }
  const set = new Set<string>()
  for (const file of files.value) {
    for (const version of file.mcVersions) {
      if (version) set.add(version)
    }
  }
  return Array.from(set).sort(compareVersionsDesc)
})

const availableLoaders = computed(() => {
  const preset = props.preset
  if (preset && preset.loaders.length > 0 && props.item?.kind === 'mod') {
    return [...preset.loaders].sort(compareLoaders)
  }
  const set = new Set<string>()
  for (const file of files.value) {
    for (const loader of file.loaders) {
      if (loader) set.add(loader)
    }
  }
  return Array.from(set).sort(compareLoaders)
})

const filteredFiles = computed(() =>
  files.value.filter((file) => {
    if (selectedVersion.value && !file.mcVersions.includes(selectedVersion.value)) {
      return false
    }
    if (
      selectedLoader.value &&
      !file.loaders.some((loader) => loader.toLowerCase() === selectedLoader.value.toLowerCase())
    ) {
      return false
    }
    return true
  }),
)

function fileSize(bytes: number): string {
  if (bytes >= 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
  return `${(bytes / 1024).toFixed(1)} KB`
}

/** Pins the file for the selected context; clicking the file already pinned there unpins it. */
async function togglePin(file: CompFile): Promise<void> {
  const preset = props.preset
  const item = props.item
  if (!preset || !item) return
  if (!hasContext.value) {
    toast.info(t('presets.pin.contextHint'))
    return
  }
  try {
    if (isPinnedForContext(file)) {
      await store.removePinRule(preset.id, item.platform, item.projectId, file.id)
      toast.success(t('presets.pin.ruleRemoved'))
    } else {
      await store.pinRule(preset.id, item.platform, item.projectId, {
        gameVersions: [...contextGameVersions.value],
        loaders: [...contextLoaders.value],
        fileId: file.id,
        fileName: file.fileName,
      })
      toast.success(t('presets.pin.ruleAdded'))
    }
  } catch (error) {
    toast.danger(formatError(error))
  }
}

async function removeRule(rule: PresetPinRule): Promise<void> {
  const preset = props.preset
  const item = props.item
  if (!preset || !item) return
  try {
    await store.removePinRule(preset.id, item.platform, item.projectId, rule.fileId)
    toast.success(t('presets.pin.ruleRemoved'))
  } catch (error) {
    toast.danger(formatError(error))
  }
}

/** Clears every rule at once, which the backend treats as "follow the latest file". */
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
      <!-- Configured rules -->
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
            :disabled="currentRules.length === 0"
            data-testid="follow-latest-btn"
            @click="followLatest"
          >
            {{ t('presets.pin.followLatest') }}
          </MButton>
        </div>

        <div
          v-if="currentRules.length > 0"
          class="space-y-1.5 max-h-36 overflow-y-auto pr-1"
          data-testid="pin-rule-list"
        >
          <div
            v-for="rule in currentRules"
            :key="`${rule.gameVersions.join(',')}|${rule.loaders.join(',')}|${rule.fileId}`"
            class="flex items-center justify-between gap-2 p-2 rounded-lg bg-panel border border-line text-xs"
            data-testid="pin-rule-row"
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
              :data-testid="`delete-rule-${rule.fileId}`"
              @click="removeRule(rule)"
            />
          </div>
        </div>

        <p
          v-else
          class="text-[11px] text-fg-muted leading-relaxed flex items-start gap-1.5"
          data-testid="pin-following-latest"
        >
          <span class="i-lucide-arrow-down-to-line w-3.5 h-3.5 text-primary shrink-0 mt-0.5" />
          <span>{{ t('presets.item.followingLatest') }}</span>
        </p>

        <p class="text-[11px] text-fg-muted leading-relaxed flex items-start gap-1.5">
          <span class="i-lucide-info w-3.5 h-3.5 text-primary shrink-0 mt-0.5" />
          <span>{{ t('presets.pin.rulesHint') }}</span>
        </p>
      </div>

      <!-- Pin context + file picker -->
      <div class="flex flex-col gap-2.5 flex-1 min-h-0">
        <div class="text-xs font-semibold text-fg-muted flex items-center gap-1.5 shrink-0">
          <span class="i-lucide-plus-circle w-3.5 h-3.5" />
          <span>{{ t('presets.pin.addRuleTitle') }}</span>
        </div>

        <div class="p-2.5 rounded-xl bg-sunken/30 border border-line space-y-2 shrink-0 text-xs">
          <!-- Game version of the pin -->
          <div class="flex items-start gap-1.5">
            <span class="font-semibold text-fg-muted shrink-0 pt-0.5">
              {{ t('presets.pin.filterVersion') }}:
            </span>
            <div
              class="flex flex-wrap items-center gap-1 flex-1 max-h-16 overflow-y-auto pr-1"
              data-testid="pin-context-versions"
            >
              <button
                v-for="version in availableVersions"
                :key="version"
                type="button"
                class="px-2 py-0.5 rounded-md transition font-mono cursor-pointer"
                :class="
                  selectedVersion === version
                    ? 'bg-primary text-white shadow-xs font-semibold'
                    : 'text-fg-muted hover:text-fg hover:bg-sunken'
                "
                @click="selectedVersion = selectedVersion === version ? '' : version"
              >
                {{ version }}
              </button>
            </div>
          </div>

          <!-- Loader of the pin (mods only) -->
          <div
            v-if="item?.kind === 'mod' && availableLoaders.length > 0"
            class="flex items-center gap-1.5 pt-1.5 border-t border-line/40"
          >
            <span class="font-semibold text-fg-muted shrink-0">
              {{ t('presets.pin.filterLoader') }}:
            </span>
            <div class="flex flex-wrap items-center gap-1 flex-1" data-testid="pin-context-loaders">
              <button
                v-for="loader in availableLoaders"
                :key="loader"
                type="button"
                class="px-2 py-0.5 rounded-md transition font-medium cursor-pointer capitalize"
                :class="
                  selectedLoader === loader
                    ? 'bg-primary text-white shadow-xs font-semibold'
                    : 'text-fg-muted hover:text-fg hover:bg-sunken'
                "
                @click="selectedLoader = selectedLoader === loader ? '' : loader"
              >
                {{ loader }}
              </button>
            </div>
          </div>
        </div>

        <p
          v-if="!hasContext"
          class="text-[11px] text-fg-subtle flex items-start gap-1.5"
          data-testid="pin-context-hint"
        >
          <span class="i-lucide-circle-help w-3.5 h-3.5 shrink-0 mt-0.5" />
          <span>{{ t('presets.pin.contextHint') }}</span>
        </p>

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
            class="flex items-center gap-3 py-2 text-left rounded-lg px-2 transition-colors"
            :class="
              hasContext
                ? 'cursor-pointer hover:bg-sunken'
                : 'opacity-60 cursor-not-allowed hover:bg-transparent'
            "
            data-testid="pin-file-row"
            @click="togglePin(file)"
          >
            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-1.5 min-w-0">
                <span class="text-sm text-fg truncate">{{ file.fileName }}</span>
                <MBadge
                  v-if="isPinnedForContext(file)"
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
            <span
              class="w-4 h-4 shrink-0"
              :class="
                isPinnedForContext(file)
                  ? 'i-lucide-pin-off text-primary'
                  : 'i-lucide-pin text-fg-subtle'
              "
            />
          </button>
        </div>
      </div>
    </div>
  </MDrawer>
</template>

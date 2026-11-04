<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { CompFile, CompProject } from '../../../ipc/client'
import { ipcClient } from '../../../ipc/client'
import {
  MCard,
  MButton,
  MIconButton,
  MBadge,
  MEmpty,
  MSkeleton,
  MCollapse,
  MImage,
} from '../../../ui'
import { useFormat } from '../../../composables/useFormat'

export interface FileGroup {
  id: string
  title: string
  files: CompFile[]
  open: boolean
}

const props = defineProps<{
  filesLoading: boolean
  filteredFilesCount: number
  fileGroups: FileGroup[]
  mcVersionFilterOptions: Array<{ label: string; value: string }>
  loaderFilterOptions: Array<{ label: string; value: string }>
  /** False for modpacks (installed whole) and unsupported platforms/types. */
  presetSupported?: boolean
}>()

const selectedMcVersion = defineModel<string>('selectedMcVersion', { default: '' })
const selectedLoader = defineModel<string>('selectedLoader', { default: '' })

const emit = defineEmits<{
  (e: 'download', file: CompFile): void
  (e: 'open-website'): void
  (e: 'add-to-preset', file: CompFile): void
  (e: 'open-dependency', project: CompProject): void
}>()

const filteredVersionOptions = computed(() =>
  props.mcVersionFilterOptions.filter((o) => Boolean(o.value)),
)
const filteredLoaderOptions = computed(() =>
  props.loaderFilterOptions.filter((o) => Boolean(o.value)),
)

const { t } = useI18n()

// Region-aware formatting: culture comes from the language settings.
const { formatDate } = useFormat()

function formatFileSize(bytes: number): string {
  if (bytes >= 1024 * 1024) {
    return (bytes / (1024 * 1024)).toFixed(1) + ' MB'
  }
  return (bytes / 1024).toFixed(1) + ' KB'
}

function getChannelVariant(channel: string): 'primary' | 'warning' | 'neutral' {
  if (channel === 'release') return 'primary'
  if (channel === 'beta') return 'warning'
  return 'neutral'
}
const openState = ref<Record<string, boolean>>({})

function isGroupOpen(id: string, index: number): boolean {
  if (openState.value[id] !== undefined) {
    return openState.value[id]
  }
  return index === 0
}

function toggleGroup(id: string, index: number) {
  openState.value[id] = !isGroupOpen(id, index)
}

// --- Per-file dependencies -----------------------------------------------------------
// Declared dependencies are just ids; the project cards come from one batched lookup per
// file, cached so re-expanding a section never re-queries.
const depOpenState = ref<Record<string, boolean>>({})
const depProjects = ref<Record<string, CompProject[]>>({})
const depLoading = ref<Record<string, boolean>>({})

function dependencyIds(file: CompFile): string[] {
  return Array.from(new Set(file.dependencies.map((dep) => dep.projectId)))
}

async function toggleFileDependencies(file: CompFile) {
  const open = !depOpenState.value[file.id]
  depOpenState.value = { ...depOpenState.value, [file.id]: open }
  if (!open || depProjects.value[file.id] || file.dependencies.length === 0) return

  depLoading.value = { ...depLoading.value, [file.id]: true }
  try {
    const projects = await ipcClient.compDepProjects(file.platform, dependencyIds(file))
    depProjects.value = { ...depProjects.value, [file.id]: projects }
  } catch {
    depProjects.value = { ...depProjects.value, [file.id]: [] }
  } finally {
    depLoading.value = { ...depLoading.value, [file.id]: false }
  }
}

function dependencyProject(fileId: string, projectId: string): CompProject | null {
  return depProjects.value[fileId]?.find((p) => p.id === projectId) ?? null
}

function openDependency(file: CompFile, projectId: string) {
  const project = dependencyProject(file.id, projectId)
  if (project) emit('open-dependency', project)
}
</script>

<template>
  <div class="space-y-4">
    <div class="flex items-center justify-between">
      <h3 class="text-lg font-bold">{{ t('comp.project.files') }}</h3>
    </div>

    <!-- Filters (PCL Style Horizontal Chips) -->
    <div class="space-y-2.5 p-3.5 rounded-2xl bg-sunken/30 border border-line">
      <!-- Instance / MC Version Filter Chips -->
      <div class="flex flex-wrap items-center gap-1.5 text-xs">
        <span class="font-semibold text-fg-muted shrink-0 mr-1">
          {{ t('comp.quickDownload.filterVersion') }}:
        </span>
        <button
          type="button"
          class="px-2.5 py-0.5 rounded-lg transition font-medium cursor-pointer"
          :class="
            !selectedMcVersion
              ? 'bg-primary text-white shadow-xs'
              : 'text-fg-muted hover:text-fg hover:bg-sunken'
          "
          @click="selectedMcVersion = ''"
        >
          {{ t('comp.search.allMcVersions') }}
        </button>
        <button
          v-for="option in filteredVersionOptions"
          :key="option.value"
          type="button"
          class="px-2 py-0.5 rounded-lg transition cursor-pointer font-mono"
          :class="
            selectedMcVersion === option.value
              ? 'bg-primary text-white shadow-xs font-semibold'
              : 'text-fg-muted hover:text-fg hover:bg-sunken'
          "
          @click="selectedMcVersion = option.value"
        >
          {{ option.label }}
        </button>
      </div>

      <!-- Loader Filter Chips -->
      <div class="flex flex-wrap items-center gap-1.5 text-xs pt-1.5 border-t border-line/40">
        <span class="font-semibold text-fg-muted shrink-0 mr-1">
          {{ t('comp.quickDownload.filterLoader') }}:
        </span>
        <button
          type="button"
          class="px-2.5 py-0.5 rounded-lg transition font-medium cursor-pointer"
          :class="
            !selectedLoader
              ? 'bg-primary text-white shadow-xs'
              : 'text-fg-muted hover:text-fg hover:bg-sunken'
          "
          @click="selectedLoader = ''"
        >
          {{ t('comp.search.allLoaders') }}
        </button>
        <button
          v-for="ldr in filteredLoaderOptions"
          :key="ldr.value"
          type="button"
          class="px-2.5 py-0.5 rounded-lg transition cursor-pointer capitalize"
          :class="
            selectedLoader.toLowerCase() === ldr.value.toLowerCase()
              ? 'bg-primary text-white shadow-xs font-semibold'
              : 'text-fg-muted hover:text-fg hover:bg-sunken'
          "
          @click="selectedLoader = ldr.value"
        >
          {{ ldr.label }}
        </button>
      </div>
    </div>

    <!-- Files List -->
    <div class="space-y-3">
      <div v-if="filesLoading" class="space-y-3 py-1">
        <div v-for="n in 4" :key="n" class="m-card p-4 space-y-3">
          <div class="flex items-center justify-between">
            <div class="flex items-center space-x-2">
              <MSkeleton class="h-5 w-16 rounded-full" />
              <MSkeleton class="h-4 w-40" />
            </div>
            <MSkeleton class="h-3 w-16" />
          </div>
          <div class="flex items-center space-x-1">
            <MSkeleton class="h-5 w-16 rounded-full" />
            <MSkeleton class="h-5 w-16 rounded-full" />
          </div>
          <div class="pt-2 border-t border-line flex items-center justify-end">
            <MSkeleton class="h-8 w-20 rounded-lg" />
          </div>
        </div>
      </div>

      <div v-else-if="filteredFilesCount === 0" class="py-12">
        <MEmpty :title="t('comp.project.noFiles')" />
      </div>

      <div v-else class="space-y-3">
        <MCard
          padding="none"
          v-for="(group, idx) in fileGroups"
          :key="group.id"
          class="overflow-hidden border border-line"
        >
          <button
            type="button"
            class="w-full px-4 py-3 flex items-center justify-between text-left font-semibold text-xs text-fg hover:bg-sunken/40 transition cursor-pointer"
            @click="toggleGroup(group.id, idx)"
          >
            <div class="flex items-center space-x-2">
              <span
                class="i-lucide-chevron-right w-4 h-4 text-fg-muted transition-transform duration-200"
                :class="{ 'rotate-90': isGroupOpen(group.id, idx) }"
              />
              <span>{{ group.title }}</span>
              <MBadge variant="neutral" size="sm">{{ group.files.length }}</MBadge>
            </div>
          </button>
          <MCollapse :open="isGroupOpen(group.id, idx)">
            <div class="px-3 pb-3 pt-0">
              <div class="mt-2.5 p-3 rounded-xl bg-sunken/40 space-y-3">
                <div
                  v-for="file in group.files"
                  :key="file.id"
                  class="p-3 rounded-xl bg-panel border border-line/70 hover:border-line-strong transition flex flex-col gap-2"
                >
                  <!-- Top Row: Channel Badge & Full File Title -->
                  <div class="flex items-start justify-between gap-2.5">
                    <div class="flex items-center gap-2 min-w-0 flex-1">
                      <MBadge :variant="getChannelVariant(file.channel)" size="sm" class="shrink-0">
                        {{ t(`comp.files.channel.${file.channel}`) }}
                      </MBadge>
                      <span
                        class="font-medium text-xs text-fg break-all select-text leading-snug"
                        :title="file.name"
                      >
                        {{ file.name }}
                      </span>
                    </div>
                  </div>

                  <!-- Middle Row: Date, Size, Version & Loader Badges + Actions -->
                  <div
                    class="flex items-center justify-between gap-2 flex-wrap text-xs text-fg-muted"
                  >
                    <div class="flex items-center gap-1.5 flex-wrap select-text">
                      <span>{{ formatDate(file.releaseDate) }}</span>
                      <span>•</span>
                      <span>{{ formatFileSize(file.size) }}</span>
                      <template v-if="file.loaders.length > 0">
                        <span>•</span>
                        <MBadge
                          v-for="l in file.loaders"
                          :key="l"
                          variant="neutral"
                          size="sm"
                          class="capitalize"
                        >
                          {{ l }}
                        </MBadge>
                      </template>
                      <template v-if="file.mcVersions.length > 0">
                        <span>•</span>
                        <MBadge
                          v-for="v in file.mcVersions.slice(0, 2)"
                          :key="v"
                          variant="neutral"
                          size="sm"
                          class="font-mono"
                        >
                          {{ v }}
                        </MBadge>
                        <span
                          v-if="file.mcVersions.length > 2"
                          :title="file.mcVersions.join(', ')"
                          class="inline-flex cursor-help"
                        >
                          <MBadge variant="neutral" size="sm" class="font-mono">
                            +{{ file.mcVersions.length - 2 }}
                          </MBadge>
                        </span>
                      </template>
                    </div>

                    <div class="flex items-center gap-1.5 shrink-0 ml-auto">
                      <div v-if="file.distributionDisabled" class="flex items-center gap-1.5">
                        <span class="text-xs text-danger">{{
                          t('comp.files.distributionDisabled')
                        }}</span>
                        <MButton variant="tonal" size="sm" @click="emit('open-website')">
                          {{ t('comp.files.openWebsite') }}
                        </MButton>
                      </div>
                      <template v-else>
                        <MButton variant="primary" size="sm" @click="emit('download', file)">
                          <span class="i-lucide-download mr-1 w-3.5 h-3.5" />
                          {{ t('comp.files.download') }}
                        </MButton>
                        <MIconButton
                          v-if="presetSupported"
                          icon="i-lucide-package-plus"
                          variant="tonal"
                          size="sm"
                          :tooltip="t('comp.fileDependencies.pinToPreset')"
                          data-testid="pin-file-to-preset"
                          @click="emit('add-to-preset', file)"
                        />
                      </template>
                    </div>
                  </div>

                  <!-- Declared dependencies: one batched project lookup per file. -->
                  <div v-if="file.dependencies.length > 0" class="pt-1.5 border-t border-line/40">
                    <button
                      type="button"
                      class="inline-flex items-center gap-1 text-xs font-medium text-primary hover:underline cursor-pointer"
                      data-testid="file-dependencies-toggle"
                      @click.stop="toggleFileDependencies(file)"
                    >
                      <span class="i-lucide-link w-3.5 h-3.5" />
                      {{ t('comp.fileDependencies.title') }} ({{ file.dependencies.length }})
                      <span
                        class="i-lucide-chevron-down w-3.5 h-3.5 transition-transform"
                        :class="{ 'rotate-180': depOpenState[file.id] }"
                      />
                    </button>

                    <div v-if="depOpenState[file.id]" class="mt-2 space-y-1.5">
                      <div
                        v-if="depLoading[file.id]"
                        class="text-xs text-fg-muted flex items-center gap-1.5"
                      >
                        <span class="i-lucide-loader-circle w-3.5 h-3.5 animate-spin" />
                        {{ t('comp.fileDependencies.loading') }}
                      </div>
                      <template v-else>
                        <button
                          v-for="dep in file.dependencies"
                          :key="dep.projectId + '-' + dep.kind"
                          type="button"
                          class="w-full flex items-center gap-2 p-2 rounded-lg border border-line bg-panel hover:border-line-strong text-left transition"
                          :class="
                            dependencyProject(file.id, dep.projectId)
                              ? 'cursor-pointer'
                              : 'cursor-default'
                          "
                          @click.stop="openDependency(file, dep.projectId)"
                        >
                          <MImage
                            :src="dependencyProject(file.id, dep.projectId)?.iconUrl"
                            fallback-icon="i-lucide-puzzle"
                            class="w-6 h-6 rounded-md shrink-0"
                          />
                          <span class="text-xs text-fg truncate flex-1 min-w-0">
                            {{ dependencyProject(file.id, dep.projectId)?.title || dep.projectId }}
                          </span>
                          <MBadge variant="neutral" size="sm">
                            {{ t(`comp.fileDependencies.kind.${dep.kind}`) }}
                          </MBadge>
                        </button>
                      </template>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </MCollapse>
        </MCard>
      </div>
    </div>
  </div>
</template>

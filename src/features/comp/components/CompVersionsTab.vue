<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { CompFile } from '../../../ipc/client'
import { MCard, MButton, MBadge, MEmpty, MSkeleton, MCollapse } from '../../../ui'
import { useFormat } from '../../../composables/useFormat'

export interface FileGroup {
  id: string
  title: string
  files: CompFile[]
  open: boolean
}

defineProps<{
  filesLoading: boolean
  filteredFilesCount: number
  fileGroups: FileGroup[]
  mcVersionFilterOptions: Array<{ label: string; value: string }>
  loaderFilterOptions: Array<{ label: string; value: string }>
}>()

const selectedMcVersion = defineModel<string>('selectedMcVersion', { default: '' })
const selectedLoader = defineModel<string>('selectedLoader', { default: '' })

const emit = defineEmits<{
  (e: 'download', file: CompFile): void
  (e: 'open-website'): void
}>()

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
          {{ t('comp.files.filterInstance') }}:
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
          {{ t('comp.filter.all') }}
        </button>
        <button
          v-for="option in mcVersionFilterOptions"
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
          {{ t('comp.files.filterLoader') }}:
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
          {{ t('comp.filter.all') }}
        </button>
        <button
          v-for="ldr in loaderFilterOptions"
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
                  class="p-3 rounded-xl bg-panel border border-line/70 hover:border-line-strong transition flex flex-col sm:flex-row sm:items-center justify-between gap-3"
                >
                  <div class="min-w-0 flex-1 space-y-1.5">
                    <div class="flex items-center gap-2">
                      <MBadge :variant="getChannelVariant(file.channel)" size="sm">
                        {{ t(`comp.files.channel.${file.channel}`) }}
                      </MBadge>
                      <span
                        class="font-medium text-sm text-fg truncate select-text"
                        :title="file.name"
                      >
                        {{ file.name }}
                      </span>
                    </div>

                    <div class="text-xs text-fg-muted select-text">
                      {{ formatDate(file.releaseDate) }} • {{ formatFileSize(file.size) }}
                    </div>

                    <div class="flex flex-wrap gap-1 select-text pt-0.5">
                      <MBadge v-for="v in file.mcVersions" :key="v" variant="neutral" size="sm">
                        {{ v }}
                      </MBadge>
                      <MBadge
                        v-for="l in file.loaders"
                        :key="l"
                        variant="neutral"
                        size="sm"
                        class="capitalize"
                      >
                        {{ l }}
                      </MBadge>
                    </div>
                  </div>

                  <div class="shrink-0 flex items-center gap-2 self-end sm:self-center">
                    <div v-if="file.distributionDisabled" class="flex items-center space-x-2">
                      <span class="text-xs text-danger">{{
                        t('comp.files.distributionDisabled')
                      }}</span>
                      <MButton variant="tonal" size="sm" @click="emit('open-website')">
                        {{ t('comp.files.openWebsite') }}
                      </MButton>
                    </div>
                    <MButton v-else variant="primary" size="sm" @click="emit('download', file)">
                      <span class="i-lucide-download mr-1 w-3.5 h-3.5" />
                      {{ t('comp.files.download') }}
                    </MButton>
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

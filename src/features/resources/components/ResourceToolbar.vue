<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { ResourceKind } from '../../../ipc/client'
import { MButton, MSelect, MSearchBar } from '../../../ui'

defineProps<{
  kind: ResourceKind
  searchQuery: string
  activeSort: 'name' | 'fileName' | 'tagCount' | 'createdTime' | 'fileSize'
  sortOptions: Array<{ label: string; value: string }>
  activeFilter: 'all' | 'enabled' | 'disabled' | 'updatable' | 'error' | 'duplicate'
  filterCounts: {
    all: number
    enabled: number
    disabled: number
    updatable: number
    error: number
    duplicate: number
  }
  currentRelPath?: string
}>()

const emit = defineEmits<{
  (e: 'update:searchQuery', val: string): void
  (e: 'update:activeSort', val: 'name' | 'fileName' | 'tagCount' | 'createdTime' | 'fileSize'): void
  (
    e: 'update:activeFilter',
    val: 'all' | 'enabled' | 'disabled' | 'updatable' | 'error' | 'duplicate',
  ): void
  (e: 'addFiles', files: FileList): void
  (e: 'batchUpdate'): void
  (e: 'batchEnableAll'): void
  (e: 'batchDisableAll'): void
  (e: 'openFolder'): void
  (e: 'navigateBack'): void
  (e: 'downloadMore'): void
  (e: 'export', format: 'txt' | 'csv'): void
}>()

const { t } = useI18n()
const fileInputRef = ref<HTMLInputElement | null>(null)

function triggerFileInput() {
  fileInputRef.value?.click()
}

function handleFileInputChange(e: Event) {
  const target = e.target as HTMLInputElement
  if (target.files && target.files.length > 0) {
    emit('addFiles', target.files)
    target.value = ''
  }
}
</script>

<template>
  <div class="space-y-3">
    <!-- Toolbar -->
    <div class="flex flex-wrap gap-2 min-w-0 items-center justify-between">
      <div class="flex flex-wrap items-center gap-2">
        <MButton variant="primary" size="sm" @click="triggerFileInput">
          <span class="i-lucide-plus mr-1 w-4 h-4" />
          {{ t('resources.actions.addFiles') }}
        </MButton>
        <input
          ref="fileInputRef"
          type="file"
          multiple
          class="hidden"
          @change="handleFileInputChange"
        />

        <MButton
          v-if="kind === 'mod' && filterCounts.updatable > 0"
          variant="tonal"
          size="sm"
          @click="emit('batchUpdate')"
        >
          <span class="i-lucide-arrow-up-circle mr-1 w-4 h-4 text-primary" />
          {{ t('resources.actions.updateAll') }} ({{ filterCounts.updatable }})
        </MButton>

        <MButton variant="tonal" size="sm" @click="emit('openFolder')">
          <span class="i-lucide-folder mr-1 w-4 h-4" />
          {{ t('resources.actions.openFolder') }}
        </MButton>

        <MButton
          v-if="kind === 'schematic' && currentRelPath"
          variant="tonal"
          size="sm"
          @click="emit('navigateBack')"
        >
          <span class="i-lucide-corner-up-left mr-1 w-4 h-4" />
          {{ t('resources.schematic.folderBack') }}
        </MButton>

        <MButton variant="tonal" size="sm" @click="emit('downloadMore')">
          <span class="i-lucide-download-cloud mr-1 w-4 h-4" />
          {{ t('resources.downloadMore') }}
        </MButton>

        <div class="relative group">
          <MButton variant="tonal" size="sm">
            <span class="i-lucide-file-output mr-1 w-4 h-4" />
            {{ t('resources.actions.export') }}
          </MButton>
          <div
            class="absolute top-full left-0 mt-1 hidden group-hover:flex flex-col bg-panel border border-line rounded-lg shadow-lg z-20 py-1 min-w-32"
          >
            <button
              class="px-3 py-1.5 text-xs text-left hover:bg-sunken text-fg transition cursor-pointer"
              @click="emit('export', 'txt')"
            >
              {{ t('resources.actions.exportTxt') }}
            </button>
            <button
              class="px-3 py-1.5 text-xs text-left hover:bg-sunken text-fg transition cursor-pointer"
              @click="emit('export', 'csv')"
            >
              {{ t('resources.actions.exportCsv') }}
            </button>
          </div>
        </div>

        <!-- Batch Quick Actions (PCL parity) -->
        <div v-if="kind === 'mod'" class="relative group">
          <MButton variant="tonal" size="sm">
            <span class="i-lucide-list-checks mr-1 w-4 h-4" />
            {{ t('resources.title') }}
          </MButton>
          <div
            class="absolute top-full left-0 mt-1 hidden group-hover:flex flex-col bg-panel border border-line rounded-lg shadow-lg z-20 py-1 min-w-32"
          >
            <button
              class="px-3 py-1.5 text-xs text-left hover:bg-sunken text-fg transition cursor-pointer"
              @click="emit('batchEnableAll')"
            >
              {{ t('resources.actions.enableAll') }}
            </button>
            <button
              class="px-3 py-1.5 text-xs text-left hover:bg-sunken text-fg transition cursor-pointer"
              @click="emit('batchDisableAll')"
            >
              {{ t('resources.actions.disableAll') }}
            </button>
          </div>
        </div>
      </div>

      <div class="flex items-center gap-2">
        <MSearchBar
          :model-value="searchQuery"
          class="w-full sm:w-60 shrink min-w-36"
          :placeholder="t('resources.searchPlaceholder')"
          size="sm"
          @update:model-value="emit('update:searchQuery', $event)"
        />
        <div class="w-36 shrink-0">
          <MSelect
            :model-value="activeSort"
            :options="sortOptions"
            size="sm"
            @update:model-value="emit('update:activeSort', $event as any)"
          />
        </div>
      </div>
    </div>

    <!-- Filter Chips -->
    <div class="flex flex-wrap items-center gap-1.5 text-xs">
      <button
        class="px-2.5 py-1 rounded-full border transition cursor-pointer"
        :class="
          activeFilter === 'all'
            ? 'bg-primary text-primary-on border-primary font-medium'
            : 'bg-sunken/40 hover:bg-sunken text-fg-muted border-line'
        "
        @click="emit('update:activeFilter', 'all')"
      >
        {{ searchQuery ? t('resources.filter.searchResult') : t('resources.filter.all') }}
        ({{ filterCounts.all }})
      </button>

      <button
        v-if="kind === 'mod' && (filterCounts.enabled > 0 || activeFilter === 'enabled')"
        class="px-2.5 py-1 rounded-full border transition cursor-pointer"
        :class="
          activeFilter === 'enabled'
            ? 'bg-primary text-primary-on border-primary font-medium'
            : 'bg-sunken/40 hover:bg-sunken text-fg-muted border-line'
        "
        @click="emit('update:activeFilter', 'enabled')"
      >
        {{ t('resources.filter.enabled') }} ({{ filterCounts.enabled }})
      </button>

      <button
        v-if="kind === 'mod' && (filterCounts.disabled > 0 || activeFilter === 'disabled')"
        class="px-2.5 py-1 rounded-full border transition cursor-pointer"
        :class="
          activeFilter === 'disabled'
            ? 'bg-primary text-primary-on border-primary font-medium'
            : 'bg-sunken/40 hover:bg-sunken text-fg-muted border-line'
        "
        @click="emit('update:activeFilter', 'disabled')"
      >
        {{ t('resources.filter.disabled') }} ({{ filterCounts.disabled }})
      </button>

      <button
        v-if="kind === 'mod' && (filterCounts.updatable > 0 || activeFilter === 'updatable')"
        class="px-2.5 py-1 rounded-full border transition cursor-pointer"
        :class="
          activeFilter === 'updatable'
            ? 'bg-primary text-primary-on border-primary font-medium'
            : 'bg-sunken/40 hover:bg-sunken text-fg-muted border-line'
        "
        @click="emit('update:activeFilter', 'updatable')"
      >
        {{ t('resources.filter.updatable') }} ({{ filterCounts.updatable }})
      </button>

      <button
        v-if="filterCounts.error > 0 || activeFilter === 'error'"
        class="px-2.5 py-1 rounded-full border transition cursor-pointer"
        :class="
          activeFilter === 'error'
            ? 'bg-danger text-danger-fg border-danger font-medium'
            : 'bg-sunken/40 hover:bg-sunken text-danger border-line'
        "
        @click="emit('update:activeFilter', 'error')"
      >
        {{ t('resources.filter.error') }} ({{ filterCounts.error }})
      </button>

      <button
        v-if="filterCounts.duplicate > 0 || activeFilter === 'duplicate'"
        class="px-2.5 py-1 rounded-full border transition cursor-pointer"
        :class="
          activeFilter === 'duplicate'
            ? 'bg-primary text-primary-on border-primary font-medium'
            : 'bg-sunken/40 hover:bg-sunken text-fg-muted border-line'
        "
        @click="emit('update:activeFilter', 'duplicate')"
      >
        {{ t('resources.filter.duplicate') }} ({{ filterCounts.duplicate }})
      </button>

      <span v-if="kind === 'schematic' && currentRelPath" class="ml-auto text-fg-muted">
        {{ t('resources.schematic.currentFolder', { path: currentRelPath }) }}
      </span>
    </div>
  </div>
</template>

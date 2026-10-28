<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { ResourceKind } from '../../../ipc/client'
import { MButton, MSelect, MSearchBar, MPopover } from '../../../ui'

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
  checkingUpdates?: boolean
  selectedCount?: number
  totalCount?: number
}>()

const emit = defineEmits<{
  (e: 'update:searchQuery', val: string): void
  (e: 'update:activeSort', val: 'name' | 'fileName' | 'tagCount' | 'createdTime' | 'fileSize'): void
  (
    e: 'update:activeFilter',
    val: 'all' | 'enabled' | 'disabled' | 'updatable' | 'error' | 'duplicate',
  ): void
  /** Asks the parent to open a native file picker (a web `<input type=file>`
   *  cannot hand a real filesystem path to the backend). */
  (e: 'addFiles'): void
  (e: 'batchUpdate'): void
  (e: 'batchEnableAll'): void
  (e: 'batchDisableAll'): void
  (e: 'openFolder'): void
  (e: 'navigateBack'): void
  (e: 'downloadMore'): void
  (e: 'export', format: 'txt' | 'csv'): void
  (e: 'checkUpdates'): void
  (e: 'toggleSelectAll'): void
}>()

const { t } = useI18n()

const exportMenuOpen = ref(false)
const batchMenuOpen = ref(false)

function pickExport(format: 'txt' | 'csv') {
  exportMenuOpen.value = false
  emit('export', format)
}

function pickBatch(action: 'batchEnableAll' | 'batchDisableAll') {
  batchMenuOpen.value = false
  if (action === 'batchEnableAll') emit('batchEnableAll')
  else emit('batchDisableAll')
}
</script>

<template>
  <div class="space-y-3">
    <!-- Toolbar -->
    <div class="flex flex-wrap gap-2 min-w-0 items-center justify-between">
      <div class="flex flex-wrap items-center gap-2">
        <!-- Select All Button (PCL Parity) -->
        <MButton
          v-if="totalCount !== undefined && totalCount > 0"
          variant="tonal"
          size="sm"
          :icon="selectedCount === totalCount ? 'i-lucide-square-check' : 'i-lucide-square'"
          @click="emit('toggleSelectAll')"
        >
          {{
            selectedCount === totalCount
              ? t('resources.selection.clear')
              : t('resources.selection.selectAll')
          }}
        </MButton>

        <MButton
          variant="primary"
          size="sm"
          icon="i-lucide-plus"
          data-testid="resources-add-files"
          @click="emit('addFiles')"
        >
          {{ t('resources.actions.addFiles') }}
        </MButton>

        <MButton
          v-if="kind === 'mod' && filterCounts.updatable > 0"
          variant="tonal"
          size="sm"
          @click="emit('batchUpdate')"
        >
          <span class="i-lucide-circle-arrow-up mr-1 w-4 h-4 text-primary" />
          {{ t('resources.actions.updateAll') }} ({{ filterCounts.updatable }})
        </MButton>
        <MButton
          v-if="kind === 'mod'"
          variant="tonal"
          size="sm"
          icon="i-lucide-refresh-cw"
          :loading="checkingUpdates"
          @click="emit('checkUpdates')"
        >
          {{ t('resources.actions.checkUpdates') }}
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
          <span class="i-lucide-cloud-download mr-1 w-4 h-4" />
          {{ t('resources.downloadMore') }}
        </MButton>

        <!-- Collision-aware menu (MPopover flips/shifts inside the viewport). -->
        <MPopover v-model:open="exportMenuOpen" placement="bottom-end">
          <template #trigger>
            <MButton variant="tonal" size="sm" @click="exportMenuOpen = !exportMenuOpen">
              <span class="i-lucide-file-output mr-1 w-4 h-4" />
              {{ t('resources.actions.export') }}
            </MButton>
          </template>
          <div class="flex flex-col min-w-32" data-testid="resource-export-menu">
            <button
              class="px-3 py-1.5 text-xs text-left rounded-lg hover:bg-sunken text-fg transition cursor-pointer"
              @click="pickExport('txt')"
            >
              {{ t('resources.actions.exportTxt') }}
            </button>
            <button
              class="px-3 py-1.5 text-xs text-left rounded-lg hover:bg-sunken text-fg transition cursor-pointer"
              @click="pickExport('csv')"
            >
              {{ t('resources.actions.exportCsv') }}
            </button>
          </div>
        </MPopover>

        <!-- Batch Quick Actions (PCL parity) -->
        <MPopover v-if="kind === 'mod'" v-model:open="batchMenuOpen" placement="bottom-end">
          <template #trigger>
            <MButton variant="tonal" size="sm" @click="batchMenuOpen = !batchMenuOpen">
              <span class="i-lucide-list-checks mr-1 w-4 h-4" />
              {{ t('resources.title') }}
            </MButton>
          </template>
          <div class="flex flex-col min-w-32" data-testid="resource-batch-menu">
            <button
              class="px-3 py-1.5 text-xs text-left rounded-lg hover:bg-sunken text-fg transition cursor-pointer"
              @click="pickBatch('batchEnableAll')"
            >
              {{ t('resources.actions.enableAll') }}
            </button>
            <button
              class="px-3 py-1.5 text-xs text-left rounded-lg hover:bg-sunken text-fg transition cursor-pointer"
              @click="pickBatch('batchDisableAll')"
            >
              {{ t('resources.actions.disableAll') }}
            </button>
          </div>
        </MPopover>
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

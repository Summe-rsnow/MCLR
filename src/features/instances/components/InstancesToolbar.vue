<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { InstanceSort } from '../store'
import { MCard, MSelect, MIconButton, MInput, MSwitch } from '../../../ui'

defineProps<{
  currentFolder: string
  folderOptions: Array<{ label: string; value: string }>
  searchQuery: string
  sortBy: InstanceSort
  sortOptions: Array<{ label: string; value: string }>
  showHidden: boolean
}>()

const emit = defineEmits<{
  (e: 'update:currentFolder', val: string): void
  (e: 'update:searchQuery', val: string): void
  (e: 'update:sortBy', val: InstanceSort): void
  (e: 'update:showHidden', val: boolean): void
  (e: 'manageFolders'): void
}>()
const { t } = useI18n()
</script>

<template>
  <MCard padding="sm" class="flex flex-wrap items-center justify-between gap-4">
    <!-- Folder Select -->
    <div class="flex items-center gap-2">
      <span class="w-4 h-4 i-lucide-folder text-fg-muted shrink-0" />
      <span class="text-xs font-semibold text-fg-muted">
        {{ t('instances.folder.current') }}:
      </span>
      <div class="w-48">
        <MSelect
          :model-value="currentFolder"
          :options="folderOptions"
          size="sm"
          @update:model-value="emit('update:currentFolder', String($event))"
        />
      </div>
      <MIconButton
        icon="i-lucide-folder-cog"
        size="sm"
        :title="t('instances.folders.manage')"
        @click="emit('manageFolders')"
      />
    </div>

    <!-- Search, Sort & Show Hidden -->
    <div class="flex flex-wrap items-center gap-3">
      <div class="w-48 sm:w-60">
        <MInput
          :model-value="searchQuery"
          :placeholder="t('instances.search')"
          icon="i-lucide-search"
          size="sm"
          clearable
          @update:model-value="emit('update:searchQuery', $event)"
        />
      </div>

      <div class="w-36">
        <MSelect
          :model-value="sortBy"
          :options="sortOptions"
          size="sm"
          @update:model-value="emit('update:sortBy', $event as InstanceSort)"
        />
      </div>

      <div class="flex items-center gap-2 pl-2 border-l border-line/50">
        <span class="text-xs text-fg-muted">{{ t('instances.toolbar.showHidden') }}</span>
        <MSwitch
          :model-value="showHidden"
          size="sm"
          @update:model-value="emit('update:showHidden', $event)"
        />
      </div>
    </div>
  </MCard>
</template>

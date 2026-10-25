<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { convertFileSrc } from '@tauri-apps/api/core'
import type { ResourceEntry, ResourceKind } from '../../../ipc/client'
import { MCheckbox, MBadge, MIconButton, MSwitch } from '../../../ui'

defineProps<{
  entry: ResourceEntry
  kind: ResourceKind
  selected: boolean
}>()

const emit = defineEmits<{
  (e: 'select', path: string): void
  (e: 'navigateDir', entry: ResourceEntry): void
  (e: 'inspectSchematic', entry: ResourceEntry): void
  (e: 'jarInJar', entry: ResourceEntry): void
  (e: 'update', entry: ResourceEntry): void
  (e: 'delete', entry: ResourceEntry, permanent: boolean): void
  (e: 'toggle', entry: ResourceEntry): void
}>()

const { t } = useI18n()

function formatBytes(bytes: number): string {
  if (bytes <= 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(1024))
  return `${(bytes / Math.pow(1024, i)).toFixed(1)} ${units[i]}`
}
</script>

<template>
  <div
    class="flex items-center justify-between p-3 rounded-xl border transition group"
    :class="[
      selected ? 'bg-primary/10 border-primary' : 'bg-panel hover:bg-sunken/60 border-line',
      !entry.enabled && kind === 'mod' ? 'opacity-70' : '',
    ]"
  >
    <!-- Left: Checkbox, Icon, Title, Tags, Description -->
    <div class="flex items-center space-x-3 min-w-0 flex-1 mr-3">
      <MCheckbox :model-value="selected" @update:model-value="emit('select', entry.path)" />

      <!-- Resource Icon / Thumbnail -->
      <div
        class="w-10 h-10 rounded-lg overflow-hidden bg-sunken shrink-0 flex items-center justify-center border border-line"
      >
        <img
          v-if="entry.icon"
          :src="convertFileSrc(entry.icon)"
          class="w-full h-full object-cover"
          loading="lazy"
          decoding="async"
          alt="pack-icon"
        />
        <span v-else-if="entry.isDir" class="i-lucide-folder text-primary w-5 h-5" />
        <span v-else-if="kind === 'mod'" class="i-lucide-puzzle text-primary w-5 h-5" />
        <span v-else-if="kind === 'resourcePack'" class="i-lucide-box text-primary w-5 h-5" />
        <span v-else-if="kind === 'shader'" class="i-lucide-sparkles text-primary w-5 h-5" />
        <span v-else class="i-lucide-file-code text-primary w-5 h-5" />
      </div>

      <div
        class="min-w-0 flex-1 cursor-pointer"
        @click="entry.isDir ? emit('navigateDir', entry) : emit('inspectSchematic', entry)"
      >
        <div class="flex items-center space-x-2">
          <span
            class="font-medium text-sm text-fg truncate"
            :class="{ 'line-through': entry.state === 'unavailable' }"
          >
            {{ entry.title || entry.fileName }}
          </span>

          <span v-if="entry.version" class="text-xs text-fg-muted font-mono">
            v{{ entry.version }}
          </span>

          <MBadge v-if="entry.state === 'unavailable'" variant="danger" size="sm">
            {{ t('resources.status.unavailable') }}
          </MBadge>

          <MBadge v-if="entry.updateAvailable" variant="primary" size="sm">
            {{
              t('resources.status.updateAvailable', {
                version: entry.latestFile?.name || '',
              })
            }}
          </MBadge>

          <MBadge v-for="tag in entry.tags" :key="tag" variant="neutral" size="sm">
            {{ tag }}
          </MBadge>
        </div>

        <div class="flex items-center space-x-2 text-xs text-fg-muted truncate mt-0.5">
          <span class="truncate">{{ entry.fileName }}</span>
          <span>•</span>
          <span>{{ formatBytes(entry.size) }}</span>
          <span v-if="entry.description" class="truncate">• {{ entry.description }}</span>
        </div>
      </div>
    </div>

    <!-- Right: Actions & Switch -->
    <div class="flex items-center space-x-2 shrink-0">
      <!-- Jar-in-Jar action for mods -->
      <MIconButton
        v-if="kind === 'mod'"
        icon="i-lucide-package"
        size="sm"
        variant="text"
        :title="t('resources.actions.jarInJar')"
        @click="emit('jarInJar', entry)"
      />

      <!-- Schematic info action -->
      <MIconButton
        v-if="kind === 'schematic' && !entry.isDir"
        icon="i-lucide-info"
        size="sm"
        variant="text"
        :title="t('resources.actions.info')"
        @click="emit('inspectSchematic', entry)"
      />

      <!-- Update arrow button -->
      <MIconButton
        v-if="kind === 'mod' && entry.updateAvailable"
        icon="i-lucide-arrow-up-circle"
        size="sm"
        variant="text"
        class="text-primary hover:bg-primary/10"
        :title="t('resources.actions.update')"
        @click="emit('update', entry)"
      />

      <!-- Delete -->
      <MIconButton
        icon="i-lucide-trash-2"
        size="sm"
        variant="text"
        class="text-danger hover:bg-danger/10"
        :title="t('resources.actions.delete')"
        @click.exact="emit('delete', entry, false)"
        @click.shift="emit('delete', entry, true)"
      />

      <!-- Toggle Switch for mods -->
      <MSwitch
        v-if="kind === 'mod'"
        :model-value="entry.enabled"
        size="sm"
        @update:model-value="emit('toggle', entry)"
      />
    </div>
  </div>
</template>

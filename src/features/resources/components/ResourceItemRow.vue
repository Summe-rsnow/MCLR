<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { convertFileSrc } from '@tauri-apps/api/core'
import type { ResourceEntry, ResourceKind } from '../../../ipc/client'
import { MBadge, MIconButton, MSwitch, MPopover, MImage } from '../../../ui'

const props = defineProps<{
  entry: ResourceEntry
  kind: ResourceKind
  selected: boolean
}>()

const emit = defineEmits<{
  (e: 'select', path: string): void
  (e: 'navigateDir', entry: ResourceEntry): void
  (e: 'inspectSchematic', entry: ResourceEntry): void
  (e: 'jarInJar', entry: ResourceEntry): void
  (e: 'reveal', entry: ResourceEntry): void
  (e: 'update', entry: ResourceEntry): void
  (e: 'delete', entry: ResourceEntry, permanent: boolean): void
  (e: 'toggle', entry: ResourceEntry): void
}>()

const { t } = useI18n()
const menuOpen = ref(false)

function formatBytes(bytes: number): string {
  if (bytes <= 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(1024))
  return `${(bytes / Math.pow(1024, i)).toFixed(1)} ${units[i]}`
}

function onMenuReveal() {
  menuOpen.value = false
  emit('reveal', props.entry)
}

function onMenuJarInJar() {
  menuOpen.value = false
  emit('jarInJar', props.entry)
}

function onMenuDelete() {
  menuOpen.value = false
  emit('delete', props.entry, false)
}
</script>
<template>
  <div
    class="relative flex items-center justify-between p-3 rounded-xl border transition-all duration-150 group max-w-full overflow-hidden cursor-pointer select-none"
    :class="[
      selected
        ? 'bg-primary-soft/50 border-primary ring-1 ring-primary/30 shadow-sm'
        : 'bg-panel hover:bg-sunken/60 border-line',
      !entry.enabled && kind === 'mod' ? 'opacity-70' : '',
    ]"
    @click="emit('select', entry.path)"
  >
    <!-- Left Active Accent Bar (PCL Parity) -->
    <div
      v-if="selected"
      class="absolute left-0 top-2 bottom-2 w-1 bg-primary rounded-r transition-all duration-200"
    />

    <!-- Left: Icon, Title, Tags, Description -->
    <div class="flex items-center space-x-3 min-w-0 flex-1 mr-3 overflow-hidden">
      <!-- Resource Icon / Thumbnail -->
      <div
        class="w-10 h-10 rounded-lg overflow-hidden bg-sunken shrink-0 flex items-center justify-center border border-line"
      >
        <MImage
          v-if="entry.icon"
          :src="convertFileSrc(entry.icon)"
          alt="pack-icon"
          :fallback-icon="
            entry.isDir
              ? 'i-lucide-folder'
              : kind === 'mod'
                ? 'i-lucide-puzzle'
                : kind === 'resourcePack'
                  ? 'i-lucide-box'
                  : kind === 'shader'
                    ? 'i-lucide-sparkles'
                    : 'i-lucide-file-code'
          "
          class="w-full h-full"
        />
        <span v-else-if="entry.isDir" class="i-lucide-folder text-primary w-5 h-5" />
        <span v-else-if="kind === 'mod'" class="i-lucide-puzzle text-primary w-5 h-5" />
        <span v-else-if="kind === 'resourcePack'" class="i-lucide-box text-primary w-5 h-5" />
        <span v-else-if="kind === 'shader'" class="i-lucide-sparkles text-primary w-5 h-5" />
        <span v-else class="i-lucide-file-code text-primary w-5 h-5" />
      </div>

      <div
        class="min-w-0 flex-1 cursor-pointer overflow-hidden"
        @click.stop="entry.isDir ? emit('navigateDir', entry) : emit('inspectSchematic', entry)"
      >
        <div class="flex items-center space-x-2 min-w-0 overflow-hidden">
          <span
            class="font-medium text-sm text-fg truncate shrink"
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

        <div
          class="flex items-center space-x-2 text-xs text-fg-muted min-w-0 overflow-hidden mt-0.5"
        >
          <span class="truncate shrink max-w-[240px]">{{ entry.fileName }}</span>
          <span class="shrink-0">•</span>
          <span class="shrink-0">{{ formatBytes(entry.size) }}</span>
          <span v-if="entry.description" class="truncate min-w-0 flex-1"
            >• {{ entry.description }}</span
          >
        </div>
      </div>
    </div>

    <!-- Right: Actions & Switch -->
    <div class="flex items-center space-x-1.5 shrink-0" @click.stop>
      <!-- Update arrow button -->
      <MIconButton
        v-if="kind === 'mod' && entry.updateAvailable"
        icon="i-lucide-circle-arrow-up"
        size="sm"
        variant="text"
        class="text-primary hover:bg-primary/10"
        :title="t('resources.actions.update')"
        @click="emit('update', entry)"
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

      <!-- More Menu Popover for items -->
      <MPopover v-model:open="menuOpen" placement="bottom-end">
        <template #trigger>
          <MIconButton
            icon="i-lucide-ellipsis-vertical"
            size="sm"
            variant="text"
            class="text-fg-muted hover:text-fg"
            @click="menuOpen = !menuOpen"
          />
        </template>
        <div class="w-44 py-1 flex flex-col text-xs text-fg space-y-0.5">
          <button
            type="button"
            class="w-full px-3 py-1.5 flex items-center gap-2 hover:bg-sunken rounded-lg text-left transition cursor-pointer"
            @click="onMenuReveal"
          >
            <span class="i-lucide-folder text-primary w-4 h-4 shrink-0" />
            <span class="truncate">{{ t('resources.actions.reveal') }}</span>
          </button>

          <button
            v-if="kind === 'mod'"
            type="button"
            class="w-full px-3 py-1.5 flex items-center gap-2 hover:bg-sunken rounded-lg text-left transition cursor-pointer"
            @click="onMenuJarInJar"
          >
            <span class="i-lucide-package text-primary w-4 h-4 shrink-0" />
            <span class="truncate">{{ t('resources.actions.jarInJar') }}</span>
          </button>

          <div class="h-px bg-line my-1" />

          <button
            type="button"
            class="w-full px-3 py-1.5 flex items-center gap-2 hover:bg-danger/10 text-danger rounded-lg text-left transition cursor-pointer"
            @click="onMenuDelete"
          >
            <span class="i-lucide-trash w-4 h-4 shrink-0" />
            <span class="truncate">{{ t('resources.actions.delete') }}</span>
          </button>
        </div>
      </MPopover>

      <!-- Toggle Switch for mods -->
      <MSwitch
        v-if="kind === 'mod'"
        :model-value="entry.enabled"
        size="sm"
        class="ml-1"
        @update:model-value="emit('toggle', entry)"
      />
    </div>
  </div>
</template>

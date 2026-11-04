<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { convertFileSrc } from '@tauri-apps/api/core'
import type { ResourceEntry, ResourceKind } from '../../../ipc/client'
import { MBadge, MIconButton, MSwitch, MPopover, MImage } from '../../../ui'

const props = defineProps<{
  entry: ResourceEntry
  kind: ResourceKind
  selected: boolean
  /** `settings.comp.showZhName`: use the Chinese mod name in the row. */
  showZhName?: boolean
  /** `settings.comp.localNameStyle`: 0 = Chinese title, 1 = file-name title. */
  localNameStyle?: number
  /** Dependency warning count (missing / mismatched / disabled / conflict). */
  depWarnings?: number
  /** Flashed for two seconds after a quick jump from the dependency drawer. */
  highlighted?: boolean
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
  (e: 'analyze', entry: ResourceEntry): void
}>()

const { t, te } = useI18n()
const menuOpen = ref(false)

/**
 * Primary/secondary labels for local mods.
 *
 * The Chinese name is the friendlier identifier, so it leads by default; users who organize
 * by jar file name flip the order with `localNameStyle`.
 */
const primaryName = computed(() => {
  if (props.kind === 'mod' && props.showZhName && props.entry.zhName) {
    return props.localNameStyle === 1 ? props.entry.fileName : props.entry.zhName
  }
  return props.entry.title || props.entry.fileName
})

const secondaryName = computed(() => {
  if (props.kind === 'mod' && props.showZhName && props.entry.zhName) {
    return props.localNameStyle === 1 ? props.entry.zhName : props.entry.fileName
  }
  return null
})

/**
 * Cached jar artwork is a local file served through the asset protocol; a mod whose jar
 * ships no artwork falls back to its Modrinth project icon, which is a plain URL.
 */
const iconSrc = computed(() => {
  const icon = props.entry.icon
  if (!icon) return null
  return /^https?:\/\//i.test(icon) ? icon : convertFileSrc(icon)
})

const fallbackIcon = computed(() => {
  if (props.entry.isDir) return 'i-lucide-folder'
  switch (props.kind) {
    case 'mod':
      return 'i-lucide-puzzle'
    case 'resourcePack':
      return 'i-lucide-box'
    case 'shader':
      return 'i-lucide-sparkles'
    default:
      return 'i-lucide-file-code'
  }
})

/** PCL parity: category slugs from the project metadata are shown in the UI language. */
function tagLabel(tag: string): string {
  const key = `comp.search.categories.${tag}`
  return te(key) ? t(key) : tag
}

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

function onMenuAnalyze() {
  menuOpen.value = false
  emit('analyze', props.entry)
}

function onMenuDelete() {
  menuOpen.value = false
  emit('delete', props.entry, false)
}
</script>
<template>
  <div
    class="relative flex items-center justify-between p-3 rounded-xl border transition-all duration-150 group max-w-full overflow-hidden cursor-pointer select-none"
    :data-resource-row="entry.path"
    :class="[
      selected
        ? 'bg-primary-soft/50 border-primary ring-1 ring-inset ring-primary/30 shadow-sm'
        : highlighted
          ? 'bg-primary-soft/30 border-primary ring-1 ring-inset ring-primary/50 shadow-sm'
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
      <!--
        Icon tile: the fallback glyph paints immediately and the artwork fades in over it
        once decoded, so a slow jar read or CDN request never blocks or blanks the list.
        The image gets an explicit box — MImage's root is `position: relative`, so an
        `absolute inset-0` class would lose to it and leave the art at its natural size.
      -->
      <div
        class="relative w-10 h-10 rounded-lg overflow-hidden shrink-0 border border-line flex items-center justify-center"
      >
        <MImage
          v-if="iconSrc"
          :src="iconSrc"
          alt=""
          fit="cover"
          :fallback-icon="fallbackIcon"
          class="w-10 h-10"
        />
        <span v-else :class="[fallbackIcon, 'w-5 h-5 text-primary']" />
      </div>

      <div
        class="min-w-0 flex-1 cursor-pointer overflow-hidden"
        @click.stop="entry.isDir ? emit('navigateDir', entry) : emit('inspectSchematic', entry)"
      >
        <div class="flex items-center space-x-2 min-w-0 overflow-hidden">
          <span
            class="font-medium text-sm text-fg truncate shrink"
            :class="{ 'line-through': entry.state === 'unavailable' }"
            data-testid="resource-row-title"
          >
            {{ primaryName }}
          </span>

          <span v-if="entry.version" class="text-xs text-fg-muted font-mono">
            v{{ entry.version }}
          </span>

          <MBadge
            v-if="kind === 'mod' && (depWarnings ?? 0) > 0"
            variant="warning"
            size="sm"
            icon="i-lucide-triangle-alert"
            data-testid="resource-dependency-warning"
          >
            {{ t('resources.dependencyWarning', { count: depWarnings }) }}
          </MBadge>

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
        </div>

        <!--
          PCL parity: categories first (project metadata), then file name · size ·
          description on the same dim line.
        -->
        <div
          class="flex items-center space-x-2 text-xs text-fg-muted min-w-0 overflow-hidden mt-0.5"
        >
          <MBadge v-for="tag in entry.tags" :key="tag" variant="neutral" size="sm">
            {{ tagLabel(tag) }}
          </MBadge>
          <span v-if="entry.tags.length > 0" class="shrink-0">•</span>
          <span class="truncate shrink max-w-[240px]" data-testid="resource-row-subtitle">{{
            secondaryName || entry.fileName
          }}</span>
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

          <button
            v-if="kind === 'mod'"
            type="button"
            class="w-full px-3 py-1.5 flex items-center gap-2 hover:bg-sunken rounded-lg text-left transition cursor-pointer"
            data-testid="resource-analyze-action"
            @click="onMenuAnalyze"
          >
            <span class="i-lucide-git-branch text-primary w-4 h-4 shrink-0" />
            <span class="truncate">{{ t('resources.actions.analyzeDependencies') }}</span>
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

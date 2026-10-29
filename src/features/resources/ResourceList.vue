<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type {
  ResourceEntry,
  ResourceKind,
  ResourcesPatchEvent,
  SchematicInfo,
} from '../../ipc/client'
import { ipcClient } from '../../ipc/client'
import { MButton, MEmpty, MSkeleton, MVirtualList, useToast, useDialog } from '../../ui'
import { useInstancesStore } from '../instances/store'
import ResourceToolbar from './components/ResourceToolbar.vue'
import ResourceItemRow from './components/ResourceItemRow.vue'
import ResourceModals from './components/ResourceModals.vue'
const props = defineProps<{
  instanceId: string
  kind: ResourceKind
}>()

const router = useRouter()
const { t } = useI18n()
const toast = useToast()
const { confirmDanger } = useDialog()
const instancesStore = useInstancesStore()

// State
const checkingUpdates = ref(false)
const initialized = ref(false)
const loading = ref(false)
const entries = ref<ResourceEntry[]>([])
const searchQuery = ref('')
const activeFilter = ref<'all' | 'enabled' | 'disabled' | 'updatable' | 'error' | 'duplicate'>(
  'all',
)
const activeSort = ref<'name' | 'fileName' | 'tagCount' | 'createdTime' | 'fileSize'>('name')
const selectedPaths = ref<Set<string>>(new Set())

// Schematic directory navigation
const currentRelPath = ref('')

// Jar-in-Jar Drawer
const jijDrawerOpen = ref(false)
const selectedJijMod = ref<ResourceEntry | null>(null)

// Schematic info dialog
const schematicDialogOpen = ref(false)
const inspectingSchematic = ref(false)
const schematicDetail = ref<SchematicInfo | null>(null)

// Cascade Prompt Dialog
const cascadeDialogOpen = ref(false)
const cascadeAction = ref<'disable' | 'delete'>('disable')
const cascadeTargetEntry = ref<ResourceEntry | null>(null)
const cascadeDependents = ref<string[]>([])

// Update Confirm Dialog
const updateDialogOpen = ref(false)
const updateTargetEntry = ref<ResourceEntry | null>(null)

// Batch Update Risk Dialog
const batchUpdateRiskDialogOpen = ref(false)
const pendingBatchUpdates = ref<ResourceEntry[]>([])

let unlistenPatch: UnlistenFn | null = null

// Subfolder name mapping for instance_open_path
const folderName = computed(() => {
  switch (props.kind) {
    case 'mod':
      return 'mods'
    case 'resourcePack':
      return 'resourcepacks'
    case 'shader':
      return 'shaderpacks'
    case 'schematic':
      return 'schematics'
  }
})

// Search & filter
const debouncedQuery = ref('')
let debounceTimer: ReturnType<typeof setTimeout> | null = null
watch(searchQuery, (q) => {
  if (debounceTimer) clearTimeout(debounceTimer)
  debounceTimer = setTimeout(() => {
    debouncedQuery.value = q.trim()
  }, 350)
})

function computeScore(entry: ResourceEntry, query: string): number {
  if (!query) return 1
  const q = query.toLowerCase()
  let score = 0
  if (entry.title?.toLowerCase().includes(q)) score += 1.0
  if (entry.fileName.toLowerCase().includes(q)) score += 1.0
  if (entry.description?.toLowerCase().includes(q)) score += 0.4
  if (entry.version?.toLowerCase().includes(q)) score += 0.2
  for (const tag of entry.tags) {
    if (tag.toLowerCase().includes(q)) {
      score += 0.2
      break
    }
  }
  return score
}

// Counts for filter chips
const filterCounts = computed(() => {
  let enabled = 0
  let disabled = 0
  let updatable = 0
  let error = 0

  const nameMap = new Map<string, number>()
  for (const e of entries.value) {
    if (e.enabled) enabled++
    else disabled++
    if (e.updateAvailable) updatable++
    if (e.state === 'unavailable') error++

    const key = e.modId || e.projectId || e.title || e.fileName
    nameMap.set(key, (nameMap.get(key) || 0) + 1)
  }

  let duplicate = 0
  for (const e of entries.value) {
    const key = e.modId || e.projectId || e.title || e.fileName
    if ((nameMap.get(key) || 0) > 1) duplicate++
  }

  return {
    all: entries.value.length,
    enabled,
    disabled,
    updatable,
    error,
    duplicate,
  }
})

const filteredEntries = computed(() => {
  const query = debouncedQuery.value
  let list = entries.value

  // Search filter
  if (query) {
    list = list.filter((e) => computeScore(e, query) >= 0.35)
  }

  // Chip filter
  if (activeFilter.value === 'enabled') {
    list = list.filter((e) => e.enabled)
  } else if (activeFilter.value === 'disabled') {
    list = list.filter((e) => !e.enabled)
  } else if (activeFilter.value === 'updatable') {
    list = list.filter((e) => e.updateAvailable)
  } else if (activeFilter.value === 'error') {
    list = list.filter((e) => e.state === 'unavailable')
  } else if (activeFilter.value === 'duplicate') {
    const counts = new Map<string, number>()
    for (const e of entries.value) {
      const key = e.modId || e.projectId || e.title || e.fileName
      counts.set(key, (counts.get(key) || 0) + 1)
    }
    list = list.filter((e) => {
      const key = e.modId || e.projectId || e.title || e.fileName
      return (counts.get(key) || 0) > 1
    })
  }

  // Sort
  const sortKey = activeSort.value
  return [...list].sort((a, b) => {
    if (a.isDir && !b.isDir) return -1
    if (!a.isDir && b.isDir) return 1
    switch (sortKey) {
      case 'fileName':
        return a.fileName.localeCompare(b.fileName)
      case 'tagCount':
        return b.tags.length - a.tags.length
      case 'createdTime':
        return (b.createdMs || 0) - (a.createdMs || 0)
      case 'fileSize':
        return b.size - a.size
      case 'name':
      default: {
        const nameA = a.title || a.fileName
        const nameB = b.title || b.fileName
        return nameA.localeCompare(nameB)
      }
    }
  })
})

const sortOptions = computed(() => [
  { label: t('resources.sort.name'), value: 'name' },
  { label: t('resources.sort.fileName'), value: 'fileName' },
  { label: t('resources.sort.tagCount'), value: 'tagCount' },
  { label: t('resources.sort.createdTime'), value: 'createdTime' },
  { label: t('resources.sort.fileSize'), value: 'fileSize' },
])

async function loadResources() {
  loading.value = true
  selectedPaths.value.clear()
  const reqId = `${props.instanceId}-${props.kind}-${Date.now()}`

  try {
    entries.value = await ipcClient.resourcesList(
      props.instanceId,
      props.kind,
      currentRelPath.value || null,
      reqId,
    )
  } catch (err: unknown) {
    toast.danger(String(err))
  } finally {
    loading.value = false
    initialized.value = true
  }
}

// Navigation into schematic directory
function handleNavigateDir(entry: ResourceEntry) {
  if (props.kind !== 'schematic' || !entry.isDir) return
  currentRelPath.value = currentRelPath.value
    ? `${currentRelPath.value}/${entry.fileName}`
    : entry.fileName
  loadResources()
}

function handleNavigateBack() {
  if (!currentRelPath.value) return
  const parts = currentRelPath.value.split('/')
  parts.pop()
  currentRelPath.value = parts.join('/')
  loadResources()
}

// Toggle enabled/disabled
async function handleToggle(entry: ResourceEntry) {
  if (props.kind !== 'mod') return

  if (entry.enabled) {
    // Check dependents before disabling
    try {
      const deps = await ipcClient.localModsDependents(props.instanceId, entry.fileName)
      if (deps.length > 0) {
        cascadeTargetEntry.value = entry
        cascadeDependents.value = deps
        cascadeAction.value = 'disable'
        cascadeDialogOpen.value = true
        return
      }
    } catch {
      // Proceed on error
    }
  }

  await doToggle(entry)
}

async function doToggle(entry: ResourceEntry) {
  try {
    const newPath = await ipcClient.resourceToggle(props.instanceId, entry.path)
    if (newPath) {
      entry.path = newPath
      entry.fileName = newPath.split(/[\\/]/).pop() || entry.fileName
    } else if (entry.path.endsWith('.disabled')) {
      entry.path = entry.path.slice(0, -9)
      entry.fileName = entry.path.split(/[\\/]/).pop() || entry.fileName
    } else {
      entry.path = `${entry.path}.disabled`
      entry.fileName = `${entry.fileName}.disabled`
    }
    entry.enabled = !entry.enabled
  } catch (err: unknown) {
    toast.danger(String(err))
  }
}

// Delete
async function handleDelete(entry: ResourceEntry, permanent = false) {
  if (props.kind === 'mod') {
    try {
      const deps = await ipcClient.localModsDependents(props.instanceId, entry.fileName)
      if (deps.length > 0) {
        cascadeTargetEntry.value = entry
        cascadeDependents.value = deps
        cascadeAction.value = 'delete'
        cascadeDialogOpen.value = true
        return
      }
    } catch {
      // Proceed
    }
  }

  const ok = await confirmDanger({
    title: t('resources.delete.confirmTitle'),
    content: permanent
      ? t('resources.delete.confirmPermanentMessage', { name: entry.title || entry.fileName })
      : t('resources.delete.confirmMessage', { name: entry.title || entry.fileName }),
  })
  if (!ok) return

  try {
    await ipcClient.resourcesDelete([entry.path], permanent)
    entries.value = entries.value.filter((e) => e.path !== entry.path)
    selectedPaths.value.delete(entry.path)
    toast.success(t('resources.actions.delete'))
  } catch (err: unknown) {
    toast.danger(String(err))
  }
}

// Single Mod Update
function handleUpdateClick(entry: ResourceEntry) {
  updateTargetEntry.value = entry
  updateDialogOpen.value = true
}

async function doSingleUpdate(entry: ResourceEntry) {
  if (!entry.latestFile) return
  try {
    await ipcClient.localModUpdate(props.instanceId, entry.fileName, entry.latestFile)
    toast.success(t('resources.update.success'))
    updateDialogOpen.value = false
  } catch (err: unknown) {
    toast.danger(String(err))
  }
}

// Batch Update
function handleBatchUpdateClick() {
  const updatables = entries.value.filter((e) => e.updateAvailable && e.latestFile)
  if (updatables.length === 0) return

  const firstEver = !localStorage.getItem('mclr.firstModUpdate')
  if (firstEver || updatables.length >= 15) {
    pendingBatchUpdates.value = updatables
    batchUpdateRiskDialogOpen.value = true
    return
  }

  doBatchUpdate(updatables)
}

async function doBatchUpdate(list: ResourceEntry[]) {
  localStorage.setItem('mclr.firstModUpdate', '1')
  batchUpdateRiskDialogOpen.value = false

  const updates = list
    .filter((e) => !!e.latestFile)
    .map((e) => ({
      currentFileName: e.fileName,
      newFile: e.latestFile!,
    }))

  try {
    await ipcClient.localModsUpdateMany(props.instanceId, updates)
    toast.success(t('resources.update.success'))
  } catch (err: unknown) {
    toast.danger(String(err))
  }
}

function openJarInJar(entry: ResourceEntry) {
  selectedJijMod.value = entry
  jijDrawerOpen.value = true
}

function handleCascadeConfirm() {
  if (cascadeTargetEntry.value) {
    if (cascadeAction.value === 'disable') {
      doToggle(cascadeTargetEntry.value)
    } else {
      handleDelete(cascadeTargetEntry.value)
    }
  }
  cascadeDialogOpen.value = false
}

// Multi-select & Batch Actions
function isSelected(path: string): boolean {
  return selectedPaths.value.has(path)
}

function toggleSelect(path: string) {
  if (selectedPaths.value.has(path)) {
    selectedPaths.value.delete(path)
  } else {
    selectedPaths.value.add(path)
  }
}

function selectAll() {
  if (selectedPaths.value.size === filteredEntries.value.length) {
    selectedPaths.value.clear()
  } else {
    for (const e of filteredEntries.value) {
      selectedPaths.value.add(e.path)
    }
  }
}

async function handleBatchDelete() {
  const count = selectedPaths.value.size
  if (count === 0) return

  const ok = await confirmDanger({
    title: t('resources.delete.confirmTitle'),
    content: t('resources.delete.batchConfirmMessage', { count }),
  })
  if (!ok) return
  try {
    const paths = Array.from(selectedPaths.value)
    await ipcClient.resourcesDelete(paths, false)
    entries.value = entries.value.filter((e) => !selectedPaths.value.has(e.path))
    selectedPaths.value.clear()
    toast.success(t('resources.actions.delete'))
  } catch (err: unknown) {
    toast.danger(String(err))
  }
}

async function handleBatchToggle(enable: boolean) {
  const selectedEntries = entries.value.filter(
    (e) => selectedPaths.value.has(e.path) && e.enabled !== enable,
  )
  for (const entry of selectedEntries) {
    await doToggle(entry)
  }
}

async function handleBatchToggleAll(enable: boolean) {
  const targetEntries = entries.value.filter((e) => e.enabled !== enable)
  if (targetEntries.length === 0) return
  for (const entry of targetEntries) {
    await doToggle(entry)
  }
  toast.success(enable ? t('resources.actions.enableAll') : t('resources.actions.disableAll'))
}

async function handleReveal(entry: ResourceEntry) {
  try {
    await ipcClient.screenshotReveal(entry.path)
  } catch {
    await handleOpenFolder()
  }
}

// Open folder
async function handleOpenFolder() {
  try {
    await ipcClient.instanceOpenPath(props.instanceId, folderName.value)
  } catch (err: unknown) {
    toast.danger(String(err))
  }
}

// Download more
function handleDownloadMore() {
  const compTypeMap: Record<ResourceKind, string> = {
    mod: 'mod',
    resourcePack: 'resourcepack',
    shader: 'shader',
    schematic: 'mod',
  }
  const type = compTypeMap[props.kind]
  router.push(`/comp/${type}?instance=${props.instanceId}`)
}

// Export TXT / CSV
async function handleExport(format: 'txt' | 'csv') {
  try {
    const content = await ipcClient.resourcesExport(filteredEntries.value, format)
    const blob = new Blob([content], {
      type: format === 'csv' ? 'text/csv' : 'text/plain',
    })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `${props.instanceId}-${props.kind}-export.${format}`
    a.click()
    URL.revokeObjectURL(a.href)
    toast.success(t('resources.actions.exportSuccess'))
  } catch (err: unknown) {
    toast.danger(String(err))
  }
}

// Schematic inspect
async function handleSchematicInspect(entry: ResourceEntry) {
  if (props.kind !== 'schematic' || entry.isDir) return
  inspectingSchematic.value = true
  schematicDialogOpen.value = true
  schematicDetail.value = null

  try {
    schematicDetail.value = await ipcClient.schematicInspect(entry.path)
  } catch (err: unknown) {
    toast.danger(String(err))
  } finally {
    inspectingSchematic.value = false
  }
}
/**
 * Imports user-picked files through the native dialog.
 *
 * A browser `<input type="file">` cannot hand the backend a real filesystem
 * path, so the toolbar only emits an intent and the path selection happens here.
 */
async function handleAddFiles() {
  try {
    const dialog = await import('@tauri-apps/plugin-dialog')
    const picked = await dialog.open({
      multiple: true,
      directory: false,
      filters: [{ name: props.kind, extensions: importExtensions(props.kind) }],
    })
    const sources = (Array.isArray(picked) ? picked : picked ? [picked] : []).filter(
      (p): p is string => typeof p === 'string',
    )
    if (sources.length === 0) return
    const result = await ipcClient.resourcesImport(
      props.instanceId,
      props.kind,
      sources,
      'overwrite',
    )
    toast.success(t('resources.actions.addFiles') + ': ' + String(result.imported))
    await loadResources()
  } catch (err) {
    toast.danger(String(err))
  }
}

function importExtensions(resourceKind: string): string[] {
  switch (resourceKind) {
    case 'mod':
    case 'datapack':
      return ['jar', 'zip']
    case 'resourcepack':
    case 'shaderpack':
      return ['zip']
    case 'schematic':
      return ['schematic', 'litematic']
    default:
      return []
  }
}

async function handleCheckUpdates() {
  if (checkingUpdates.value) return
  checkingUpdates.value = true
  try {
    let target = instancesStore.instances.find((i) => i.id === props.instanceId)
    if (!target) {
      await instancesStore.fetchInstances()
      target = instancesStore.instances.find((i) => i.id === props.instanceId)
    }
    const mcVersion = target?.mcVersion || ''
    const loader = target?.loaders?.[0] || target?.kind || 'fabric'

    const updatedMods = await ipcClient.localModsCheckUpdates(props.instanceId, mcVersion, loader)

    let updateCount = 0
    for (const m of updatedMods) {
      const entry = entries.value.find((e) => e.fileName === m.fileName || e.path === m.path)
      if (entry && m.updateFile) {
        entry.updateAvailable = true
        entry.latestFile = m.updateFile as unknown as ResourceEntry['latestFile']
        updateCount++
      }
    }

    if (updateCount > 0) {
      toast.success(t('resources.update.checkSuccess', { count: updateCount }))
    } else {
      toast.info(t('resources.status.noUpdates'))
    }
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : String(err)
    toast.danger(msg)
  } finally {
    checkingUpdates.value = false
  }
}

// File input / drag & drop

// Keyboard shortcuts (Ctrl+A)
function handleKeyDown(ev: KeyboardEvent) {
  if ((ev.ctrlKey || ev.metaKey) && ev.key === 'a') {
    const active = document.activeElement
    if (active && (active.tagName === 'INPUT' || active.tagName === 'TEXTAREA')) {
      return
    }
    ev.preventDefault()
    selectAll()
  }
}

onMounted(async () => {
  window.addEventListener('keydown', handleKeyDown)
  await loadResources()

  // Listen for background streaming patches
  try {
    unlistenPatch = await listen<ResourcesPatchEvent>('resources://patch', (event) => {
      if (event.payload.instanceId !== props.instanceId) return
      for (const patch of event.payload.patches) {
        const idx = entries.value.findIndex((e) => e.fileName === patch.fileName)
        if (idx !== -1) {
          const cur = entries.value[idx]
          if (cur) {
            if (patch.title !== undefined) cur.title = patch.title
            if (patch.description !== undefined) cur.description = patch.description
            if (patch.version !== undefined) cur.version = patch.version
            if (patch.tags !== undefined && patch.tags !== null) cur.tags = patch.tags
            if (patch.modId !== undefined) cur.modId = patch.modId
            if (patch.projectId !== undefined) cur.projectId = patch.projectId
            if (patch.icon !== undefined) cur.icon = patch.icon
            if (patch.latestFile !== undefined) cur.latestFile = patch.latestFile
            if (patch.updateAvailable !== undefined && patch.updateAvailable !== null) {
              cur.updateAvailable = patch.updateAvailable
            }
            if (patch.state !== undefined && patch.state !== null) cur.state = patch.state
          }
        }
      }
    })
  } catch {
    // Ignore in non-Tauri test environments
  }
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', handleKeyDown)
  if (unlistenPatch) unlistenPatch()
})

watch(
  () => props.instanceId,
  () => {
    currentRelPath.value = ''
    loadResources()
  },
)
</script>

<template>
  <div class="relative flex flex-col gap-3 min-w-0">
    <!-- Toolbar -->
    <ResourceToolbar
      :kind="kind"
      :search-query="searchQuery"
      :active-sort="activeSort"
      :sort-options="sortOptions"
      :active-filter="activeFilter"
      :filter-counts="filterCounts"
      :current-rel-path="currentRelPath"
      :checking-updates="checkingUpdates"
      :selected-count="selectedPaths.size"
      :total-count="filteredEntries.length"
      @update:search-query="searchQuery = $event"
      @update:active-sort="activeSort = $event"
      @update:active-filter="activeFilter = $event"
      @batch-update="handleBatchUpdateClick"
      @batch-enable-all="handleBatchToggleAll(true)"
      @batch-disable-all="handleBatchToggleAll(false)"
      @open-folder="handleOpenFolder"
      @navigate-back="handleNavigateBack"
      @download-more="handleDownloadMore"
      @export="handleExport"
      @check-updates="handleCheckUpdates"
      @toggle-select-all="selectAll"
      @add-files="handleAddFiles"
    />
    <!-- Batch selection actions (inline, below the select-all row) -->
    <Transition name="m-selection-bar">
      <div v-if="selectedPaths.size > 0" class="min-w-0 overflow-hidden">
        <div
          class="rounded-xl border border-primary/25 bg-primary-soft/40 px-3 py-2 flex flex-wrap items-center justify-between gap-3"
        >
          <div class="flex items-center gap-3 text-sm min-w-0">
            <span class="font-semibold text-primary-soft-fg truncate">
              {{ t('resources.selection.selected', { count: selectedPaths.size }) }}
            </span>
          </div>

          <div class="flex flex-wrap items-center gap-2">
            <MButton
              v-if="kind === 'mod'"
              variant="tonal"
              size="sm"
              icon="i-lucide-check"
              @click="handleBatchToggle(true)"
            >
              {{ t('resources.actions.enable') }}
            </MButton>
            <MButton
              v-if="kind === 'mod'"
              variant="tonal"
              size="sm"
              icon="i-lucide-x"
              @click="handleBatchToggle(false)"
            >
              {{ t('resources.actions.disable') }}
            </MButton>
            <MButton variant="danger" size="sm" icon="i-lucide-trash" @click="handleBatchDelete">
              {{ t('resources.actions.delete') }}
            </MButton>
            <MButton variant="text" size="sm" @click="selectedPaths.clear()">
              {{ t('resources.actions.cancel') }}
            </MButton>
          </div>
        </div>
      </div>
    </Transition>
    <!-- Content / Skeletons / List -->
    <div class="min-w-0">
      <div v-if="!initialized && loading" class="space-y-2">
        <div
          v-for="n in 5"
          :key="n"
          class="h-16 flex items-center justify-between p-3 rounded-xl border border-line bg-panel"
        >
          <div class="flex items-center space-x-3 min-w-0 flex-1 mr-3">
            <MSkeleton width="1.25rem" height="1.25rem" class="rounded shrink-0" />
            <MSkeleton width="2.5rem" height="2.5rem" class="rounded-lg shrink-0" />
            <div class="space-y-1.5 min-w-0 flex-1">
              <div class="flex items-center gap-2">
                <MSkeleton :width="n % 2 === 0 ? '11rem' : '8rem'" height="1rem" />
                <MSkeleton width="3rem" height="1rem" class="rounded-md" />
              </div>
              <div class="flex items-center gap-3">
                <MSkeleton width="4.5rem" height="0.75rem" />
                <MSkeleton width="6rem" height="0.75rem" />
              </div>
            </div>
          </div>
          <div class="flex items-center space-x-2 shrink-0">
            <MSkeleton width="2rem" height="2rem" class="rounded-lg" />
            <MSkeleton width="2.25rem" height="1.25rem" class="rounded-full" />
          </div>
        </div>
      </div>

      <div v-else-if="filteredEntries.length === 0" class="py-12">
        <MEmpty :title="searchQuery ? t('resources.empty.search') : t(`resources.empty.${kind}`)" />
      </div>

      <div v-else class="space-y-2">
        <MVirtualList
          :items="filteredEntries"
          item-key="path"
          :estimate-size="64"
          :gap="10"
          :threshold="60"
        >
          <template #item="{ item: entry }">
            <ResourceItemRow
              :entry="entry"
              :kind="kind"
              :selected="isSelected(entry.path)"
              @select="toggleSelect"
              @navigate-dir="handleNavigateDir"
              @inspect-schematic="handleSchematicInspect"
              @jar-in-jar="openJarInJar"
              @reveal="handleReveal"
              @update="handleUpdateClick"
              @delete="handleDelete"
              @toggle="handleToggle"
            />
          </template>
        </MVirtualList>
      </div>
    </div>

    <!-- Modals -->
    <ResourceModals
      :instance-id="instanceId"
      :jij-open="jijDrawerOpen"
      :jij-mod="selectedJijMod"
      :schematic-open="schematicDialogOpen"
      :inspecting-schematic="inspectingSchematic"
      :schematic-detail="schematicDetail"
      :cascade-open="cascadeDialogOpen"
      :cascade-action="cascadeAction"
      :cascade-dependents="cascadeDependents"
      :cascade-target="cascadeTargetEntry"
      :update-open="updateDialogOpen"
      :update-target="updateTargetEntry"
      :batch-update-risk-open="batchUpdateRiskDialogOpen"
      :pending-batch-updates="pendingBatchUpdates"
      @update:jij-open="jijDrawerOpen = $event"
      @update:schematic-open="schematicDialogOpen = $event"
      @update:cascade-open="cascadeDialogOpen = $event"
      @update:update-open="updateDialogOpen = $event"
      @update:batch-update-risk-open="batchUpdateRiskDialogOpen = $event"
      @cascade-confirm="handleCascadeConfirm"
      @single-update="doSingleUpdate"
      @batch-update="doBatchUpdate"
    />
  </div>
</template>

<style scoped>
/* Inline batch-selection row: height + fade + slight lift, 200 ms */
.m-selection-bar-enter-active,
.m-selection-bar-leave-active {
  transition:
    max-height var(--dur-base) var(--ease),
    opacity var(--dur-base) var(--ease),
    transform var(--dur-base) var(--ease),
    margin-top var(--dur-base) var(--ease);
  overflow: hidden;
}

.m-selection-bar-enter-from,
.m-selection-bar-leave-to {
  max-height: 0;
  opacity: 0;
  transform: translateY(-8px);
  margin-top: -0.75rem;
}

.m-selection-bar-enter-to,
.m-selection-bar-leave-from {
  max-height: 120px;
  opacity: 1;
  transform: translateY(0);
  margin-top: 0;
}

html[data-motion='reduced'] .m-selection-bar-enter-from,
html[data-motion='reduced'] .m-selection-bar-leave-to {
  transform: none;
}
</style>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useInstancesStore } from './store'
import type { InstanceInfo } from '../../ipc/client'
import {
  MInput,
  MIconButton,
  MSelect,
  MSegmented,
  MCard,
  MBadge,
  MDialog,
  MContextMenu,
  MEmpty,
  useDialog,
  type ContextMenuItem,
} from '../../ui'

const { t } = useI18n()
const instancesStore = useInstancesStore()
const { confirm } = useDialog()

onMounted(async () => {
  await instancesStore.loadSettings()
  await instancesStore.fetchInstances()
})

// Folder options for MSelect
const folderOptions = computed(() =>
  instancesStore.folders.map((f) => ({
    label: f.name,
    value: f.path,
  })),
)

// Filter tabs
const filterOptions = computed(() => [
  { label: t('instances.filter.all'), value: 'all' },
  { label: t('instances.filter.pinned'), value: 'pinned' },
  { label: t('instances.filter.vanilla'), value: 'vanilla' },
  { label: t('instances.filter.modded'), value: 'modded' },
])

// Add Folder Dialog State
const showAddFolderDialog = ref(false)
const newFolderName = ref('')
const newFolderPath = ref('')
const folderPathError = ref('')

function openAddFolderDialog() {
  newFolderName.value = ''
  newFolderPath.value = ''
  folderPathError.value = ''
  showAddFolderDialog.value = true
}

async function handleAddFolder() {
  if (!newFolderPath.value.trim()) {
    folderPathError.value = t('instances.folder.pathError')
    return
  }
  const name = newFolderName.value.trim() || newFolderPath.value.split(/[\\/]/).pop() || 'Folder'
  await instancesStore.addFolder(name, newFolderPath.value.trim())
  showAddFolderDialog.value = false
}

// Rename Dialog State
const showRenameDialog = ref(false)
const targetInstance = ref<InstanceInfo | null>(null)
const instanceNewName = ref('')
const instanceNameError = ref('')

const invalidNameRegex = /[\\/:*?"<>|!;]/

function openRenameDialog(inst: InstanceInfo) {
  targetInstance.value = inst
  instanceNewName.value = inst.name
  instanceNameError.value = ''
  showRenameDialog.value = true
}

async function handleRename() {
  const name = instanceNewName.value.trim()
  if (!name || invalidNameRegex.test(name)) {
    instanceNameError.value = t('instances.dialog.nameError')
    return
  }
  if (targetInstance.value) {
    await instancesStore.rename(targetInstance.value.id, name)
  }
  showRenameDialog.value = false
}

// Duplicate Dialog State
const showDuplicateDialog = ref(false)

function openDuplicateDialog(inst: InstanceInfo) {
  targetInstance.value = inst
  instanceNewName.value = `${inst.name} (Copy)`
  instanceNameError.value = ''
  showDuplicateDialog.value = true
}

async function handleDuplicate() {
  const name = instanceNewName.value.trim()
  if (!name || invalidNameRegex.test(name)) {
    instanceNameError.value = t('instances.dialog.nameError')
    return
  }
  if (targetInstance.value) {
    await instancesStore.duplicate(targetInstance.value.id, name)
  }
  showDuplicateDialog.value = false
}

// Delete instance with confirmation
async function handleDelete(inst: InstanceInfo) {
  const ok = await confirm({
    title: t('instances.actions.deleteConfirmTitle'),
    content: t('instances.actions.deleteConfirmMessage', { name: inst.name }),
    confirmText: t('common.delete'),
    cancelText: t('common.cancel'),
  })
  if (ok) {
    await instancesStore.remove(inst.id)
  }
}

const contextMenuOpen = ref(false)
const contextMenuX = ref(0)
const contextMenuY = ref(0)
const activeContextItems = ref<ContextMenuItem[]>([])

function onContextMenu(e: MouseEvent, inst: InstanceInfo) {
  e.preventDefault()
  contextMenuX.value = e.clientX
  contextMenuY.value = e.clientY
  activeContextItems.value = [
    {
      key: 'pin',
      label: inst.config?.pinned ? t('instances.card.unpin') : t('instances.card.pin'),
      icon: inst.config?.pinned ? 'i-lucide-pin-off' : 'i-lucide-pin',
    },
    {
      key: 'openFolder',
      label: t('instances.actions.openFolder'),
      icon: 'i-lucide-folder-open',
    },
    {
      key: 'rename',
      label: t('instances.actions.rename'),
      icon: 'i-lucide-edit-2',
    },
    {
      key: 'duplicate',
      label: t('instances.actions.duplicate'),
      icon: 'i-lucide-copy',
    },
    {
      key: 'delete',
      label: t('instances.actions.delete'),
      icon: 'i-lucide-trash-2',
      danger: true,
    },
  ]
  targetInstance.value = inst
  contextMenuOpen.value = true
}

function handleContextMenuSelect(key: string) {
  if (!targetInstance.value) return
  const inst = targetInstance.value
  switch (key) {
    case 'pin':
      instancesStore.togglePin(inst.id)
      break
    case 'openFolder':
      instancesStore.openFolder(inst.id)
      break
    case 'rename':
      openRenameDialog(inst)
      break
    case 'duplicate':
      openDuplicateDialog(inst)
      break
    case 'delete':
      handleDelete(inst)
      break
  }
}

function getLoaderBadgeVariant(kind: string): 'primary' | 'warning' | 'neutral' {
  switch (kind) {
    case 'fabric':
      return 'primary'
    case 'forge':
    case 'neoforge':
      return 'warning'
    case 'cleanroom':
    case 'quilt':
    default:
      return 'neutral'
  }
}
</script>

<template>
  <div class="h-full flex flex-col p-6 max-w-6xl mx-auto overflow-y-auto space-y-6">
    <!-- Top Bar: Folder Switcher & Actions -->
    <div class="flex flex-wrap items-center justify-between gap-4">
      <!-- Folder Select -->
      <div class="flex items-center gap-2">
        <span class="w-4 h-4 i-lucide-folder text-textMuted shrink-0" />
        <span class="text-xs font-semibold text-textMuted">{{ t('instances.folder.current') }}:</span>
        <div class="w-48">
          <MSelect
            :model-value="instancesStore.currentFolder"
            :options="folderOptions"
            size="sm"
            @update:model-value="(val) => instancesStore.switchFolder(String(val))"
          />
        </div>
        <MIconButton
          icon="i-lucide-folder-plus"
          size="sm"
          :title="t('instances.folder.add')"
          @click="openAddFolderDialog"
        />
      </div>

      <!-- Search & Filters -->
      <div class="flex flex-wrap items-center gap-3">
        <div class="w-48 sm:w-64">
          <MInput
            v-model="instancesStore.searchQuery"
            :placeholder="t('instances.search')"
            icon="i-lucide-search"
            size="sm"
            clearable
          />
        </div>

        <MSegmented
          v-model="instancesStore.filterType"
          :options="filterOptions"
          size="sm"
        />

        <MIconButton
          icon="i-lucide-rotate-cw"
          size="sm"
          :title="t('instances.refresh')"
          :loading="instancesStore.loading"
          @click="instancesStore.fetchInstances"
        />
      </div>
    </div>

    <!-- Empty State -->
    <div v-if="instancesStore.filteredInstances.length === 0" class="py-20">
      <MEmpty
        :title="instancesStore.instances.length === 0 ? t('instances.emptyFolder') : t('instances.empty')"
        :description="instancesStore.instances.length === 0 ? t('instances.emptyFolderSubtitle') : ''"
      />
    </div>

    <!-- Instance Cards Container -->
    <div v-else class="space-y-6">
      <!-- Pinned Section -->
      <div v-if="instancesStore.groupedInstances.pinned.length > 0" class="space-y-3">
        <div class="flex items-center gap-2 text-xs font-bold text-textMuted uppercase tracking-wider">
          <span class="w-3.5 h-3.5 i-lucide-pin text-primary" />
          <span>{{ t('instances.filter.pinned') }}</span>
          <span class="text-[10px] text-textMuted/70">({{ instancesStore.groupedInstances.pinned.length }})</span>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          <template v-for="inst in instancesStore.groupedInstances.pinned" :key="inst.id">
            <MCard
              class="relative p-4 flex flex-col justify-between h-36 hover:border-primary/50 transition-all duration-200 cursor-pointer group shadow-sm"
              @contextmenu="onContextMenu($event, inst)"
            >
              <!-- Top Card Info -->
              <div class="flex items-start justify-between gap-2">
                <div class="flex items-center gap-3 min-w-0">
                  <div class="w-10 h-10 rounded-lg bg-surfaceVariant/60 flex items-center justify-center shrink-0 border border-outline/30 text-primary">
                    <span
                      class="w-6 h-6"
                      :class="{
                        'i-lucide-box': inst.kind === 'vanilla',
                        'i-lucide-layers': inst.kind === 'fabric',
                        'i-lucide-flame': inst.kind === 'forge' || inst.kind === 'neoforge',
                        'i-lucide-cpu': inst.kind !== 'vanilla' && inst.kind !== 'fabric' && inst.kind !== 'forge' && inst.kind !== 'neoforge',
                      }"
                    />
                  </div>
                  <div class="min-w-0">
                    <div class="font-bold text-sm text-text truncate group-hover:text-primary transition-colors">
                      {{ inst.name }}
                    </div>
                    <div class="text-xs text-textMuted font-mono">
                      {{ inst.mcVersion }}
                    </div>
                  </div>
                </div>

                <div class="flex items-center gap-1">
                  <button
                    type="button"
                    class="text-primary hover:text-primary/80 transition-colors p-1"
                    :title="t('instances.card.unpin')"
                    @click.stop="instancesStore.togglePin(inst.id)"
                  >
                    <span class="w-4 h-4 i-lucide-pin" />
                  </button>
                </div>
              </div>

              <!-- Bottom Badges & Status -->
              <div class="flex items-center justify-between gap-2 pt-2 border-t border-outline/20">
                <div class="flex flex-wrap items-center gap-1.5 overflow-hidden">
                  <MBadge v-if="inst.state !== 'ok'" variant="danger">
                    {{ t(`instances.card.state.${inst.state}`) }}
                  </MBadge>
                  <MBadge
                    v-for="loader in inst.loaders"
                    :key="loader"
                    :variant="getLoaderBadgeVariant(inst.kind)"
                  >
                    {{ loader }}
                  </MBadge>
                  <MBadge v-if="inst.loaders.length === 0" variant="neutral">
                    Vanilla
                  </MBadge>
                </div>

                <div class="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                  <MIconButton
                    icon="i-lucide-folder-open"
                    size="sm"
                    :title="t('instances.actions.openFolder')"
                    @click.stop="instancesStore.openFolder(inst.id)"
                  />
                  <MIconButton
                    icon="i-lucide-edit-2"
                    size="sm"
                    :title="t('instances.actions.rename')"
                    @click.stop="openRenameDialog(inst)"
                  />
                </div>
              </div>
            </MCard>
          </template>
        </div>
      </div>

      <!-- Categories Sections -->
      <template v-for="cat in instancesStore.groupedInstances.categories" :key="cat.name">
        <div v-if="cat.items.length > 0" class="space-y-3">
          <div class="flex items-center gap-2 text-xs font-bold text-textMuted uppercase tracking-wider">
            <span class="w-3.5 h-3.5 i-lucide-tag" />
            <span>{{ cat.name }}</span>
            <span class="text-[10px] text-textMuted/70">({{ cat.items.length }})</span>
          </div>

          <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          <template v-for="inst in cat.items" :key="inst.id">
            <MCard
              class="relative p-4 flex flex-col justify-between h-36 hover:border-primary/50 transition-all duration-200 cursor-pointer group shadow-sm"
              @contextmenu="onContextMenu($event, inst)"
            >
              <!-- Top Card Info -->
              <div class="flex items-start justify-between gap-2">
                <div class="flex items-center gap-3 min-w-0">
                  <div class="w-10 h-10 rounded-lg bg-surfaceVariant/60 flex items-center justify-center shrink-0 border border-outline/30 text-primary">
                    <span
                      class="w-6 h-6"
                      :class="{
                        'i-lucide-box': inst.kind === 'vanilla',
                        'i-lucide-layers': inst.kind === 'fabric',
                        'i-lucide-flame': inst.kind === 'forge' || inst.kind === 'neoforge',
                        'i-lucide-cpu': inst.kind !== 'vanilla' && inst.kind !== 'fabric' && inst.kind !== 'forge' && inst.kind !== 'neoforge',
                      }"
                    />
                  </div>
                  <div class="min-w-0">
                    <div class="font-bold text-sm text-text truncate group-hover:text-primary transition-colors">
                      {{ inst.name }}
                    </div>
                    <div class="text-xs text-textMuted font-mono">
                      {{ inst.mcVersion }}
                    </div>
                  </div>
                </div>

                <div class="flex items-center gap-1">
                  <button
                    type="button"
                    class="text-textMuted/40 hover:text-primary transition-colors p-1"
                    :title="t('instances.card.pin')"
                    @click.stop="instancesStore.togglePin(inst.id)"
                  >
                    <span class="w-4 h-4 i-lucide-pin" />
                  </button>
                </div>
              </div>

              <!-- Bottom Badges & Status -->
              <div class="flex items-center justify-between gap-2 pt-2 border-t border-outline/20">
                <div class="flex flex-wrap items-center gap-1.5 overflow-hidden">
                  <MBadge v-if="inst.state !== 'ok'" variant="danger">
                    {{ t(`instances.card.state.${inst.state}`) }}
                  </MBadge>
                  <MBadge
                    v-for="loader in inst.loaders"
                    :key="loader"
                    :variant="getLoaderBadgeVariant(inst.kind)"
                  >
                    {{ loader }}
                  </MBadge>
                  <MBadge v-if="inst.loaders.length === 0" variant="neutral">
                    Vanilla
                  </MBadge>
                </div>

                <div class="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                  <MIconButton
                    icon="i-lucide-folder-open"
                    size="sm"
                    :title="t('instances.actions.openFolder')"
                    @click.stop="instancesStore.openFolder(inst.id)"
                  />
                  <MIconButton
                    icon="i-lucide-edit-2"
                    size="sm"
                    :title="t('instances.actions.rename')"
                    @click.stop="openRenameDialog(inst)"
                  />
                </div>
              </div>
            </MCard>
          </template>
          </div>
        </div>
      </template>
    </div>

    <!-- Rename Dialog -->
    <MDialog
      :open="showRenameDialog"
      :title="t('instances.dialog.renameTitle')"
      @confirm="handleRename"
      @cancel="showRenameDialog = false"
    >
      <div class="space-y-3 py-2">
        <MInput
          v-model="instanceNewName"
          :label="t('instances.dialog.nameLabel')"
          :placeholder="t('instances.dialog.namePlaceholder')"
          :error="instanceNameError"
          autofocus
        />
      </div>
    </MDialog>

    <!-- Duplicate Dialog -->
    <MDialog
      :open="showDuplicateDialog"
      :title="t('instances.dialog.duplicateTitle')"
      @confirm="handleDuplicate"
      @cancel="showDuplicateDialog = false"
    >
      <div class="space-y-3 py-2">
        <MInput
          v-model="instanceNewName"
          :label="t('instances.dialog.nameLabel')"
          :placeholder="t('instances.dialog.namePlaceholder')"
          :error="instanceNameError"
          autofocus
        />
      </div>
    </MDialog>

    <!-- Add Folder Dialog -->
    <MDialog
      :open="showAddFolderDialog"
      :title="t('instances.folder.addDialogTitle')"
      @confirm="handleAddFolder"
      @cancel="showAddFolderDialog = false"
    >
      <div class="space-y-4 py-2">
        <MInput
          v-model="newFolderName"
          :label="t('instances.folder.nameLabel')"
          :placeholder="t('instances.folder.namePlaceholder')"
        />
        <MInput
          v-model="newFolderPath"
          :label="t('instances.folder.pathLabel')"
          :placeholder="t('instances.folder.pathPlaceholder')"
          :error="folderPathError"
        />
      </div>
    </MDialog>
    <!-- Context Menu Component -->
    <MContextMenu
      :items="activeContextItems"
      :open="contextMenuOpen"
      :x="contextMenuX"
      :y="contextMenuY"
      @update:model-value="(val) => (contextMenuOpen = val)"
      @select="handleContextMenuSelect"
    />
  </div>
</template>

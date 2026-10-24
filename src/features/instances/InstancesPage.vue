<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { useInstancesStore } from './store'
import type { InstanceInfo } from '../../ipc/client'
import {
  MPageHeader,
  MInput,
  MButton,
  MIconButton,
  MSelect,
  MSegmented,
  MCard,
  MBadge,
  MDialog,
  MContextMenu,
  MEmpty,
  MLoaderIcon,
  useDialog,
  type ContextMenuItem,
} from '../../ui'
import ModpackImportDialog from '../comp/ModpackImportDialog.vue'

const showImportModpackDialog = ref(false)
const { t } = useI18n()
const router = useRouter()
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
      key: 'manage',
      label: t('instances.actions.manage'),
      icon: 'i-lucide-settings-2',
    },
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
    case 'manage':
      navigateToDetail(inst.id)
      break
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

function navigateToDetail(id: string) {
  router.push(`/instances/${encodeURIComponent(id)}`)
}
</script>

<template>
  <div class="m-page">
    <!-- Header -->
    <MPageHeader
      :title="t('instances.nav')"
      icon="i-lucide-boxes"
    >
      <template #actions>
        <div class="flex items-center gap-2">
          <MButton
            variant="primary"
            size="sm"
            icon="i-lucide-plus"
            @click="router.push('/download')"
          >
            {{ t('instances.actions.create') }}
          </MButton>
          <MButton
            variant="tonal"
            size="sm"
            icon="i-lucide-file-up"
            @click="showImportModpackDialog = true"
          >
            {{ t('instances.actions.importModpack') }}
          </MButton>
          <MIconButton
            icon="i-lucide-rotate-cw"
            size="sm"
            :title="t('instances.refresh')"
            :loading="instancesStore.loading"
            @click="instancesStore.fetchInstances"
          />
        </div>
      </template>
    </MPageHeader>

    <!-- Toolbar: Folder, Search, Category -->
    <MCard padding="sm" class="flex flex-wrap items-center justify-between gap-4">
      <!-- Folder Select -->
      <div class="flex items-center gap-2">
        <span class="w-4 h-4 i-lucide-folder text-fg-muted shrink-0" />
        <span class="text-xs font-semibold text-fg-muted">{{ t('instances.folder.current') }}:</span>
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
      </div>
    </MCard>

    <!-- Empty State -->
    <div v-if="instancesStore.filteredInstances.length === 0" class="py-16">
      <MEmpty
        :title="instancesStore.instances.length === 0 ? t('instances.emptyFolder') : t('instances.empty')"
        :description="instancesStore.instances.length === 0 ? t('instances.emptyFolderSubtitle') : ''"
        icon="i-lucide-boxes"
      >
        <MButton
          variant="primary"
          size="sm"
          icon="i-lucide-plus"
          @click="router.push('/download')"
        >
          {{ t('instances.actions.create') }}
        </MButton>
      </MEmpty>
    </div>

    <!-- Instance Cards Container -->
    <div v-else class="space-y-6">
      <!-- Pinned Section -->
      <div v-if="instancesStore.groupedInstances.pinned.length > 0" class="space-y-3">
        <div class="flex items-center gap-2 text-xs font-bold text-fg-muted uppercase tracking-wider">
          <span class="w-3.5 h-3.5 i-lucide-pin text-primary" />
          <span>{{ t('instances.filter.pinned') }}</span>
          <span class="text-[10px] text-fg-muted/70">({{ instancesStore.groupedInstances.pinned.length }})</span>
        </div>

        <div class="grid grid-cols-[repeat(auto-fill,minmax(260px,1fr))] gap-4">
          <template v-for="inst in instancesStore.groupedInstances.pinned" :key="inst.id">
            <MCard
              hoverable
              padding="sm"
              class="relative flex flex-col justify-between h-36 cursor-pointer group"
              @click="navigateToDetail(inst.id)"
              @contextmenu="onContextMenu($event, inst)"
            >
              <!-- Top Card Info -->
              <div class="flex items-start justify-between gap-2">
                <div class="flex items-center gap-3 min-w-0">
                  <MLoaderIcon
                    :kind="inst.kind || (inst.loaders?.[0]?.toLowerCase() ?? 'vanilla')"
                    :size="40"
                    class="shrink-0"
                  />
                  <div class="min-w-0">
                    <div class="font-bold text-sm text-fg truncate group-hover:text-primary transition-colors">
                      {{ inst.name }}
                    </div>
                    <div class="text-xs text-fg-muted font-mono">
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
              <div class="flex items-center justify-between gap-2 pt-2 border-t border-line">
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
                    icon="i-lucide-settings-2"
                    size="sm"
                    :title="t('instances.actions.manage')"
                    @click.stop="navigateToDetail(inst.id)"
                  />
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
          <div class="flex items-center gap-2 text-xs font-bold text-fg-muted uppercase tracking-wider">
            <span class="w-3.5 h-3.5 i-lucide-tag" />
            <span>{{ cat.name }}</span>
            <span class="text-[10px] text-fg-muted/70">({{ cat.items.length }})</span>
          </div>

          <div class="grid grid-cols-[repeat(auto-fill,minmax(260px,1fr))] gap-4">
            <template v-for="inst in cat.items" :key="inst.id">
              <MCard
                hoverable
                padding="sm"
                class="relative flex flex-col justify-between h-36 cursor-pointer group"
                @click="navigateToDetail(inst.id)"
                @contextmenu="onContextMenu($event, inst)"
              >
                <!-- Top Card Info -->
                <div class="flex items-start justify-between gap-2">
                  <div class="flex items-center gap-3 min-w-0">
                    <MLoaderIcon
                      :kind="inst.kind || (inst.loaders?.[0]?.toLowerCase() ?? 'vanilla')"
                      :size="40"
                      class="shrink-0"
                    />
                    <div class="min-w-0">
                      <div class="font-bold text-sm text-fg truncate group-hover:text-primary transition-colors">
                        {{ inst.name }}
                      </div>
                      <div class="text-xs text-fg-muted font-mono">
                        {{ inst.mcVersion }}
                      </div>
                    </div>
                  </div>

                  <div class="flex items-center gap-1">
                    <button
                      type="button"
                      class="text-fg-muted/40 hover:text-primary transition-colors p-1"
                      :title="t('instances.card.pin')"
                      @click.stop="instancesStore.togglePin(inst.id)"
                    >
                      <span class="w-4 h-4 i-lucide-pin" />
                    </button>
                  </div>
                </div>

                <!-- Bottom Badges & Status -->
                <div class="flex items-center justify-between gap-2 pt-2 border-t border-line">
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
                      icon="i-lucide-settings-2"
                      size="sm"
                      :title="t('instances.actions.manage')"
                      @click.stop="navigateToDetail(inst.id)"
                    />
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
      @update:open="showRenameDialog = $event"
    >
      <div class="py-2 space-y-4">
        <MInput
          v-model="instanceNewName"
          :label="t('instances.dialog.nameLabel')"
          :placeholder="t('instances.dialog.namePlaceholder')"
          :error="instanceNameError"
          size="sm"
          autofocus
          @keyup.enter="handleRename"
        />
      </div>
      <template #footer>
        <div class="flex items-center justify-end gap-2">
          <MButton variant="text" size="sm" @click="showRenameDialog = false">
            {{ t('common.cancel') }}
          </MButton>
          <MButton variant="primary" size="sm" @click="handleRename">
            {{ t('common.save') }}
          </MButton>
        </div>
      </template>
    </MDialog>

    <!-- Duplicate Dialog -->
    <MDialog
      :open="showDuplicateDialog"
      :title="t('instances.dialog.duplicateTitle')"
      @update:open="showDuplicateDialog = $event"
    >
      <div class="py-2 space-y-4">
        <MInput
          v-model="instanceNewName"
          :label="t('instances.dialog.nameLabel')"
          :placeholder="t('instances.dialog.namePlaceholder')"
          :error="instanceNameError"
          size="sm"
          autofocus
          @keyup.enter="handleDuplicate"
        />
      </div>
      <template #footer>
        <div class="flex items-center justify-end gap-2">
          <MButton variant="text" size="sm" @click="showDuplicateDialog = false">
            {{ t('common.cancel') }}
          </MButton>
          <MButton variant="primary" size="sm" @click="handleDuplicate">
            {{ t('common.save') }}
          </MButton>
        </div>
      </template>
    </MDialog>

    <!-- Add Folder Dialog -->
    <MDialog
      :open="showAddFolderDialog"
      :title="t('instances.folder.addDialogTitle')"
      @update:open="showAddFolderDialog = $event"
    >
      <div class="py-2 space-y-4">
        <MInput
          v-model="newFolderName"
          :label="t('instances.folder.nameLabel')"
          :placeholder="t('instances.folder.namePlaceholder')"
          size="sm"
        />
        <MInput
          v-model="newFolderPath"
          :label="t('instances.folder.pathLabel')"
          :placeholder="t('instances.folder.pathPlaceholder')"
          :error="folderPathError"
          size="sm"
        />
      </div>
      <template #footer>
        <div class="flex items-center justify-end gap-2">
          <MButton variant="text" size="sm" @click="showAddFolderDialog = false">
            {{ t('common.cancel') }}
          </MButton>
          <MButton variant="primary" size="sm" @click="handleAddFolder">
            {{ t('common.add') }}
          </MButton>
        </div>
      </template>
    </MDialog>

    <!-- Context Menu Component -->
    <MContextMenu
      :open="contextMenuOpen"
      :x="contextMenuX"
      :y="contextMenuY"
      :items="activeContextItems"
      @update:open="contextMenuOpen = $event"
      @select="handleContextMenuSelect"
    />

    <!-- Modpack Import Dialog -->
    <ModpackImportDialog
      :open="showImportModpackDialog"
      @update:open="showImportModpackDialog = $event"
      @imported="instancesStore.fetchInstances"
    />
  </div>
</template>

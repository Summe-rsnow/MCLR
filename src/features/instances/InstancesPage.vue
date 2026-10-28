<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { useInstancesStore } from './store'
import { ipcClient, type InstanceInfo, type CardKind } from '../../ipc/client'
import { open as openFileDialog } from '@tauri-apps/plugin-dialog'
import {
  MPageHeader,
  MButton,
  MIconButton,
  MCard,
  MEmpty,
  MSkeleton,
  MCollapse,
  useDialog,
  useToast,
} from '../../ui'
import ModpackImportDialog from '../comp/ModpackImportDialog.vue'
import GameFoldersDialog from './GameFoldersDialog.vue'
import InstancesToolbar from './components/InstancesToolbar.vue'
import InstanceCard from './components/InstanceCard.vue'
import InstanceContextMenu from './components/InstanceContextMenu.vue'
import { useLaunchStore } from '../launch/store'
import InstanceActionDialogs from './components/InstanceActionDialogs.vue'

const toast = useToast()
const showFolders = ref(false)

const showImportModpackDialog = ref(false)
const { t } = useI18n()
const router = useRouter()
const instancesStore = useInstancesStore()
const launchStore = useLaunchStore()
const { confirmDanger } = useDialog()

onMounted(async () => {
  await instancesStore.loadSettings()
  await instancesStore.fetchInstances()
})

const folderOptions = computed(() =>
  instancesStore.folders.map((f) => ({
    label: f.name,
    value: f.path,
  })),
)

const sortOptions = computed(() => [
  { label: `${t('instances.toolbar.sort')}: ${t('instances.toolbar.sortName')}`, value: 'name' },
  {
    label: `${t('instances.toolbar.sort')}: ${t('instances.toolbar.sortVersion')}`,
    value: 'version',
  },
  {
    label: `${t('instances.toolbar.sort')}: ${t('instances.toolbar.sortCategory')}`,
    value: 'category',
  },
])

// Collapse state for card groups (default collapsed: rubbish, error, fool)
const collapsedGroups = ref<Record<string, boolean>>({
  rubbish: true,
  error: true,
  fool: true,
})

function isGroupCollapsed(key: CardKind): boolean {
  if (instancesStore.cardGroups.length <= 1) {
    return false
  }
  return !!collapsedGroups.value[key]
}

function toggleGroupCollapse(key: CardKind) {
  collapsedGroups.value[key] = !isGroupCollapsed(key)
}

async function onPickAndAddFolder() {
  try {
    const selected = await openFileDialog({ directory: true })
    if (!selected) return
    const path = Array.isArray(selected) ? selected[0] : selected
    if (!path) return
    try {
      await ipcClient.assetsAllow(path)
    } catch {
      // ignore
    }
    const parts = path.replace(/\\/g, '/').split('/').filter(Boolean)
    const name = parts[parts.length - 1] || '.minecraft'
    await instancesStore.addFolder(name, path)
  } catch (err) {
    toast.danger(String(err))
  }
}
// Dialog State
const showRenameDialog = ref(false)
const showDuplicateDialog = ref(false)
const targetInstance = ref<InstanceInfo | null>(null)

function openRenameDialog(inst: InstanceInfo) {
  targetInstance.value = inst
  showRenameDialog.value = true
}

function openDuplicateDialog(inst: InstanceInfo) {
  targetInstance.value = inst
  showDuplicateDialog.value = true
}

async function onRename(name: string) {
  if (targetInstance.value) {
    await instancesStore.rename(targetInstance.value.id, name)
  }
}

async function onDuplicate(name: string) {
  if (targetInstance.value) {
    await instancesStore.duplicate(targetInstance.value.id, name)
  }
}

// Delete instance with confirmation
async function handleDelete(inst: InstanceInfo, permanent = false) {
  let message = permanent
    ? t('instances.actions.deletePermanentConfirm', { name: inst.name })
    : t('instances.actions.deleteConfirmMessage', { name: inst.name })

  if (inst.config?.isolation === 'on') {
    message += `\n\n${t('instances.actions.deleteIsolatedWarning')}`
  }

  const ok = await confirmDanger({
    title: permanent
      ? t('instances.actions.deletePermanent')
      : t('instances.actions.deleteConfirmTitle'),
    content: message,
    confirmText: permanent ? t('instances.actions.deletePermanent') : t('common.delete'),
    cancelText: t('common.cancel'),
  })
  if (ok) {
    await instancesStore.remove(inst.id, permanent)
  }
}

// Context Menu State
const contextMenuOpen = ref(false)
const contextMenuX = ref(0)
const contextMenuY = ref(0)
const contextMenuPermanent = ref(false)

function onContextMenu(e: MouseEvent, inst: InstanceInfo) {
  e.preventDefault()
  contextMenuX.value = e.clientX
  contextMenuY.value = e.clientY
  targetInstance.value = inst
  contextMenuPermanent.value = e.shiftKey
  contextMenuOpen.value = true
}

function handleContextMenuSelect(key: string, inst: InstanceInfo) {
  switch (key) {
    case 'manage':
      navigateToDetail(inst.id, 'settings')
      break
    case 'openFolder':
      instancesStore.openFolder(inst.id)
      break
    case 'star':
      instancesStore.toggleStar(inst.id)
      break
    case 'rename':
      openRenameDialog(inst)
      break
    case 'duplicate':
      openDuplicateDialog(inst)
      break
    case 'hide':
      instancesStore.toggleHide(inst.id)
      break
    case 'delete':
      handleDelete(inst, false)
      break
    case 'deletePermanent':
      handleDelete(inst, true)
      break
  }
}

function navigateToDetail(id: string, tab?: string) {
  const query = tab ? `?tab=${tab}` : ''
  router.push(`/instances/${encodeURIComponent(id)}${query}`)
}
</script>

<template>
  <div class="m-page">
    <!-- Header -->
    <MPageHeader :title="t('instances.nav')" icon="i-lucide-boxes">
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
            :loading="instancesStore.loading || instancesStore.refreshing"
            @click="() => instancesStore.fetchInstances()"
          />
        </div>
      </template>
    </MPageHeader>

    <!-- Toolbar: Folder, Search, Sort, ShowHidden -->
    <InstancesToolbar
      :current-folder="instancesStore.currentFolder"
      :folder-options="folderOptions"
      :search-query="instancesStore.searchQuery"
      :sort-by="instancesStore.sortBy"
      :sort-options="sortOptions"
      :show-hidden="instancesStore.showHidden"
      @update:current-folder="instancesStore.switchFolder"
      @update:search-query="instancesStore.searchQuery = $event"
      @update:sort-by="instancesStore.sortBy = $event"
      @update:show-hidden="instancesStore.showHidden = $event"
      @manage-folders="showFolders = true"
    />

    <!-- Guide: No game folder configured -->
    <div v-if="!instancesStore.currentFolder" class="py-12" data-testid="no-folder-guide">
      <MCard padding="lg" class="max-w-md mx-auto text-center flex flex-col items-center gap-4">
        <div
          class="w-16 h-16 rounded-2xl bg-primary/10 text-primary flex items-center justify-center"
        >
          <span class="w-8 h-8 i-lucide-folder-plus" />
        </div>
        <div class="space-y-1">
          <h3 class="text-base font-semibold text-fg">{{ t('instances.noFolder.title') }}</h3>
          <p class="text-sm text-fg-muted">{{ t('instances.noFolder.desc') }}</p>
        </div>
        <MButton
          variant="primary"
          size="md"
          icon="i-lucide-folder-open"
          data-testid="pick-game-folder-btn"
          @click="onPickAndAddFolder"
        >
          {{ t('instances.noFolder.action') }}
        </MButton>
      </MCard>
    </div>

    <!-- Skeleton Loading while initializing -->
    <div
      v-else-if="!instancesStore.initialized"
      class="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-4"
      data-testid="instances-skeleton"
    >
      <MCard v-for="n in 8" :key="n" padding="sm" class="flex flex-col justify-between h-36">
        <div class="flex items-start justify-between gap-2">
          <div class="flex items-center gap-3 min-w-0">
            <MSkeleton width="3rem" height="3rem" class="rounded-xl shrink-0" />
            <div class="space-y-1.5 min-w-0">
              <MSkeleton width="7rem" height="1rem" />
              <MSkeleton width="4rem" height="0.875rem" />
            </div>
          </div>
          <MSkeleton width="1.5rem" height="1.5rem" class="rounded-lg shrink-0" />
        </div>
        <div class="flex items-center gap-2 mt-2">
          <MSkeleton width="4.5rem" height="1.25rem" class="rounded-full" />
        </div>
      </MCard>
    </div>
    <!-- Empty State -->
    <div v-else-if="instancesStore.filteredInstances.length === 0" class="py-16">
      <MEmpty
        :title="
          instancesStore.instances.length === 0
            ? t('instances.empty.title')
            : t('instances.empty.filtered')
        "
        :description="instancesStore.instances.length === 0 ? t('instances.empty.subtitle') : ''"
        icon="i-lucide-boxes"
      >
        <MButton
          v-if="instancesStore.instances.length === 0"
          variant="primary"
          size="sm"
          icon="i-lucide-download"
          @click="router.push('/download')"
        >
          {{ t('instances.empty.action') }}
        </MButton>
      </MEmpty>
    </div>

    <!-- Card Groups -->
    <div v-else class="space-y-6">
      <div v-for="group in instancesStore.cardGroups" :key="group.key" class="space-y-3">
        <!-- Group Header Collapse Trigger -->
        <button
          type="button"
          class="flex items-center gap-2 text-xs font-bold text-fg-muted uppercase tracking-wider hover:text-fg transition-colors cursor-pointer select-none"
          @click="toggleGroupCollapse(group.key)"
        >
          <span
            class="w-3.5 h-3.5 i-lucide-chevron-right transition-transform duration-200"
            :class="{ 'rotate-90': !isGroupCollapsed(group.key) }"
          />
          <span v-if="group.key === 'star'" class="w-3.5 h-3.5 i-lucide-star text-amber-400" />
          <span>{{ t(group.titleKey) }}</span>
          <span class="text-[10px] text-fg-muted/70">({{ group.items.length }})</span>
        </button>

        <!-- Group Content in MCollapse -->
        <MCollapse :open="!isGroupCollapsed(group.key)">
          <TransitionGroup
            name="m-list"
            tag="div"
            class="grid grid-cols-[repeat(auto-fill,minmax(280px,1fr))] gap-4 pt-2"
          >
            <InstanceCard
              v-for="inst in group.items"
              :key="inst.id"
              :instance="inst"
              :last-used="launchStore.lastUsedInstanceId === inst.id"
              @click="navigateToDetail(inst.id)"
              @contextmenu="onContextMenu($event, inst)"
              @toggle-star="instancesStore.toggleStar(inst.id)"
              @quick-action="(target, tab) => navigateToDetail(target.id, tab)"
            />
          </TransitionGroup>
        </MCollapse>
      </div>
    </div>

    <!-- Rename / Duplicate Dialogs -->
    <InstanceActionDialogs
      v-model:rename-open="showRenameDialog"
      v-model:duplicate-open="showDuplicateDialog"
      :instance="targetInstance"
      @rename="onRename"
      @duplicate="onDuplicate"
    />

    <!-- Context Menu Component -->
    <InstanceContextMenu
      v-model:open="contextMenuOpen"
      :instance="targetInstance"
      :permanent="contextMenuPermanent"
      :x="contextMenuX"
      :y="contextMenuY"
      @action="handleContextMenuSelect"
    />

    <!-- Modpack Import Dialog -->
    <ModpackImportDialog
      v-model:open="showImportModpackDialog"
      @imported="() => instancesStore.fetchInstances()"
    />

    <!-- Game Folders Dialog -->
    <GameFoldersDialog v-model:open="showFolders" />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { useInstancesStore } from './store'
import { ipcClient, type InstanceInfo, type CardKind } from '../../ipc/client'
import { open as openFileDialog } from '@tauri-apps/plugin-dialog'
import {
  MPageHeader,
  MInput,
  MButton,
  MIconButton,
  MSelect,
  MSwitch,
  MCard,
  MBadge,
  MDialog,
  MContextMenu,
  MEmpty,
  MSkeletonList,
  MCollapse,
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

const folderOptions = computed(() =>
  instancesStore.folders.map((f) => ({
    label: f.name,
    value: f.path,
  })),
)

const sortOptions = computed(() => [
  { label: `${t('instances.toolbar.sort')}: ${t('instances.toolbar.sortName')}`, value: 'name' },
  { label: `${t('instances.toolbar.sort')}: ${t('instances.toolbar.sortVersion')}`, value: 'version' },
  { label: `${t('instances.toolbar.sort')}: ${t('instances.toolbar.sortCategory')}`, value: 'category' },
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
    console.error('Failed to add folder:', err)
  }
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
  if (!name || invalidNameRegex.test(name) || name.endsWith('.') || name.length > 70) {
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
  if (!name || invalidNameRegex.test(name) || name.endsWith('.') || name.length > 70) {
    instanceNameError.value = t('instances.dialog.nameError')
    return
  }
  if (targetInstance.value) {
    await instancesStore.duplicate(targetInstance.value.id, name)
  }
  showDuplicateDialog.value = false
}

// Delete instance with confirmation
async function handleDelete(inst: InstanceInfo, permanent = false) {
  let message = permanent
    ? t('instances.actions.deletePermanentConfirm', { name: inst.name })
    : t('instances.actions.deleteConfirmMessage', { name: inst.name })

  if (inst.config?.isolation === 'on') {
    message += `\n\n${t('instances.actions.deleteIsolatedWarning')}`
  }

  const ok = await confirm({
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
const activeContextItems = ref<ContextMenuItem[]>([])

function onContextMenu(e: MouseEvent, inst: InstanceInfo) {
  e.preventDefault()
  contextMenuX.value = e.clientX
  contextMenuY.value = e.clientY
  const isStarred = inst.config?.starred || inst.config?.pinned
  const isHidden = inst.card === 'hidden' || inst.config?.cardType === 'hidden'
  const isPermanent = e.shiftKey

  activeContextItems.value = [
    {
      key: 'manage',
      label: t('instances.actions.manage'),
      icon: 'i-lucide-settings-2',
    },
    {
      key: 'openFolder',
      label: t('instances.actions.openFolder'),
      icon: 'i-lucide-folder-open',
    },
    {
      key: 'star',
      label: isStarred ? t('instances.actions.unstar') : t('instances.actions.star'),
      icon: isStarred ? 'i-lucide-star-off' : 'i-lucide-star',
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
      key: 'hide',
      label: isHidden ? t('instances.actions.unhide') : t('instances.actions.hide'),
      icon: isHidden ? 'i-lucide-eye' : 'i-lucide-eye-off',
    },
    {
      key: isPermanent ? 'deletePermanent' : 'delete',
      label: isPermanent
        ? t('instances.actions.deletePermanent')
        : t('instances.actions.delete'),
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

function navigateToDetail(id: string, tab?: string) {
  const query = tab ? `?tab=${tab}` : ''
  router.push(`/instances/${encodeURIComponent(id)}${query}`)
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
            :loading="instancesStore.loading || instancesStore.refreshing"
            @click="() => instancesStore.fetchInstances()"
          />
        </div>
      </template>
    </MPageHeader>

    <!-- Toolbar: Folder, Search, Sort, ShowHidden -->
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
          @click="onPickAndAddFolder"
        />
      </div>

      <!-- Search, Sort & Show Hidden -->
      <div class="flex flex-wrap items-center gap-3">
        <div class="w-48 sm:w-60">
          <MInput
            v-model="instancesStore.searchQuery"
            :placeholder="t('instances.search')"
            icon="i-lucide-search"
            size="sm"
            clearable
          />
        </div>

        <div class="w-36">
          <MSelect
            v-model="instancesStore.sortBy"
            :options="sortOptions"
            size="sm"
          />
        </div>

        <div class="flex items-center gap-2 pl-2 border-l border-line/50">
          <span class="text-xs text-fg-muted">{{ t('instances.toolbar.showHidden') }}</span>
          <MSwitch v-model="instancesStore.showHidden" size="sm" />
        </div>
      </div>
    </MCard>

    <!-- Guide: No game folder configured -->
    <div v-if="!instancesStore.currentFolder" class="py-12" data-testid="no-folder-guide">
      <MCard padding="lg" class="max-w-md mx-auto text-center flex flex-col items-center gap-4">
        <div class="w-16 h-16 rounded-2xl bg-primary/10 text-primary flex items-center justify-center">
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
    <div v-else-if="!instancesStore.initialized" class="py-6">
      <MSkeletonList variant="card" :rows="6" />
    </div>

    <!-- Empty State -->
    <div v-else-if="instancesStore.filteredInstances.length === 0" class="py-16">
      <MEmpty
        :title="instancesStore.instances.length === 0 ? t('instances.empty.title') : t('instances.empty.filtered')"
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
      <div
        v-for="group in instancesStore.cardGroups"
        :key="group.key"
        class="space-y-3"
      >
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
          <span
            v-if="group.key === 'star'"
            class="w-3.5 h-3.5 i-lucide-star text-amber-400"
          />
          <span>{{ t(group.titleKey) }}</span>
          <span class="text-[10px] text-fg-muted/70">({{ group.items.length }})</span>
        </button>

        <!-- Group Content in MCollapse -->
        <MCollapse :open="!isGroupCollapsed(group.key)">
          <TransitionGroup
            name="m-list"
            tag="div"
            class="grid grid-cols-[repeat(auto-fill,minmax(280px,1fr))] gap-4"
          >
            <MCard
              v-for="inst in group.items"
              :key="inst.id"
              hoverable
              padding="sm"
              class="relative flex flex-col justify-between h-40 cursor-pointer group"
              @click="navigateToDetail(inst.id)"
              @contextmenu="onContextMenu($event, inst)"
            >
              <!-- Top Row Info -->
              <div class="flex items-start justify-between gap-2">
                <div class="flex items-center gap-3 min-w-0">
                  <MLoaderIcon
                    :kind="inst.kind || (inst.loaders?.[0]?.toLowerCase() ?? 'vanilla')"
                    :icon="inst.config?.icon"
                    :size="40"
                    class="shrink-0"
                  />
                  <div class="min-w-0">
                    <div class="font-bold text-sm text-fg truncate group-hover:text-primary transition-colors">
                      {{ inst.name }}
                    </div>
                    <div class="text-xs text-fg-muted font-mono truncate">
                      {{ inst.mcVersion }}
                    </div>
                  </div>
                </div>

                <!-- Star / More Options -->
                <div class="flex items-center gap-1">
                  <button
                    type="button"
                    class="p-1 transition-colors rounded-lg hover:bg-sunken"
                    :class="inst.config?.starred || inst.config?.pinned ? 'text-amber-400' : 'text-fg-muted/40 hover:text-fg-muted'"
                    :title="inst.config?.starred || inst.config?.pinned ? t('instances.actions.unstar') : t('instances.actions.star')"
                    @click.stop="instancesStore.toggleStar(inst.id)"
                  >
                    <span
                      class="w-4 h-4"
                      :class="inst.config?.starred || inst.config?.pinned ? 'i-lucide-star fill-current' : 'i-lucide-star'"
                    />
                  </button>

                  <button
                    type="button"
                    class="p-1 text-fg-muted hover:text-fg transition-colors rounded-lg hover:bg-sunken"
                    :title="t('instances.card.more')"
                    @click.stop="onContextMenu($event, inst)"
                  >
                    <span class="w-4 h-4 i-lucide-more-vertical" />
                  </button>
                </div>
              </div>

              <!-- Badges / Loaders Row -->
              <div class="flex flex-wrap items-center gap-1.5 my-2">
                <MBadge
                  variant="primary"
                  size="sm"
                >
                  Minecraft {{ inst.mcVersion }}
                </MBadge>
                <MBadge
                  v-if="inst.loaders && inst.loaders.length > 0"
                  v-for="loader in inst.loaders"
                  :key="loader"
                  :variant="getLoaderBadgeVariant(inst.kind)"
                  size="sm"
                >
                  {{ loader }}
                </MBadge>
                <MBadge
                  v-else-if="inst.kind && inst.kind !== 'vanilla'"
                  variant="neutral"
                  size="sm"
                >
                  {{ inst.kind }}
                </MBadge>
                <MBadge
                  v-if="inst.state !== 'ok'"
                  variant="danger"
                  size="sm"
                >
                  {{ t(`instances.card.state.${inst.state}`) }}
                </MBadge>
                <MBadge
                  v-if="inst.config?.isolation === 'on'"
                  variant="info"
                  size="sm"
                >
                  {{ t('instances.actions.isolationOn') }}
                </MBadge>
              </div>

              <!-- Bottom Row: Quick Access Icon Buttons -->
              <div class="flex items-center justify-between pt-2 border-t border-line/40 text-xs text-fg-muted">
                <span class="truncate max-w-[120px]">{{ inst.folder }}</span>

                <div class="flex items-center gap-1">
                  <!-- Mods quick entry (only if modable) -->
                  <button
                    v-if="inst.modable"
                    type="button"
                    class="p-1 rounded text-fg-muted hover:text-primary hover:bg-primary/10 transition-colors"
                    :title="t('instances.actions.modsQuick')"
                    @click.stop="navigateToDetail(inst.id, 'mods')"
                  >
                    <span class="w-3.5 h-3.5 i-lucide-puzzle" />
                  </button>

                  <!-- Resourcepacks quick entry -->
                  <button
                    type="button"
                    class="p-1 rounded text-fg-muted hover:text-primary hover:bg-primary/10 transition-colors"
                    :title="t('instances.actions.resourcepacksQuick')"
                    @click.stop="navigateToDetail(inst.id, 'resourcepacks')"
                  >
                    <span class="w-3.5 h-3.5 i-lucide-palette" />
                  </button>

                  <!-- Screenshots quick entry -->
                  <button
                    type="button"
                    class="p-1 rounded text-fg-muted hover:text-primary hover:bg-primary/10 transition-colors"
                    :title="t('instances.actions.screenshotsQuick')"
                    @click.stop="navigateToDetail(inst.id, 'screenshots')"
                  >
                    <span class="w-3.5 h-3.5 i-lucide-image" />
                  </button>
                </div>
              </div>
            </MCard>
          </TransitionGroup>
        </MCollapse>
      </div>
    </div>

    <!-- Rename Dialog -->
    <MDialog
      :open="showRenameDialog"
      :title="t('instances.dialog.renameTitle')"
      @update:open="showRenameDialog = $event"
    >
      <div class="space-y-4 py-2">
        <MInput
          v-model="instanceNewName"
          :label="t('instances.dialog.nameLabel')"
          :placeholder="t('instances.dialog.namePlaceholder')"
          :error="instanceNameError"
          @keyup.enter="handleRename"
        />
      </div>
      <template #footer>
        <div class="flex justify-end gap-2">
          <MButton variant="tonal" @click="showRenameDialog = false">
            {{ t('common.cancel') }}
          </MButton>
          <MButton variant="primary" @click="handleRename">
            {{ t('common.confirm') }}
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
      <div class="space-y-4 py-2">
        <MInput
          v-model="instanceNewName"
          :label="t('instances.dialog.nameLabel')"
          :placeholder="t('instances.dialog.namePlaceholder')"
          :error="instanceNameError"
          @keyup.enter="handleDuplicate"
        />
      </div>
      <template #footer>
        <div class="flex justify-end gap-2">
          <MButton variant="tonal" @click="showDuplicateDialog = false">
            {{ t('common.cancel') }}
          </MButton>
          <MButton variant="primary" @click="handleDuplicate">
            {{ t('common.confirm') }}
          </MButton>
        </div>
      </template>
    </MDialog>


    <!-- Context Menu Component -->
    <MContextMenu
      v-model="contextMenuOpen"
      :items="activeContextItems"
      :x="contextMenuX"
      :y="contextMenuY"
      @select="handleContextMenuSelect"
    />

    <!-- Modpack Import Dialog -->
    <ModpackImportDialog
      v-model:open="showImportModpackDialog"
      @imported="() => instancesStore.fetchInstances()"
    />
  </div>
</template>

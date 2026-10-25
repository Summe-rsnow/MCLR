<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useInstancesStore } from './store'
import type { GameFolderEntry } from '../../ipc/client'
import { ipcClient } from '../../ipc/client'
import { open as openFileDialog } from '@tauri-apps/plugin-dialog'
import {
  MDialog,
  MButton,
  MIconButton,
  MInput,
  MBadge,
  MEmpty,
  MHelpTooltip,
  useToast,
  useDialog,
} from '../../ui'
import { useErrorText } from '../../composables/useErrorText'

defineProps<{
  open: boolean
}>()

const emit = defineEmits<{
  (e: 'update:open', val: boolean): void
}>()

const { t } = useI18n()
const toast = useToast()
const { confirm } = useDialog()
const { formatError } = useErrorText()
const instancesStore = useInstancesStore()

const editingFolderPath = ref<string | null>(null)
const editingRemarkName = ref('')

function formatFolderPath(path: string): string {
  const parts = path.replace(/\\/g, '/').split('/').filter(Boolean)
  if (parts.length <= 2) return path
  return `.../${parts.slice(-2).join('/')}`
}

function startEditRemark(folder: GameFolderEntry) {
  editingFolderPath.value = folder.path
  editingRemarkName.value = folder.name
}

function cancelEditRemark() {
  editingFolderPath.value = null
  editingRemarkName.value = ''
}

async function saveRemark(path: string) {
  const name = editingRemarkName.value.trim()
  if (!name) return
  await instancesStore.renameFolder(path, name)
  editingFolderPath.value = null
  editingRemarkName.value = ''
  toast.success(t('common.saved'))
}

async function onPickAndAddGameFolder() {
  try {
    const selected = await openFileDialog({ directory: true })
    if (!selected) return
    const path = Array.isArray(selected) ? selected[0] : selected
    if (!path) return

    if (instancesStore.folders.some((f) => f.path === path)) {
      toast.warning(t('instances.folders.alreadyExists'))
      return
    }

    try {
      await ipcClient.assetsAllow(path)
    } catch {
      // ignore
    }

    const parts = path.replace(/\\/g, '/').split('/').filter(Boolean)
    const name = parts[parts.length - 1] || '.minecraft'
    await instancesStore.addFolder(name, path)
    toast.success(t('common.saved'))
  } catch (err) {
    toast.danger(`${t('common.error')}: ${String(err)}`)
  }
}

async function openFolderInOs(path: string) {
  try {
    await ipcClient.pathOpen(path)
  } catch (err) {
    toast.danger(formatError(err))
  }
}

async function onRemoveFolder(path: string) {
  const ok = await confirm({
    title: t('instances.folders.removeConfirmTitle'),
    content: t('instances.folders.removeConfirmDesc', { path }),
    confirmText: t('common.delete'),
    cancelText: t('common.cancel'),
  })
  if (!ok) return

  await instancesStore.removeFolder(path)
  toast.success(t('common.saved'))
}
</script>

<template>
  <MDialog
    :open="open"
    :title="t('instances.folders.title')"
    :hide-footer="true"
    @update:open="emit('update:open', $event)"
  >
    <div class="flex flex-col gap-4 max-h-[60vh] overflow-y-auto">
      <MEmpty
        v-if="instancesStore.folders.length === 0"
        :title="t('instances.folders.emptyFolders')"
      >
        <MButton
          variant="primary"
          size="sm"
          icon="i-lucide-folder-plus"
          data-testid="add-folder-btn"
          @click="onPickAndAddGameFolder"
        >
          {{ t('instances.folders.addFolder') }}
        </MButton>
      </MEmpty>

      <div v-else class="space-y-2">
        <div
          v-for="folder in instancesStore.folders"
          :key="folder.path"
          class="flex flex-col sm:flex-row sm:items-center justify-between p-3 rounded-xl border border-line bg-sunken/40 gap-3"
        >
          <div class="min-w-0 flex-1">
            <div v-if="editingFolderPath === folder.path" class="flex items-center gap-2">
              <MInput
                v-model="editingRemarkName"
                size="sm"
                class="w-48"
                data-testid="edit-remark-input"
                @keydown.enter="saveRemark(folder.path)"
                @keydown.esc="cancelEditRemark"
              />
              <MButton
                variant="primary"
                size="xs"
                icon="i-lucide-check"
                data-testid="save-remark-btn"
                @click="saveRemark(folder.path)"
              >
                {{ t('common.confirm') }}
              </MButton>
              <MButton variant="text" size="xs" icon="i-lucide-x" @click="cancelEditRemark">
                {{ t('common.cancel') }}
              </MButton>
            </div>
            <div v-else class="flex items-center gap-2">
              <span class="font-medium text-sm text-fg">{{ folder.name }}</span>
              <MBadge v-if="folder.path === instancesStore.currentFolder" variant="primary">
                {{ t('instances.folders.currentBadge') }}
              </MBadge>
            </div>
            <div class="flex items-center gap-1.5 text-xs text-fg-muted font-mono mt-1 min-w-0">
              <span class="truncate select-text">{{ formatFolderPath(folder.path) }}</span>
              <MHelpTooltip :text="folder.path" size="xs" />
            </div>
          </div>
          <div class="flex items-center gap-2 shrink-0 flex-wrap">
            <MButton
              v-if="folder.path !== instancesStore.currentFolder"
              variant="tonal"
              size="xs"
              @click="instancesStore.switchFolder(folder.path)"
            >
              {{ t('instances.folders.switchFolder') }}
            </MButton>
            <MIconButton
              variant="text"
              size="sm"
              icon="i-lucide-pencil"
              data-testid="edit-remark-btn"
              :title="t('instances.folders.editRemark')"
              @click="startEditRemark(folder)"
            />
            <MIconButton
              variant="text"
              size="sm"
              icon="i-lucide-folder-open"
              data-testid="open-folder-btn"
              :title="t('instances.folders.openFolder')"
              @click="openFolderInOs(folder.path)"
            />
            <MIconButton
              variant="text"
              size="sm"
              icon="i-lucide-trash-2"
              class="text-danger"
              data-testid="delete-folder-btn"
              :title="t('instances.folders.removeFolder')"
              @click="onRemoveFolder(folder.path)"
            />
          </div>
        </div>

        <div class="pt-2 flex justify-end">
          <MButton
            variant="primary"
            size="sm"
            icon="i-lucide-folder-plus"
            data-testid="add-folder-btn"
            @click="onPickAndAddGameFolder"
          >
            {{ t('instances.folders.addFolder') }}
          </MButton>
        </div>
      </div>
    </div>
  </MDialog>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import type { InstanceInfo } from '../../../ipc/client'
import { ipcClient } from '../../../ipc/client'
import { useInstancesStore } from '../../instances/store'
import {
  MCard,
  MButton,
  MBadge,
  MDialog,
  MInput,
  useToast,
  useDialog,
} from '../../../ui'

const props = defineProps<{
  instance: InstanceInfo
}>()

const { t } = useI18n()
const router = useRouter()
const toast = useToast()
const { confirm } = useDialog()
const instancesStore = useInstancesStore()

const showRenameModal = ref(false)
const newName = ref('')
const repairing = ref(false)

function openRename() {
  newName.value = props.instance.name
  showRenameModal.value = true
}

async function handleRename() {
  const trimmed = newName.value.trim()
  if (!trimmed || trimmed === props.instance.name) {
    showRenameModal.value = false
    return
  }
  try {
    await instancesStore.rename(props.instance.id, trimmed)
    toast.success(t('instanceDetail.overview.renameSuccess'))
    showRenameModal.value = false
  } catch (e: unknown) {
    toast.danger(String(e))
  }
}

async function handleOpenFolder() {
  try {
    await ipcClient.instanceOpenFolder(props.instance.id)
  } catch (e: unknown) {
    toast.danger(String(e))
  }
}

async function handleRepair() {
  const ok = await confirm({
    title: t('instanceDetail.overview.repair'),
    content: t('instanceDetail.overview.repairConfirm'),
  })
  if (!ok) return

  repairing.value = true
  try {
    await ipcClient.installRepair(props.instance.id)
    toast.success(t('instanceDetail.overview.repairStarted'))
  } catch (e: unknown) {
    toast.danger(String(e))
  } finally {
    repairing.value = false
  }
}

async function handleDelete() {
  const ok = await confirm({
    title: t('instanceDetail.overview.deleteConfirmTitle'),
    content: t('instanceDetail.overview.deleteConfirm', { name: props.instance.name }),
  })
  if (!ok) return

  try {
    await instancesStore.remove(props.instance.id)
    toast.success(t('instanceDetail.overview.deleteSuccess'))
    router.push('/instances')
  } catch (e: unknown) {
    toast.danger(String(e))
  }
}

async function setIcon(iconKey: string) {
  try {
    await ipcClient.instanceUpdateConfig(props.instance.id, {
      ...props.instance.config,
      icon: iconKey,
    })
    await instancesStore.fetchInstances()
  } catch (e: unknown) {
    toast.danger(String(e))
  }
}

const presetIcons = [
  { key: 'grass_block', labelKey: 'instanceDetail.overview.iconGrass', icon: 'i-lucide-box' },
  { key: 'chest', labelKey: 'instanceDetail.overview.iconChest', icon: 'i-lucide-archive' },
  { key: 'diamond', labelKey: 'instanceDetail.overview.iconDiamond', icon: 'i-lucide-gem' },
  { key: 'sword', labelKey: 'instanceDetail.overview.iconSword', icon: 'i-lucide-sword' },
  { key: 'furnace', labelKey: 'instanceDetail.overview.iconFurnace', icon: 'i-lucide-flame' },
]
</script>

<template>
  <div class="max-w-4xl space-y-6">
    <!-- Basic Info Card -->
    <MCard class="p-6 space-y-4">
      <h2 class="text-base font-bold text-text flex items-center gap-2">
        <span class="w-5 h-5 i-lucide-info text-primary" />
        {{ t('instanceDetail.overview.title') }}
      </h2>

      <div class="grid grid-cols-1 md:grid-cols-2 gap-4 text-sm">
        <div class="space-y-1">
          <div class="text-xs text-textMuted font-medium">
            {{ t('instanceDetail.overview.name') }}
          </div>
          <div class="text-text font-semibold flex items-center gap-2">
            <span>{{ instance.name }}</span>
            <MButton variant="text" size="sm" @click="openRename">
              <span class="w-3.5 h-3.5 i-lucide-edit-2 mr-1" />
              {{ t('instanceDetail.overview.rename') }}
            </MButton>
          </div>
        </div>

        <div class="space-y-1">
          <div class="text-xs text-textMuted font-medium">
            {{ t('instanceDetail.overview.mcVersion') }}
          </div>
          <div class="text-text font-mono font-semibold">
            {{ instance.mcVersion }}
          </div>
        </div>

        <div class="space-y-1">
          <div class="text-xs text-textMuted font-medium">
            {{ t('instanceDetail.overview.kind') }}
          </div>
          <div class="text-text capitalize">
            {{ instance.kind }}
          </div>
        </div>

        <div class="space-y-1">
          <div class="text-xs text-textMuted font-medium">
            {{ t('instanceDetail.overview.loaders') }}
          </div>
          <div class="flex flex-wrap gap-1.5 items-center">
            <MBadge
              v-for="loader in instance.loaders"
              :key="loader"
              variant="primary"
            >
              {{ loader }}
            </MBadge>
            <span v-if="instance.loaders.length === 0" class="text-textMuted">
              Vanilla
            </span>
          </div>
        </div>

        <div class="space-y-1 md:col-span-2">
          <div class="text-xs text-textMuted font-medium">
            {{ t('instanceDetail.overview.path') }}
          </div>
          <div class="text-text font-mono text-xs break-all bg-surfaceVariant/40 p-2.5 rounded-lg border border-outline/20">
            {{ instance.path }}
          </div>
        </div>
      </div>
    </MCard>

    <!-- Icon Picker Card -->
    <MCard class="p-6 space-y-4">
      <h2 class="text-base font-bold text-text flex items-center gap-2">
        <span class="w-5 h-5 i-lucide-smile text-primary" />
        {{ t('instanceDetail.overview.icon') }}
      </h2>
      <div class="flex flex-wrap gap-3">
        <button
          v-for="item in presetIcons"
          :key="item.key"
          type="button"
          class="flex items-center gap-2 px-3.5 py-2 rounded-xl border text-sm font-medium transition-all"
          :class="[
            instance.config?.icon === item.key
              ? 'border-primary bg-primary/10 text-primary'
              : 'border-outline/30 hover:border-outline text-textMuted hover:text-text',
          ]"
          @click="setIcon(item.key)"
        >
          <span class="w-4 h-4" :class="item.icon" />
          <span>{{ t(item.labelKey) }}</span>
        </button>
      </div>
    </MCard>

    <!-- Actions Card -->
    <MCard class="p-6 space-y-4">
      <h2 class="text-base font-bold text-text flex items-center gap-2">
        <span class="w-5 h-5 i-lucide-wrench text-primary" />
        {{ t('instanceDetail.overview.actions') }}
      </h2>

      <div class="flex flex-wrap gap-3">
        <MButton variant="tonal" @click="handleOpenFolder">
          <span class="w-4 h-4 i-lucide-folder-open mr-1.5" />
          {{ t('instanceDetail.overview.openFolder') }}
        </MButton>

        <MButton
          variant="tonal"
          :loading="repairing"
          @click="handleRepair"
        >
          <span class="w-4 h-4 i-lucide-refresh-cw mr-1.5" />
          {{ t('instanceDetail.overview.repair') }}
        </MButton>

        <MButton variant="danger" @click="handleDelete">
          <span class="w-4 h-4 i-lucide-trash-2 mr-1.5" />
          {{ t('instanceDetail.overview.delete') }}
        </MButton>
      </div>
    </MCard>

    <!-- Rename Dialog -->
    <MDialog
      :open="showRenameModal"
      :title="t('instanceDetail.overview.renameTitle')"
      @update:open="showRenameModal = $event"
    >
      <div class="py-2 space-y-4">
        <MInput
          v-model="newName"
          :label="t('instanceDetail.overview.name')"
          autofocus
          @keydown.enter="handleRename"
        />
      </div>
      <template #footer>
        <div class="flex justify-end gap-2">
          <MButton variant="text" @click="showRenameModal = false">
            {{ t('common.cancel') }}
          </MButton>
          <MButton variant="primary" @click="handleRename">
            {{ t('common.confirm') }}
          </MButton>
        </div>
      </template>
    </MDialog>
  </div>
</template>

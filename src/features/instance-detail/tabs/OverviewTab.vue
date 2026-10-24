<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import type { InstanceInfo } from '../../../ipc/client'
import { ipcClient } from '../../../ipc/client'
import { useInstancesStore } from '../../instances/store'
import {
  MSection,
  MSettingRow,
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
    const updated = await ipcClient.instanceUpdateConfig(props.instance.id, {
      ...props.instance.config,
      icon: iconKey,
    })
    const idx = instancesStore.instances.findIndex((i) => i.id === props.instance.id)
    if (idx >= 0) {
      instancesStore.instances[idx] = updated
    }
    toast.success(t('common.save'))
  } catch (e: unknown) {
    toast.danger(String(e))
  }
}

const presetIcons = [
  { key: 'grass', labelKey: 'instanceDetail.overview.iconGrass', icon: 'i-lucide-box' },
  { key: 'chest', labelKey: 'instanceDetail.overview.iconChest', icon: 'i-lucide-archive' },
  { key: 'diamond', labelKey: 'instanceDetail.overview.iconDiamond', icon: 'i-lucide-gem' },
  { key: 'sword', labelKey: 'instanceDetail.overview.iconSword', icon: 'i-lucide-sword' },
  { key: 'furnace', labelKey: 'instanceDetail.overview.iconFurnace', icon: 'i-lucide-flame' },
]
</script>

<template>
  <div class="space-y-6">
    <!-- Basic Info Section -->
    <MSection :title="t('instanceDetail.overview.title')" icon="i-lucide-info">
      <template #actions>
        <MButton variant="text" size="xs" icon="i-lucide-edit-2" @click="openRename">
          {{ t('instanceDetail.overview.rename') }}
        </MButton>
      </template>

      <div class="divide-y divide-line">
        <MSettingRow :label="t('instanceDetail.overview.name')">
          <span class="font-semibold text-fg">{{ instance.name }}</span>
        </MSettingRow>

        <MSettingRow :label="t('instanceDetail.overview.mcVersion')">
          <span class="font-mono text-fg">{{ instance.mcVersion }}</span>
        </MSettingRow>

        <MSettingRow :label="t('instanceDetail.overview.kind')">
          <span class="capitalize text-fg">{{ instance.kind }}</span>
        </MSettingRow>

        <MSettingRow :label="t('instanceDetail.overview.loaders')">
          <div class="flex flex-wrap gap-1.5 items-center justify-end">
            <MBadge
              v-for="loader in instance.loaders"
              :key="loader"
              variant="primary"
            >
              {{ loader }}
            </MBadge>
            <span v-if="instance.loaders.length === 0" class="text-fg-muted">
              Vanilla
            </span>
          </div>
        </MSettingRow>

        <MSettingRow :label="t('instanceDetail.overview.path')">
          <span class="font-mono text-xs text-fg-muted break-all text-right max-w-md">{{ instance.path }}</span>
        </MSettingRow>
      </div>
    </MSection>

    <!-- Icon Picker Section -->
    <MSection :title="t('instanceDetail.overview.icon')" icon="i-lucide-smile">
      <div class="flex flex-wrap gap-3">
        <button
          v-for="item in presetIcons"
          :key="item.key"
          type="button"
          class="flex items-center gap-2 px-3.5 py-2 rounded-xl border text-sm font-medium transition-all cursor-pointer"
          :class="[
            instance.config?.icon === item.key
              ? 'border-primary bg-primary-soft text-primary-soft-fg'
              : 'border-line hover:border-line-strong text-fg-muted hover:text-fg bg-sunken/40',
          ]"
          @click="setIcon(item.key)"
        >
          <span class="w-4 h-4" :class="item.icon" />
          <span>{{ t(item.labelKey) }}</span>
        </button>
      </div>
    </MSection>

    <!-- Actions Section -->
    <MSection :title="t('instanceDetail.overview.actions')" icon="i-lucide-wrench">
      <div class="flex flex-wrap gap-3">
        <MButton variant="tonal" size="sm" icon="i-lucide-folder-open" @click="handleOpenFolder">
          {{ t('instanceDetail.overview.openFolder') }}
        </MButton>

        <MButton
          variant="tonal"
          size="sm"
          icon="i-lucide-refresh-cw"
          :loading="repairing"
          @click="handleRepair"
        >
          {{ t('instanceDetail.overview.repair') }}
        </MButton>

        <MButton variant="danger" size="sm" icon="i-lucide-trash-2" @click="handleDelete">
          {{ t('instanceDetail.overview.delete') }}
        </MButton>
      </div>
    </MSection>

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
          size="sm"
          autofocus
          @keyup.enter="handleRename"
        />
      </div>
      <template #footer>
        <div class="flex justify-end gap-2">
          <MButton variant="text" size="sm" @click="showRenameModal = false">
            {{ t('common.cancel') }}
          </MButton>
          <MButton variant="primary" size="sm" @click="handleRename">
            {{ t('common.confirm') }}
          </MButton>
        </div>
      </template>
    </MDialog>
  </div>
</template>

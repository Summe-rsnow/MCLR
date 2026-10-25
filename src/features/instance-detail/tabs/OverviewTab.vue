<script setup lang="ts">
import { computed, ref } from 'vue'
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
  MSelect,
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
const renameError = ref('')
const repairing = ref(false)

const cardTypeOptions = computed(() => [
  { label: t('instanceDetail.overview.cardTypeAuto'), value: '' },
  { label: t('instances.groups.star'), value: 'star' },
  { label: t('instances.groups.api'), value: 'api' },
  { label: t('instances.groups.regular'), value: 'regular' },
  { label: t('instances.groups.rubbish'), value: 'rubbish' },
  { label: t('instances.groups.fool'), value: 'fool' },
  { label: t('instances.groups.hidden'), value: 'hidden' },
])

function validateName(val: string): string {
  const name = val.trim()
  if (!name) return t('error.instance.name.empty')
  if (val.startsWith(' ') || val.endsWith(' ')) return t('error.instance.name.invalidChar')
  if (val.endsWith('.')) return t('error.instance.name.trailingDot')
  if (val.length > 70) return t('error.instance.name.tooLong')
  if (/[\\/:*?"<>|!;]/.test(val)) return t('error.instance.name.invalidChar')
  return ''
}

function openRename() {
  newName.value = props.instance.name
  renameError.value = ''
  showRenameModal.value = true
}

async function handleRename() {
  const trimmed = newName.value.trim()
  if (!trimmed || trimmed === props.instance.name) {
    showRenameModal.value = false
    return
  }
  const err = validateName(newName.value)
  if (err) {
    renameError.value = err
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
    await ipcClient.instanceOpenPath(props.instance.id, '')
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

async function handleDelete(e?: MouseEvent) {
  const isPermanent = e?.shiftKey
  let content = isPermanent
    ? t('instances.actions.deletePermanentConfirm', { name: props.instance.name })
    : t('instanceDetail.overview.deleteConfirm', { name: props.instance.name })

  if (props.instance.config?.isolation === 'on') {
    content += `\n\n${t('instanceDetail.overview.deleteIsolatedWarning')}`
  }

  const ok = await confirm({
    title: isPermanent
      ? t('instances.actions.deletePermanent')
      : t('instanceDetail.overview.deleteConfirmTitle'),
    content,
    confirmText: isPermanent ? t('instances.actions.deletePermanent') : t('common.delete'),
  })
  if (!ok) return

  try {
    await instancesStore.remove(props.instance.id, isPermanent)
    toast.success(t('instanceDetail.overview.deleteSuccess'))
    router.push('/instances')
  } catch (e: unknown) {
    toast.danger(String(e))
  }
}

async function toggleStar() {
  try {
    await instancesStore.toggleStar(props.instance.id)
    toast.success(t('common.save'))
  } catch (e: unknown) {
    toast.danger(String(e))
  }
}

async function setCardType(val: string) {
  try {
    const updated = await ipcClient.instanceUpdateConfig(props.instance.id, {
      ...props.instance.config,
      cardType: (val || null) as any,
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

        <MSettingRow :label="t('instanceDetail.overview.cardTypeLabel')" :hint="t('instanceDetail.overview.cardTypeHint')">
          <MSelect
            :model-value="instance.config?.cardType || ''"
            :options="cardTypeOptions"
            size="sm"
            class="w-48"
            @update:model-value="setCardType(String($event))"
          />
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
        <MButton
          :variant="instance.config?.starred || instance.config?.pinned ? 'primary' : 'tonal'"
          size="sm"
          :icon="instance.config?.starred || instance.config?.pinned ? 'i-lucide-star-off' : 'i-lucide-star'"
          @click="toggleStar"
        >
          {{ instance.config?.starred || instance.config?.pinned ? t('instances.actions.unstar') : t('instances.actions.star') }}
        </MButton>

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

        <MButton variant="danger" size="sm" icon="i-lucide-trash-2" @click="handleDelete($event)">
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
          :error="renameError || validateName(newName)"
          size="sm"
          autofocus
          @input="renameError = validateName(newName)"
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

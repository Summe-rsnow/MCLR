<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useModsStore } from './store'
import { useInstancesStore } from '../instances/store'
import type { LocalModInfo } from '../../ipc/client'
import JarInJarItem from './JarInJarItem.vue'
import {
  MInput,
  MSelect,
  MSegmented,
  MButton,
  MIconButton,
  MCard,
  MBadge,
  MSwitch,
  MDialog,
  MEmpty,
  MSkeleton,
  useToast,
  useDialog,
} from '../../ui'

const props = defineProps<{
  instanceId?: string
}>()

const { t } = useI18n()
const toast = useToast()
const dialog = useDialog()
const modsStore = useModsStore()
const instancesStore = useInstancesStore()

const currentInstanceId = ref<string>('')

const filterOptions = computed(() => [
  { label: t('mods.filter.all'), value: 'all' },
  { label: t('mods.filter.enabled'), value: 'enabled' },
  { label: t('mods.filter.disabled'), value: 'disabled' },
  { label: t('mods.filter.updatable'), value: 'updatable' },
])

const instanceOptions = computed(() => {
  return instancesStore.instances.map((i) => ({
    label: `${i.name} (${i.mcVersion})`,
    value: i.name,
  }))
})

const selectedInstance = computed(() => {
  return instancesStore.instances.find((i) => i.name === currentInstanceId.value)
})

const updatableCount = computed(() => {
  return modsStore.mods.filter((m) => !!m.updateFile).length
})

watch(
  () => props.instanceId,
  (newId) => {
    if (newId) {
      currentInstanceId.value = newId
      modsStore.loadMods(newId)
    }
  },
  { immediate: true },
)

watch(currentInstanceId, (newId) => {
  if (newId && newId !== modsStore.selectedInstanceId) {
    modsStore.loadMods(newId)
  }
})

async function onCheckUpdates(): Promise<void> {
  const mcVer = selectedInstance.value?.mcVersion || '1.20.1'
  const loader = selectedInstance.value?.loaders[0] || 'fabric'
  await modsStore.checkUpdates(mcVer, loader)
  if (updatableCount.value === 0) {
    toast.show({
      message: t('mods.status.noUpdates'),
      type: 'info',
    })
  }
}

async function onUpdateAll(): Promise<void> {
  const count = await modsStore.updateAllMods()
  toast.show({
    message: t('mods.updateAllSuccess', { count }),
    type: 'success',
  })
}

async function onToggleMod(mod: LocalModInfo): Promise<void> {
  try {
    await modsStore.toggleMod(mod)
  } catch (err: unknown) {
    toast.show({
      message: String(err),
      type: 'danger',
    })
  }
}

async function onDeleteMod(mod: LocalModInfo): Promise<void> {
  const confirmed = await dialog.confirm({
    title: t('mods.deleteConfirmTitle'),
    content: t('mods.deleteConfirmMessage', { name: mod.name, fileName: mod.fileName }),
    confirmText: t('mods.actions.delete'),
  })
  if (confirmed) {
    await modsStore.deleteMod(mod)
  }
}

async function onUpdateMod(mod: LocalModInfo): Promise<void> {
  await modsStore.updateMod(mod)
  toast.show({
    message: t('mods.updateSuccess', { name: mod.name }),
    type: 'success',
  })
}

function formatSize(bytes: number): string {
  if (bytes >= 1024 * 1024) return (bytes / (1024 * 1024)).toFixed(2) + ' MB'
  if (bytes >= 1024) return (bytes / 1024).toFixed(1) + ' KB'
  return bytes + ' B'
}

onMounted(async () => {
  if (instancesStore.instances.length === 0) {
    await instancesStore.fetchInstances()
  }
  if (!currentInstanceId.value && instancesStore.instances.length > 0) {
    currentInstanceId.value = instancesStore.instances[0].name
    await modsStore.loadMods(currentInstanceId.value)
  }
})
</script>

<template>
  <div class="h-full flex flex-col p-6 space-y-4 overflow-y-auto">
    <!-- Header Controls -->
    <div class="flex flex-wrap items-center justify-between gap-4">
      <!-- Instance Selector (if not fixed by props) -->
      <div v-if="!props.instanceId" class="flex items-center gap-2">
        <span class="w-4 h-4 i-lucide-layers text-textMuted shrink-0" />
        <span class="text-xs font-semibold text-textMuted">{{ t('mods.instanceSelect') }}:</span>
        <div class="w-56">
          <MSelect
            v-model="currentInstanceId"
            :options="instanceOptions"
            size="sm"
          />
        </div>
      </div>

      <!-- Action Buttons -->
      <div class="flex items-center space-x-2">
        <MButton
          variant="tonal"
          size="sm"
          :loading="modsStore.loading"
          @click="onCheckUpdates"
        >
          <span class="i-lucide-refresh-cw mr-1.5 w-4 h-4" />
          {{ t('mods.actions.checkUpdates') }}
        </MButton>
        <MButton
          v-if="updatableCount > 0"
          variant="primary"
          size="sm"
          @click="onUpdateAll"
        >
          <span class="i-lucide-arrow-up-circle mr-1.5 w-4 h-4" />
          {{ t('mods.actions.updateAll') }} ({{ updatableCount }})
        </MButton>
      </div>
    </div>

    <!-- Filter Bar -->
    <div class="flex flex-wrap items-center gap-3 bg-surface-variant/20 p-3 rounded-xl border border-outline/20">
      <div class="flex-1 min-w-[200px]">
        <MInput
          v-model="modsStore.searchQuery"
          :placeholder="t('mods.searchPlaceholder')"
          size="sm"
          clearable
        />
      </div>

      <MSegmented
        v-model="modsStore.filterState"
        :options="filterOptions"
        size="sm"
      />
    </div>

    <!-- Mod List Content -->
    <div v-if="modsStore.loading && modsStore.mods.length === 0" class="space-y-3">
      <MSkeleton class="h-20 rounded-xl" />
      <MSkeleton class="h-20 rounded-xl" />
      <MSkeleton class="h-20 rounded-xl" />
    </div>

    <div v-else-if="modsStore.filteredMods.length === 0" class="py-16">
      <MEmpty
        :title="modsStore.mods.length === 0 ? t('mods.empty') : t('mods.emptySearch')"
      />
    </div>

    <div v-else class="space-y-3">
      <MCard
        v-for="mod in modsStore.filteredMods"
        :key="mod.fileName"
        class="p-4 flex flex-col md:flex-row md:items-center justify-between gap-4 transition-all"
        :class="{ 'opacity-60 bg-surface-variant/10': !mod.enabled }"
      >
        <!-- Left: Info -->
        <div class="flex items-start space-x-3 min-w-0">
          <div class="mt-1">
            <MSwitch
              :model-value="mod.enabled"
              @update:model-value="onToggleMod(mod)"
            />
          </div>
          <div class="space-y-1 min-w-0">
            <div class="flex flex-wrap items-center gap-2">
              <span class="font-semibold text-text text-sm truncate" :title="mod.name">
                {{ mod.name }}
              </span>
              <span class="text-xs text-textMuted font-mono">v{{ mod.version }}</span>
              <MBadge variant="neutral" size="sm">{{ mod.loader }}</MBadge>
              <MBadge
                :variant="mod.enabled ? 'success' : 'neutral'"
                size="sm"
              >
                {{ mod.enabled ? t('mods.status.enabled') : t('mods.status.disabled') }}
              </MBadge>
              <MBadge
                v-if="mod.updateFile"
                variant="warning"
                size="sm"
              >
                {{ t('mods.status.updateAvailable', { version: mod.updateFile.name }) }}
              </MBadge>
            </div>

            <div v-if="mod.description" class="text-xs text-textMuted line-clamp-1">
              {{ mod.description }}
            </div>

            <div class="flex flex-wrap items-center gap-3 text-[11px] text-textMuted">
              <span class="font-mono">{{ mod.fileName }}</span>
              <span>•</span>
              <span>{{ formatSize(mod.size) }}</span>
              <template v-if="mod.authors && mod.authors.length > 0">
                <span>•</span>
                <span>{{ mod.authors.join(', ') }}</span>
              </template>
            </div>
          </div>
        </div>

        <!-- Right: Actions -->
        <div class="flex items-center space-x-2 shrink-0 self-end md:self-center">
          <MButton
            v-if="mod.updateFile"
            variant="tonal"
            size="sm"
            @click="onUpdateMod(mod)"
          >
            <span class="i-lucide-arrow-up mr-1 w-3.5 h-3.5" />
            {{ t('mods.actions.update') }}
          </MButton>

          <MIconButton
            icon="i-lucide-folder-tree"
            size="sm"
            :title="t('mods.actions.jarInJar')"
            @click="modsStore.openJarInJar(mod)"
          />

          <MIconButton
            icon="i-lucide-trash-2"
            size="sm"
            variant="text"
            class="text-danger hover:bg-danger/10"
            :title="t('mods.actions.delete')"
            @click="onDeleteMod(mod)"
          />
        </div>
      </MCard>
    </div>

    <!-- Jar-in-Jar Drawer / Dialog -->
    <MDialog
      :open="!!modsStore.jarInJarTarget"
      :title="t('mods.jarInJarDrawer.title')"
      hide-footer
      @update:open="modsStore.closeJarInJar"
      @cancel="modsStore.closeJarInJar"
    >
      <div class="space-y-3 py-2 max-h-[60vh] overflow-y-auto">
        <div class="text-xs text-textMuted">
          {{ t('mods.jarInJarDrawer.subtitle', { name: modsStore.jarInJarTarget?.name || '' }) }}
        </div>

        <div v-if="modsStore.jarInJarLoading" class="py-6 space-y-2">
          <MSkeleton class="h-8 rounded-lg" />
          <MSkeleton class="h-8 rounded-lg" />
        </div>

        <div v-else-if="modsStore.jarInJarTree.length === 0" class="py-8">
          <MEmpty
            :title="t('mods.jarInJarDrawer.empty')"
          />
        </div>

        <div v-else class="space-y-2">
          <JarInJarItem
            v-for="item in modsStore.jarInJarTree"
            :key="item.id + item.version"
            :node="item"
          />
        </div>
      </div>
    </MDialog>
  </div>
</template>

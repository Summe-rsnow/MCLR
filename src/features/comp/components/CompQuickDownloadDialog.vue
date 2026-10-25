<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { ipcClient, type CompProject, type CompFile } from '../../../ipc/client'
import MInput from '../../../ui/MInput.vue'
import { useCompStore } from '../store'
import { useInstancesStore } from '../../instances/store'
import {
  MDialog,
  MButton,
  MBadge,
  MSelect,
  MSkeleton,
  MEmpty,
  MBrandIcon,
  MImage,
  useToast,
} from '../../../ui'

const props = defineProps<{
  open: boolean
  project: CompProject | null
  initialTargetInstanceId?: string
}>()

const emit = defineEmits<{
  (e: 'update:open', val: boolean): void
  (e: 'view-details', project: CompProject): void
}>()

const { t } = useI18n()
const toast = useToast()
const compStore = useCompStore()
const instancesStore = useInstancesStore()

const files = ref<CompFile[]>([])
const loadingFiles = ref(false)
const downloadingFileId = ref<string | null>(null)
const targetInstanceId = ref<string>('')
const autoMatchInstance = ref(true)
const modpackInstanceName = ref('')

watch(
  () => props.project,
  (proj) => {
    if (proj?.compType === 'modpack') {
      modpackInstanceName.value = (proj.title || 'Modpack').replace(/[/\\?%*:|"<>]/g, '_').trim()
    }
  },
  { immediate: true },
)

const selectedMcVersion = ref('')
const selectedLoader = ref('')

const canonicalMcVersions = [
  '26.3',
  '26.2',
  '26.1',
  '1.21.1',
  '1.21',
  '1.20.1',
  '1.20',
  '1.19.4',
  '1.18.2',
  '1.16.5',
  '1.12.2',
]

const availableLoaders = [
  { label: 'NeoForge', value: 'neoforge' },
  { label: 'Fabric', value: 'fabric' },
  { label: 'Forge', value: 'forge' },
  { label: 'Quilt', value: 'quilt' },
]

function normalizeLoader(raw: string): string {
  const s = raw.trim().toLowerCase()
  if (s.includes('neoforge')) return 'neoforge'
  if (s.includes('fabric')) return 'fabric'
  if (s.includes('quilt')) return 'quilt'
  if (s.includes('forge')) return 'forge'
  if (s.includes('cleanroom')) return 'cleanroom'
  if (s.includes('liteloader')) return 'liteloader'
  if (s.includes('optifine')) return 'optifine'
  if (s.includes('labymod')) return 'labymod'
  return s
}

function getCompatibleLoaders(selectedLdr: string, mcVer: string): Set<string> {
  const norm = normalizeLoader(selectedLdr)
  const set = new Set<string>()
  if (!norm) return set
  set.add(norm)

  // PCL Rule 1: 1.20.1 NeoForge runs Forge mods!
  if (norm === 'neoforge' && (mcVer === '1.20.1' || mcVer.startsWith('1.20.1'))) {
    set.add('forge')
  }
  // PCL Rule 2: Quilt has native backward compatibility with Fabric mods across all versions
  if (norm === 'quilt') {
    set.add('fabric')
  }
  // PCL Rule 3: Cleanroom in 1.12.2 is Forge compatible
  if (norm === 'cleanroom') {
    set.add('forge')
  }
  // PCL Rule 4: LegacyFabric is Fabric compatible
  if (norm === 'legacyfabric') {
    set.add('fabric')
  }
  return set
}
const targetInstanceOptions = computed(() => {
  const isMod = props.project?.compType === 'mod'
  const available = isMod
    ? instancesStore.instances.filter((i) => i.modable)
    : instancesStore.instances

  return available.map((i) => ({
    label: `${i.name} (${i.mcVersion}${i.loaders?.[0] ? ' • ' + i.loaders[0] : ''})`,
    value: i.id,
  }))
})

const selectedInstance = computed(() => {
  return instancesStore.instances.find((i) => i.id === targetInstanceId.value) || null
})

function applyInstanceFilter(inst: typeof selectedInstance.value) {
  if (!inst) return
  selectedMcVersion.value = inst.mcVersion || ''
  const ldr = inst.loaders?.[0] || inst.kind || ''
  selectedLoader.value = normalizeLoader(ldr)
}

watch(
  selectedInstance,
  (newInst) => {
    if (autoMatchInstance.value && newInst) {
      applyInstanceFilter(newInst)
    }
  },
  { immediate: true },
)

watch(autoMatchInstance, (auto) => {
  if (auto && selectedInstance.value) {
    applyInstanceFilter(selectedInstance.value)
  }
})

watch(
  () => props.open,
  async (isOpen) => {
    if (!isOpen || !props.project) return

    // Initialize target instance
    if (
      props.initialTargetInstanceId &&
      instancesStore.instances.some((i) => i.id === props.initialTargetInstanceId)
    ) {
      targetInstanceId.value = props.initialTargetInstanceId
    } else if (instancesStore.instances.length > 0 && instancesStore.instances[0]?.id) {
      targetInstanceId.value = instancesStore.instances[0].id
    }

    if (autoMatchInstance.value && selectedInstance.value) {
      applyInstanceFilter(selectedInstance.value)
    }

    loadingFiles.value = true
    try {
      await compStore.fetchProjectFiles(props.project.platform, props.project.id)
      files.value = compStore.projectFiles
    } finally {
      loadingFiles.value = false
    }
  },
  { immediate: true },
)

// PCL compatibility check algorithm
const filteredFiles = computed(() => {
  const isNonModType =
    props.project?.compType === 'resourcepack' ||
    props.project?.compType === 'shader' ||
    props.project?.compType === 'datapack' ||
    props.project?.compType === 'world'

  const targetVer = selectedMcVersion.value.trim().toLowerCase()
  const targetLdr = selectedLoader.value.trim().toLowerCase()
  const allowedLoaders = getCompatibleLoaders(targetLdr, targetVer)

  const matched = files.value.filter((f) => {
    // 1. Version matching (PCL rules)
    const verMatch =
      !targetVer ||
      f.mcVersions.length === 0 ||
      f.mcVersions.some((v) => {
        const vClean = v.trim().toLowerCase()
        return vClean === targetVer || targetVer.startsWith(vClean) || vClean.startsWith(targetVer)
      })

    // 2. Loader matching (PCL rules)
    const fileLoaders = f.loaders.map(normalizeLoader)
    const loaderMatch =
      isNonModType ||
      allowedLoaders.size === 0 ||
      f.loaders.length === 0 ||
      fileLoaders.some((fl) => allowedLoaders.has(fl))

    return verMatch && loaderMatch
  })

  return matched
})

function formatFileSize(bytes: number): string {
  if (bytes >= 1024 * 1024) return (bytes / (1024 * 1024)).toFixed(1) + ' MB'
  return (bytes / 1024).toFixed(1) + ' KB'
}

function formatDate(iso: string): string {
  try {
    return new Date(iso).toLocaleDateString()
  } catch {
    return iso
  }
}

function getChannelVariant(channel: string): 'primary' | 'warning' | 'neutral' {
  if (channel === 'release') return 'primary'
  if (channel === 'beta') return 'warning'
  return 'neutral'
}

async function handleDownload(file: CompFile) {
  if (!targetInstanceId.value) {
    toast.warning(t('comp.quickDownload.selectInstanceFirst'))
    return
  }
  if (!props.project) return

  downloadingFileId.value = file.id
  try {
    await compStore.downloadFile(file, targetInstanceId.value, props.project.compType)
    toast.success(
      t('comp.quickDownload.downloadSuccess', {
        name: file.fileName,
        instance: selectedInstance.value?.name || targetInstanceId.value,
      }),
    )
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : t('comp.downloadDialog.failed')
    toast.danger(msg)
  } finally {
    downloadingFileId.value = null
  }
}

function onViewDetails() {
  if (!props.project) return
  emit('update:open', false)
  emit('view-details', props.project)
}

async function handleInstallModpack(file: CompFile) {
  const name = modpackInstanceName.value.trim()
  if (!name) {
    toast.warning(t('comp.modpackInstallDialog.instanceNamePlaceholder'))
    return
  }
  downloadingFileId.value = file.id
  try {
    await ipcClient.modpackInstallRemote(file, name)
    toast.success(t('comp.modpackInstallDialog.success'))
    emit('update:open', false)
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : t('comp.modpackInstallDialog.failed')
    toast.danger(msg)
  } finally {
    downloadingFileId.value = null
  }
}
function onSelectVersion(ver: string) {
  selectedMcVersion.value = ver
  autoMatchInstance.value = false
}

function onSelectLoader(ldr: string) {
  selectedLoader.value = ldr
  autoMatchInstance.value = false
}
</script>

<template>
  <MDialog
    :open="open"
    :title="t('comp.quickDownload.title')"
    max-width="lg"
    hide-footer
    @update:open="emit('update:open', $event)"
    @cancel="emit('update:open', false)"
  >
    <div v-if="project" class="space-y-3.5 text-fg">
      <!-- Project Header -->
      <div
        class="flex items-start justify-between gap-3 p-3.5 rounded-xl bg-sunken/50 border border-line"
      >
        <div class="flex items-start gap-3 min-w-0">
          <MImage
            :src="project.iconUrl"
            fallback-icon="i-lucide-box"
            class="w-12 h-12 rounded-xl bg-sunken shrink-0 border border-line"
          />

          <div class="min-w-0 flex-1">
            <div class="flex items-center gap-2">
              <h3 class="font-bold text-base text-fg truncate">
                {{ project.title }}
              </h3>
              <MBadge variant="neutral" size="sm">
                <MBrandIcon
                  v-if="project.platform === 'modrinth' || project.platform === 'curseforge'"
                  :name="project.platform"
                  class="w-3.5 h-3.5 mr-1"
                />
                {{ project.platform }}
              </MBadge>
            </div>
            <p class="text-xs text-fg-muted mt-0.5 line-clamp-1">
              {{ project.summary || project.description }}
            </p>
          </div>
        </div>

        <MButton
          variant="tonal"
          size="sm"
          class="shrink-0"
          data-testid="quick-view-details-btn"
          @click="onViewDetails"
        >
          <span class="i-lucide-external-link mr-1.5 w-3.5 h-3.5" />
          {{ t('comp.quickDownload.viewDetails') }}
        </MButton>
      </div>

      <!-- Modpack: New Instance Name Input (PCL Logic: Modpack creates new instance) -->
      <div
        v-if="project.compType === 'modpack'"
        class="p-3 rounded-xl bg-sunken/40 border border-line space-y-1.5"
      >
        <label class="text-xs font-semibold text-fg-muted block">
          {{ t('comp.modpackInstallDialog.instanceName') }}:
        </label>
        <MInput
          v-model="modpackInstanceName"
          :placeholder="t('comp.modpackInstallDialog.instanceNamePlaceholder')"
          size="sm"
        />
      </div>

      <!-- Non-Modpack: Target Instance & Auto-Match Bar -->
      <div
        v-else
        class="flex items-center justify-between gap-3 p-2.5 rounded-xl bg-sunken/40 border border-line"
      >
        <div class="flex items-center gap-2 flex-1 min-w-0">
          <span class="text-xs font-semibold text-fg-muted shrink-0">
            {{ t('comp.quickDownload.targetInstance') }}:
          </span>
          <div class="flex-1 min-w-0">
            <MSelect v-model="targetInstanceId" :options="targetInstanceOptions" size="sm" />
          </div>
        </div>

        <button
          type="button"
          class="text-xs flex items-center gap-1.5 text-fg-muted hover:text-fg cursor-pointer select-none transition shrink-0 pl-1"
          @click="autoMatchInstance = !autoMatchInstance"
        >
          <span
            class="w-4 h-4 rounded border flex items-center justify-center transition"
            :class="
              autoMatchInstance
                ? 'bg-primary border-primary text-white'
                : 'border-line-strong bg-sunken'
            "
          >
            <span v-if="autoMatchInstance" class="i-lucide-check w-3 h-3 block" />
          </span>
          <span>{{ t('comp.quickDownload.autoMatchInstance') }}</span>
        </button>
      </div>

      <!-- PCL-Style Retrieval & Filter Panel -->
      <div class="space-y-2 p-3 rounded-xl bg-sunken/40 border border-line text-xs">
        <!-- Game Version Chips -->
        <div class="flex flex-wrap items-center gap-1.5">
          <span class="font-semibold text-fg-muted shrink-0 mr-1">
            {{ t('comp.quickDownload.filterVersion') }}:
          </span>
          <button
            type="button"
            class="px-2.5 py-0.5 rounded-lg transition font-medium cursor-pointer"
            :class="
              !selectedMcVersion
                ? 'bg-primary text-white shadow-xs'
                : 'text-fg-muted hover:text-fg hover:bg-sunken'
            "
            @click="onSelectVersion('')"
          >
            {{ t('comp.quickDownload.allVersions') }}
          </button>
          <button
            v-for="ver in canonicalMcVersions"
            :key="ver"
            type="button"
            class="px-2 py-0.5 rounded-lg transition cursor-pointer font-mono"
            :class="
              selectedMcVersion === ver
                ? 'bg-primary text-white shadow-xs font-semibold'
                : 'text-fg-muted hover:text-fg hover:bg-sunken'
            "
            @click="onSelectVersion(ver)"
          >
            {{ ver }}
          </button>
        </div>

        <!-- Loader Chips (if mod type) -->
        <div
          v-if="project.compType === 'mod'"
          class="flex flex-wrap items-center gap-1.5 pt-1.5 border-t border-line/40"
        >
          <span class="font-semibold text-fg-muted shrink-0 mr-1">
            {{ t('comp.quickDownload.filterLoader') }}:
          </span>
          <button
            type="button"
            class="px-2.5 py-0.5 rounded-lg transition font-medium cursor-pointer"
            :class="
              !selectedLoader
                ? 'bg-primary text-white shadow-xs'
                : 'text-fg-muted hover:text-fg hover:bg-sunken'
            "
            @click="onSelectLoader('')"
          >
            {{ t('comp.quickDownload.allLoaders') }}
          </button>
          <button
            v-for="ldr in availableLoaders"
            :key="ldr.value"
            type="button"
            class="px-2.5 py-0.5 rounded-lg transition cursor-pointer capitalize"
            :class="
              selectedLoader.toLowerCase() === ldr.value.toLowerCase()
                ? 'bg-primary text-white shadow-xs font-semibold'
                : 'text-fg-muted hover:text-fg hover:bg-sunken'
            "
            @click="onSelectLoader(ldr.value)"
          >
            {{ ldr.label }}
          </button>
        </div>
      </div>

      <!-- Files List -->
      <div class="space-y-2">
        <div v-if="loadingFiles" class="space-y-2 py-2">
          <MSkeleton v-for="n in 3" :key="n" class="h-16 w-full rounded-xl" />
        </div>

        <div v-else-if="filteredFiles.length === 0" class="py-8 text-center">
          <MEmpty :title="t('comp.quickDownload.noFiles')" />
        </div>

        <div v-else class="space-y-2 max-h-[300px] overflow-y-auto pr-1">
          <div
            v-for="file in filteredFiles"
            :key="file.id"
            class="p-3 rounded-xl bg-sunken/40 border border-line flex items-center justify-between gap-3 hover:border-line-strong transition"
          >
            <div class="min-w-0 flex-1 space-y-1">
              <div class="flex items-center gap-2">
                <MBadge :variant="getChannelVariant(file.channel)" size="sm">
                  {{ t(`comp.files.channel.${file.channel}`) }}
                </MBadge>
                <span class="font-medium text-xs text-fg truncate" :title="file.name">
                  {{ file.name }}
                </span>
              </div>
              <div class="flex flex-wrap items-center gap-2 text-xs text-fg-muted">
                <span>{{ formatDate(file.releaseDate) }}</span>
                <span>•</span>
                <span>{{ formatFileSize(file.size) }}</span>
                <span v-if="file.loaders.length > 0">•</span>
                <span v-if="file.loaders.length > 0" class="truncate font-mono capitalize">
                  {{ file.loaders.join(', ') }}
                </span>
                <span v-if="file.mcVersions.length > 0">•</span>
                <span v-if="file.mcVersions.length > 0" class="truncate font-mono">
                  {{ file.mcVersions.slice(0, 3).join(', ') }}
                </span>
              </div>
            </div>

            <MButton
              v-if="project.compType === 'modpack'"
              variant="primary"
              size="sm"
              class="shrink-0"
              :loading="downloadingFileId === file.id"
              :disabled="file.distributionDisabled || !modpackInstanceName.trim()"
              @click="handleInstallModpack(file)"
            >
              <span class="i-lucide-box mr-1 w-3.5 h-3.5" />
              {{ t('comp.modpackInstallDialog.install') }}
            </MButton>
            <MButton
              v-else
              variant="primary"
              size="sm"
              class="shrink-0"
              :loading="downloadingFileId === file.id"
              :disabled="file.distributionDisabled"
              @click="handleDownload(file)"
            >
              <span class="i-lucide-download mr-1 w-3.5 h-3.5" />
              {{ t('comp.quickDownload.download') }}
            </MButton>
          </div>
        </div>
      </div>
    </div>
  </MDialog>
</template>

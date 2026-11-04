<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  ipcClient,
  type CompProject,
  type CompFile,
  type CompDependency,
} from '../../../ipc/client'
import MInput from '../../../ui/MInput.vue'
import { useCompStore } from '../store'
import { useInstancesStore } from '../../instances/store'
import { awaitTask } from '../../tasks/awaitTask'
import { useErrorText } from '../../../composables/useErrorText'
import {
  MDialog,
  MButton,
  MIconButton,
  MBadge,
  MSelect,
  MSkeleton,
  MEmpty,
  MBrandIcon,
  MImage,
  getInstanceIconMeta,
  useToast,
} from '../../../ui'
import { useFormat } from '../../../composables/useFormat'
import AddToPresetDialog from './AddToPresetDialog.vue'
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
const { formatError } = useErrorText()
const toast = useToast()
const compStore = useCompStore()
const instancesStore = useInstancesStore()

const projectStack = ref<CompProject[]>([])

watch(
  () => props.open,
  (isOpen) => {
    if (isOpen && props.project) {
      projectStack.value = [props.project]
    } else if (!isOpen) {
      projectStack.value = []
    }
  },
  { immediate: true },
)

watch(
  () => props.project,
  (proj) => {
    if (props.open && proj) {
      projectStack.value = [proj]
    }
  },
)

const activeProject = computed<CompProject | null>(() => {
  return projectStack.value[projectStack.value.length - 1] ?? props.project
})

const canGoBack = computed(() => projectStack.value.length > 1)
const previousProject = computed<CompProject | null>(() => {
  return projectStack.value.length > 1 ? projectStack.value[projectStack.value.length - 2] : null
})

function handleGoBack() {
  if (projectStack.value.length > 1) {
    projectStack.value.pop()
  }
}

async function handleOpenPrerequisite(dep: CompDependency) {
  let proj = getDepProject(dep.projectId)
  if (!proj) {
    try {
      proj = await ipcClient.compProject(activeProject.value?.platform || 'modrinth', dep.projectId)
    } catch {
      return
    }
  }
  if (proj) {
    projectStack.value.push(proj)
  }
}

/** A modpack is unpacked into a new instance, so it never targets an existing one. */
const isModpack = computed(() => activeProject.value?.compType === 'modpack')
const files = ref<CompFile[]>([])
const loadingFiles = ref(false)
const downloadingFileId = ref<string | null>(null)
const targetInstanceId = ref<string>('')
const autoMatchInstance = ref(true)
const modpackInstanceName = ref('')
/** Set by the install click: the name prompt only exists so the click can be confirmed. */
const modpackNameDialogOpen = ref(false)
const pendingModpackFile = ref<CompFile | null>(null)
const selectedMcVersion = ref('')
const selectedLoader = ref('')
const depProjects = ref<Record<string, CompProject>>({})
const depLoading = ref(false)
const installedModKeys = ref<Set<string>>(new Set())

const depConfirmDialogOpen = ref(false)
const pendingDepFile = ref<CompFile | null>(null)
const missingRequiredDeps = ref<CompDependency[]>([])

async function refreshInstalledMods() {
  if (!targetInstanceId.value) {
    installedModKeys.value = new Set()
    return
  }
  try {
    const entries = await ipcClient.resourcesList(targetInstanceId.value, 'mod')
    const keys = new Set<string>()
    for (const e of entries) {
      if (e.projectId) keys.add(e.projectId.toLowerCase())
      if (e.modId) keys.add(e.modId.toLowerCase())
    }
    installedModKeys.value = keys
  } catch {
    installedModKeys.value = new Set()
  }
}

watch(
  targetInstanceId,
  () => {
    void refreshInstalledMods()
  },
  { immediate: true },
)

async function loadDependenciesForFiles() {
  if (!props.project || files.value.length === 0) return
  const allIds = new Set<string>()
  for (const f of files.value) {
    for (const d of f.dependencies) {
      if (d.projectId) allIds.add(d.projectId)
    }
  }
  if (allIds.size === 0) return
  depLoading.value = true
  try {
    const projects = await ipcClient.compDepProjects(props.project.platform, [...allIds])
    const map: Record<string, CompProject> = {}
    for (const p of projects) {
      map[p.id] = p
      if (p.slug) map[p.slug] = p
    }
    depProjects.value = map
  } catch {
    // fallback
  } finally {
    depLoading.value = false
  }
}

watch(files, () => {
  void loadDependenciesForFiles()
})

function getDepProject(projectId: string): CompProject | null {
  return depProjects.value[projectId] ?? null
}

function getDepTitle(projectId: string): string {
  const p = getDepProject(projectId)
  if (p) {
    return (compStore.showZhName && p.zhName) || p.title || p.slug || projectId
  }
  return projectId
}

function isDepInstalled(projectId: string): boolean {
  const lower = projectId.toLowerCase()
  const p = getDepProject(projectId)
  if (installedModKeys.value.has(lower)) return true
  if (p) {
    if (p.id && installedModKeys.value.has(p.id.toLowerCase())) return true
    if (p.slug && installedModKeys.value.has(p.slug.toLowerCase())) return true
  }
  return false
}

watch(
  activeProject,
  async (proj) => {
    if (!proj) {
      files.value = []
      return
    }
    if (proj.compType === 'modpack') {
      modpackInstanceName.value = (proj.title || 'Modpack').replace(/[/\\?%*:|"<>]/g, '_').trim()
      selectedMcVersion.value = ''
      selectedLoader.value = ''
    } else if (autoMatchInstance.value && selectedInstance.value) {
      applyInstanceFilter(selectedInstance.value)
    }

    loadingFiles.value = true
    try {
      await compStore.fetchProjectFiles(proj.platform, proj.id)
      files.value = compStore.projectFiles
    } finally {
      loadingFiles.value = false
    }
  },
  { immediate: true },
)

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
  const isMod = activeProject.value?.compType === 'mod'
  const available = isMod
    ? instancesStore.instances.filter((i) => i.modable)
    : instancesStore.instances

  return [
    { label: t('comp.search.noTargetInstance'), value: '', icon: 'i-lucide-box-select' },
    ...available.map((i) => {
      const kind = i.kind || (i.loaders?.[0]?.toLowerCase() ?? 'vanilla')
      const meta = getInstanceIconMeta(i.config?.icon, kind)
      return {
        label: `${i.name} (${i.mcVersion}${i.loaders?.[0] ? ' • ' + i.loaders[0] : ''})`,
        value: i.id,
        image: meta.image,
        icon: meta.icon,
      }
    }),
  ]
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
    if (!isOpen || !props.project) {
      // Never leave the name prompt floating over a dialog that was closed under it.
      modpackNameDialogOpen.value = false
      pendingModpackFile.value = null
      return
    }

    if (isModpack.value) {
      // Nothing to target: the archive is unpacked into a fresh instance of its own.
      targetInstanceId.value = ''
      selectedMcVersion.value = ''
      selectedLoader.value = ''
    } else {
      // Initialize target instance strictly from initialTargetInstanceId
      if (
        props.initialTargetInstanceId &&
        instancesStore.instances.some((i) => i.id === props.initialTargetInstanceId)
      ) {
        targetInstanceId.value = props.initialTargetInstanceId
      } else {
        targetInstanceId.value = ''
      }

      if (autoMatchInstance.value && selectedInstance.value) {
        applyInstanceFilter(selectedInstance.value)
      } else if (!targetInstanceId.value) {
        selectedMcVersion.value = ''
        selectedLoader.value = ''
      }
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

// Region-aware formatting: culture comes from the language settings.
const { formatDate } = useFormat()

function getChannelVariant(channel: string): 'primary' | 'warning' | 'neutral' {
  if (channel === 'release') return 'primary'
  if (channel === 'beta') return 'warning'
  return 'neutral'
}

async function handleDownloadClick(file: CompFile) {
  if (isModpack.value) {
    openModpackNameDialog(file)
    return
  }
  if (!targetInstanceId.value) {
    toast.warning(t('comp.quickDownload.selectInstanceFirst'))
    return
  }

  const missing = file.dependencies.filter(
    (d) => d.kind === 'required' && !isDepInstalled(d.projectId),
  )
  if (missing.length > 0) {
    pendingDepFile.value = file
    missingRequiredDeps.value = missing
    depConfirmDialogOpen.value = true
    return
  }
  await executeDownload(file)
}

async function confirmDownloadWithDeps(withDeps: boolean) {
  const file = pendingDepFile.value
  if (!file) return
  depConfirmDialogOpen.value = false

  if (!withDeps) {
    await executeDownload(file)
    return
  }

  downloadingFileId.value = file.id
  try {
    const inst = selectedInstance.value
    if (!inst) {
      await executeDownload(file)
      return
    }
    const resolved = await ipcClient.compResolveDependencies(
      file,
      inst.id,
      inst.mcVersion,
      inst.loaders?.[0] || 'fabric',
    )
    const filesToDownload: Array<{ file: CompFile; slug: string | null }> = [
      { file, slug: props.project?.slug ?? null },
    ]
    for (const dep of resolved) {
      if (!dep.installed) {
        filesToDownload.push({ file: dep.file, slug: dep.project.slug })
      }
    }
    for (const item of filesToDownload) {
      await compStore.downloadFile(item.file, inst.id, 'mod', item.slug)
    }
    toast.success(
      t('comp.quickDownload.downloadSuccess', {
        name: file.fileName,
        instance: inst.name || targetInstanceId.value,
      }),
    )
    void refreshInstalledMods()
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : t('comp.downloadDialog.failed')
    toast.danger(msg)
  } finally {
    downloadingFileId.value = null
    pendingDepFile.value = null
  }
}

async function executeDownload(file: CompFile) {
  if (!targetInstanceId.value) {
    toast.warning(t('comp.quickDownload.selectInstanceFirst'))
    return
  }
  if (!activeProject.value) return

  downloadingFileId.value = file.id
  try {
    await compStore.downloadFile(
      file,
      targetInstanceId.value,
      activeProject.value.compType,
      activeProject.value.slug,
    )
    toast.success(
      t('comp.quickDownload.downloadSuccess', {
        name: file.fileName,
        instance: selectedInstance.value?.name || targetInstanceId.value,
      }),
    )
    void refreshInstalledMods()
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : t('comp.downloadDialog.failed')
    toast.danger(msg)
  } finally {
    downloadingFileId.value = null
  }
}

function onViewDetails() {
  if (!activeProject.value) return
  emit('update:open', false)
  emit('view-details', activeProject.value)
}

/**
 * A modpack is not installed *into* anything: it becomes a new instance. Asking for that
 * instance's name here — instead of demanding it before the user has even picked a version —
 * keeps the version list one click away.
 */
function openModpackNameDialog(file: CompFile) {
  pendingModpackFile.value = file
  modpackInstanceName.value = (props.project?.title || file.name || 'Modpack')
    .replace(/[/\\?%*:|"<>]/g, '_')
    .trim()
  modpackNameDialogOpen.value = true
}

async function confirmInstallModpack() {
  const file = pendingModpackFile.value
  const name = modpackInstanceName.value.trim()
  if (!file || !name) {
    toast.warning(t('comp.modpackInstallDialog.instanceNamePlaceholder'))
    return
  }
  downloadingFileId.value = file.id
  modpackNameDialogOpen.value = false
  try {
    const taskId = await ipcClient.modpackInstallRemote(file, name)
    emit('update:open', false)
    // The command only queues the work: unpack + loader + base game run in the task.
    const outcome = await awaitTask(taskId)
    if (outcome.state === 'failed' || outcome.state === 'canceled') {
      toast.danger(formatError(outcome.error) || t('comp.modpackInstallDialog.failed'))
      return
    }
    if (outcome.state === 'success') {
      // Make the freshly completed instance visible everywhere.
      await instancesStore.fetchInstances(true)
    }
    toast.success(t('comp.modpackInstallDialog.success'))
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : t('comp.modpackInstallDialog.failed')
    toast.danger(msg)
  } finally {
    downloadingFileId.value = null
    pendingModpackFile.value = null
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

function openExternalUrl(url?: string | null) {
  if (!url) return
  if (typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window) {
    import('@tauri-apps/plugin-opener').then((m) => {
      m.openUrl(url)
    })
  } else {
    window.open(url, '_blank')
  }
}

function openMcMod() {
  if (!activeProject.value) return
  const proj = activeProject.value
  const cleanTitle = proj.title.split('|')[0]?.trim() || proj.title
  const url = proj.wikiId
    ? `https://www.mcmod.cn/class/${proj.wikiId}.html`
    : `https://www.mcmod.cn/s?key=${encodeURIComponent(cleanTitle)}&site=all&filter=0`
  openExternalUrl(url)
}

function openWebsite() {
  if (!activeProject.value) return
  const p = activeProject.value
  const url =
    p.websiteUrl ||
    (p.platform === 'modrinth'
      ? `https://modrinth.com/${p.compType}/${p.slug || p.id}`
      : `https://www.curseforge.com/minecraft/mc-mods/${p.slug || p.id}`)
  openExternalUrl(url)
}

function copyLink() {
  if (!activeProject.value) return
  const p = activeProject.value
  const url =
    p.websiteUrl ||
    (p.platform === 'modrinth'
      ? `https://modrinth.com/${p.compType}/${p.slug || p.id}`
      : `https://www.curseforge.com/minecraft/mc-mods/${p.slug || p.id}`)
  navigator.clipboard.writeText(url)
  toast.success(t('comp.project.copiedLink'))
}

const addToPresetOpen = ref(false)

function openAddToPreset() {
  addToPresetOpen.value = true
}

const isFavorite = computed(() => {
  if (!activeProject.value) return false
  return compStore.isFavorite(activeProject.value.platform, activeProject.value.id)
})

function handleToggleFavorite() {
  if (!activeProject.value) return
  compStore.toggleFavorite(activeProject.value)
}
</script>

<template>
  <MDialog
    :open="open"
    :title="t('comp.quickDownload.title')"
    max-width="xl"
    hide-footer
    @update:open="emit('update:open', $event)"
    @cancel="emit('update:open', false)"
  >
    <div v-if="activeProject" class="space-y-3.5 text-fg">
      <!-- In-dialog Back Navigation -->
      <div v-if="canGoBack" class="flex items-center gap-2">
        <MButton variant="text" size="xs" @click="handleGoBack">
          <span class="i-lucide-arrow-left mr-1" />
          {{
            t('comp.quickDownload.backToPrevious', {
              name:
                (compStore.showZhName && previousProject?.zhName) || previousProject?.title || '',
            })
          }}
        </MButton>
      </div>

      <!-- Project Header -->
      <div class="p-3.5 rounded-xl bg-sunken/50 border border-line space-y-2.5">
        <div class="flex items-start gap-3">
          <MImage
            :src="activeProject.iconUrl"
            fallback-icon="i-lucide-box"
            class="w-12 h-12 rounded-xl shrink-0"
          />

          <div class="min-w-0 flex-1 space-y-1.5">
            <!-- Row 1: Title + Platform Badge on Left, View Details on Right -->
            <div class="flex items-center justify-between gap-3">
              <div class="flex items-center gap-2 flex-wrap min-w-0">
                <h3 class="font-bold text-base text-fg truncate">
                  {{ (compStore.showZhName && activeProject.zhName) || activeProject.title }}
                </h3>
                <MBadge variant="neutral" size="sm">
                  <MBrandIcon
                    v-if="
                      activeProject.platform === 'modrinth' ||
                      activeProject.platform === 'curseforge'
                    "
                    :name="activeProject.platform"
                    class="w-3.5 h-3.5 mr-1"
                  />
                  {{ activeProject.platform }}
                </MBadge>
              </div>

              <MButton
                variant="tonal"
                size="xs"
                class="h-7 text-xs px-2.5 font-medium shrink-0"
                data-testid="quick-view-details-btn"
                @click="onViewDetails"
              >
                <span class="i-lucide-external-link mr-1 w-3.5 h-3.5" />
                {{ t('comp.quickDownload.viewDetails') }}
              </MButton>
            </div>

            <!-- Row 2: Subtitle Metadata on Left, Action Toolbar on Right -->
            <div class="flex items-center justify-between gap-3">
              <!-- Left: English Name + Author combined -->
              <div class="text-xs text-fg-muted truncate flex items-center gap-1.5 min-w-0 flex-1">
                <span
                  v-if="
                    compStore.showZhName &&
                    activeProject.zhName &&
                    activeProject.zhName !== activeProject.title
                  "
                  class="truncate"
                  :title="activeProject.title"
                >
                  {{ activeProject.title }}
                </span>
                <span
                  v-if="
                    compStore.showZhName &&
                    activeProject.zhName &&
                    activeProject.zhName !== activeProject.title &&
                    activeProject.author
                  "
                  class="text-fg-subtle shrink-0"
                >
                  ·
                </span>
                <div v-if="activeProject.author" class="flex items-center gap-1 shrink-0 truncate">
                  <MBrandIcon
                    v-if="
                      activeProject.platform === 'modrinth' ||
                      activeProject.platform === 'curseforge'
                    "
                    :name="activeProject.platform"
                    class="w-3 h-3 text-fg-muted shrink-0"
                  />
                  <span class="truncate">{{
                    t('comp.project.author', { author: activeProject.author })
                  }}</span>
                </div>
              </div>

              <!-- Right: Compact Action Buttons directly under View Details -->
              <div class="flex items-center gap-1 shrink-0">
                <MIconButton
                  v-if="activeProject.compType === 'mod'"
                  icon="i-lucide-book-open"
                  variant="tonal"
                  size="sm"
                  :tooltip="t('comp.project.mcmod')"
                  @click="openMcMod"
                />

                <MIconButton
                  icon="i-lucide-globe"
                  variant="tonal"
                  size="sm"
                  :tooltip="t('comp.project.projectHome')"
                  @click="openWebsite"
                />

                <MIconButton
                  v-if="activeProject.compType !== 'modpack'"
                  icon="i-lucide-package-plus"
                  variant="tonal"
                  size="sm"
                  :tooltip="t('comp.project.addToPreset')"
                  @click="openAddToPreset"
                />

                <MIconButton
                  :icon="
                    isFavorite
                      ? 'i-lucide-heart text-rose-500 fill-current'
                      : 'i-lucide-heart text-rose-500'
                  "
                  :variant="isFavorite ? 'tonal' : 'text'"
                  size="sm"
                  :tooltip="isFavorite ? t('comp.project.favorited') : t('comp.project.favorite')"
                  @click="handleToggleFavorite"
                />

                <MIconButton
                  icon="i-lucide-link"
                  variant="text"
                  size="sm"
                  :tooltip="t('comp.project.copyLink')"
                  @click="copyLink"
                />
              </div>
            </div>
          </div>
        </div>

        <!-- Row 3: Summary -->
        <p
          v-if="activeProject.summary || activeProject.description"
          class="text-xs text-fg-muted line-clamp-2 leading-relaxed pt-2 border-t border-line/40"
        >
          {{ activeProject.summary || activeProject.description }}
        </p>
      </div>

      <!-- Non-Modpack: Target Instance & Auto-Match Bar -->
      <div
        v-if="activeProject.compType !== 'modpack'"
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
        <div class="flex items-start gap-1.5">
          <span class="font-semibold text-fg-muted shrink-0 mr-1 pt-0.5">
            {{ t('comp.quickDownload.filterVersion') }}:
          </span>
          <div class="flex flex-wrap items-center gap-1.5 flex-1 max-h-20 overflow-y-auto pr-1">
            <button
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
        </div>

        <!-- Loader Chips (mods and modpacks are the loader-bound types) -->
        <div
          v-if="activeProject?.compType === 'mod' || activeProject?.compType === 'modpack'"
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
            class="p-3 rounded-xl bg-sunken/40 border border-line flex flex-col gap-2.5 hover:border-line-strong transition"
          >
            <div class="flex items-center justify-between gap-3 w-full">
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
                v-if="activeProject?.compType === 'modpack'"
                variant="primary"
                size="sm"
                class="shrink-0"
                :loading="downloadingFileId === file.id"
                :disabled="file.distributionDisabled"
                @click="openModpackNameDialog(file)"
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
                @click="handleDownloadClick(file)"
              >
                <span class="i-lucide-download mr-1 w-3.5 h-3.5" />
                {{ t('comp.quickDownload.download') }}
              </MButton>
            </div>

            <!-- Prerequisite Mods -->
            <div
              v-if="file.dependencies.length > 0"
              class="pt-2 border-t border-line/40 flex items-center gap-1.5 flex-wrap text-xs"
            >
              <span class="text-fg-muted font-medium flex items-center gap-1 shrink-0">
                <span class="i-lucide-link w-3.5 h-3.5" />
                {{ t('comp.quickDownload.dependencies') }}:
              </span>
              <button
                v-for="dep in file.dependencies"
                :key="dep.projectId + '-' + dep.kind"
                type="button"
                class="inline-flex items-center gap-1 px-2 py-0.5 rounded-md border border-line bg-raised/70 hover:border-primary hover:bg-raised transition cursor-pointer text-left"
                :title="t('comp.quickDownload.clickToViewPrerequisite')"
                @click.stop="handleOpenPrerequisite(dep)"
              >
                <MImage
                  :src="getDepProject(dep.projectId)?.iconUrl"
                  fallback-icon="i-lucide-puzzle"
                  class="w-3.5 h-3.5 rounded shrink-0"
                />
                <span class="font-medium text-fg max-w-[140px] truncate">
                  {{ getDepTitle(dep.projectId) }}
                </span>
                <MBadge
                  v-if="dep.kind === 'required'"
                  :variant="isDepInstalled(dep.projectId) ? 'success' : 'warning'"
                  size="sm"
                >
                  {{
                    isDepInstalled(dep.projectId)
                      ? t('comp.downloadDialog.depInstalled')
                      : t('comp.quickDownload.missing')
                  }}
                </MBadge>
                <MBadge v-else variant="neutral" size="sm">
                  {{ t(`comp.fileDependencies.kind.${dep.kind}`) }}
                </MBadge>
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </MDialog>

  <!-- Prerequisite Confirmation Dialog -->
  <MDialog
    :open="depConfirmDialogOpen"
    :title="t('comp.quickDownload.confirmInstallDepsTitle')"
    @update:open="depConfirmDialogOpen = $event"
    @cancel="depConfirmDialogOpen = false"
  >
    <div class="space-y-3 py-2 text-xs">
      <p class="text-fg">
        {{ t('comp.quickDownload.confirmInstallDepsMessage') }}
      </p>
      <div class="space-y-1.5 max-h-48 overflow-y-auto">
        <div
          v-for="dep in missingRequiredDeps"
          :key="dep.projectId"
          class="flex items-center gap-2 p-2 rounded-lg border border-line bg-panel"
        >
          <MImage
            :src="getDepProject(dep.projectId)?.iconUrl"
            fallback-icon="i-lucide-puzzle"
            class="w-5 h-5 rounded shrink-0"
          />
          <span class="font-medium text-fg flex-1 min-w-0 truncate">
            {{ getDepTitle(dep.projectId) }}
          </span>
          <MBadge variant="warning" size="sm">
            {{ t('comp.quickDownload.missing') }}
          </MBadge>
        </div>
      </div>
    </div>
    <template #footer>
      <div class="flex items-center justify-end gap-2">
        <MButton variant="text" size="sm" @click="depConfirmDialogOpen = false">
          {{ t('common.cancel') }}
        </MButton>
        <MButton variant="tonal" size="sm" @click="confirmDownloadWithDeps(false)">
          {{ t('comp.quickDownload.downloadModOnly') }}
        </MButton>
        <MButton variant="primary" size="sm" @click="confirmDownloadWithDeps(true)">
          {{ t('comp.quickDownload.downloadWithDeps') }}
        </MButton>
      </div>
    </template>
  </MDialog>

  <!-- The instance name is only needed once a specific version is chosen, so it is asked here
       rather than blocking the version list behind an upfront input. -->
  <MDialog
    :open="modpackNameDialogOpen"
    :title="t('comp.modpackInstallDialog.title')"
    :confirm-text="t('comp.modpackInstallDialog.install')"
    :cancel-text="t('comp.downloadDialog.cancel')"
    :confirm-loading="downloadingFileId !== null"
    :confirm-disabled="!modpackInstanceName.trim()"
    @update:open="modpackNameDialogOpen = $event"
    @confirm="confirmInstallModpack"
    @cancel="modpackNameDialogOpen = false"
  >
    <div class="space-y-2 py-2">
      <label class="text-xs font-semibold text-fg-muted block">
        {{ t('comp.modpackInstallDialog.instanceName') }}
      </label>
      <MInput
        v-model="modpackInstanceName"
        data-testid="modpack-instance-name-input"
        :placeholder="t('comp.modpackInstallDialog.instanceNamePlaceholder')"
        @keydown.enter="confirmInstallModpack"
      />
    </div>
  </MDialog>

  <AddToPresetDialog v-model:open="addToPresetOpen" :project="activeProject" />
</template>

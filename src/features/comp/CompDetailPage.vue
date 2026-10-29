<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useCompStore } from './store'
import { useInstancesStore } from '../instances/store'
import { ipcClient, type CompFile, type CompProject, type CompPlatform } from '../../ipc/client'
import { MCard, MIconButton, MSkeleton, MMarkdown, useToast } from '../../ui'
import { awaitTask } from '../tasks/awaitTask'
import { useErrorText } from '../../composables/useErrorText'
import CompProjectHeader from './components/CompProjectHeader.vue'
import CompVersionsTab, { type FileGroup } from './components/CompVersionsTab.vue'
import CompDependencyDialogs, { type ResolvedDep } from './components/CompDependencyDialogs.vue'

const route = useRoute()
const router = useRouter()
const { t } = useI18n()
const { formatError } = useErrorText()
const toast = useToast()
const compStore = useCompStore()
const instancesStore = useInstancesStore()

const platform = computed(() => (route?.params?.platform as CompPlatform) || '')
const projectId = computed(() => (route?.params?.id as string) || '')

const selectedMcVersion = ref<string>('')
const selectedLoader = ref<string>('')

// Download Dialog state
const downloadDialogOpen = ref(false)
const selectedTargetFile = ref<CompFile | null>(null)
const selectedTargetInstance = ref<string>('')
const downloading = ref(false)
const resolvingDependencies = ref(false)
const dependencies = ref<ResolvedDep[]>([])
const selectedDepFiles = ref<Record<string, boolean>>({})
const depConfirmDialogOpen = ref(false)
const cannotInstallDialogOpen = ref(false)
const resolveFailedDialogOpen = ref(false)
const sessionTargetByKind: Record<string, string> = {}

// Modpack Install Dialog state
const modpackInstallDialogOpen = ref(false)
const modpackTargetFile = ref<CompFile | null>(null)
const modpackInstanceName = ref('')
const installingModpack = ref(false)

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
  if (norm === 'quilt') set.add('fabric')
  if (norm === 'cleanroom') set.add('forge')
  if (norm === 'legacyfabric') set.add('fabric')
  return set
}

const filteredFiles = computed(() => {
  let files = compStore.projectFiles
  if (selectedMcVersion.value) {
    const ver = selectedMcVersion.value.trim().toLowerCase()
    files = files.filter(
      (f) =>
        f.mcVersions.length === 0 ||
        f.mcVersions.some((v) => {
          const vClean = v.trim().toLowerCase()
          return vClean === ver || ver.startsWith(vClean) || vClean.startsWith(ver)
        }),
    )
  }
  if (selectedLoader.value) {
    const allowed = getCompatibleLoaders(selectedLoader.value, selectedMcVersion.value)
    files = files.filter(
      (f) => f.loaders.length === 0 || f.loaders.some((l) => allowed.has(normalizeLoader(l))),
    )
  }
  return files
})

function isCleanReleaseVersion(ver: string): boolean {
  const lower = ver.toLowerCase().trim()
  if (
    lower.includes('w') ||
    lower.includes('pre') ||
    lower.includes('rc') ||
    lower.includes('snapshot') ||
    lower.includes('alpha') ||
    lower.includes('beta')
  ) {
    return false
  }
  return /^(1\.\d+(\.\d+)?|\d{2}\.\d+(\.\d+)?)$/.test(lower)
}

function compareVersionsDesc(a: string, b: string): number {
  const pa = a.split('.').map((n) => parseInt(n, 10) || 0)
  const pb = b.split('.').map((n) => parseInt(n, 10) || 0)
  for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
    const na = pa[i] ?? 0
    const nb = pb[i] ?? 0
    if (na !== nb) return nb - na
  }
  return b.localeCompare(a)
}

const mcVersionFilterOptions = computed(() => {
  const versions = new Set<string>()
  for (const f of compStore.projectFiles) {
    for (const v of f.mcVersions) {
      if (isCleanReleaseVersion(v)) {
        versions.add(v)
      }
    }
  }
  const sorted = Array.from(versions).sort(compareVersionsDesc)
  return [
    { label: t('comp.search.allMcVersions'), value: '' },
    ...sorted.map((v) => ({ label: v, value: v })),
  ]
})

const loaderFilterOptions = computed(() => {
  const loaders = new Set<string>()
  for (const f of compStore.projectFiles) {
    for (const l of f.loaders) {
      loaders.add(l)
    }
  }
  return [
    { label: t('comp.search.allLoaders'), value: '' },
    ...Array.from(loaders).map((l) => ({ label: l, value: l })),
  ]
})

const instanceOptions = computed(() => {
  return instancesStore.instances.map((i) => ({
    label: `${i.name} (${i.mcVersion})`,
    value: i.id,
  }))
})

function isPreReleaseOrSnapshot(v: string): boolean {
  const lower = v.toLowerCase()
  return (
    lower.includes('w') ||
    lower.includes('pre') ||
    lower.includes('rc') ||
    lower.includes('snapshot') ||
    lower.includes('alpha') ||
    lower.includes('beta')
  )
}

function getMajorGroupKey(v: string): string {
  const clean = v.trim().toLowerCase()
  if (isPreReleaseOrSnapshot(clean)) {
    return '__snapshot__'
  }
  const parts = clean.split('.')
  if (parts.length >= 2) {
    return `${parts[0]}.${parts[1]}`
  }
  return clean || '__other__'
}

function getFileHighestVersion(mcVersions: string[]): string {
  if (!mcVersions.length) return ''
  const sorted = [...mcVersions].sort(compareVersionsDesc)
  return sorted[0] || ''
}

const fileGroups = computed<FileGroup[]>(() => {
  const map: Record<string, CompFile[]> = {}
  for (const f of filteredFiles.value) {
    const primaryVer = getFileHighestVersion(f.mcVersions)
    const key = getMajorGroupKey(primaryVer)
    if (!map[key]) map[key] = []
    map[key].push(f)
  }

  const keys = Object.keys(map)
  const numericKeys = keys.filter((k) => k !== '__snapshot__' && k !== '__other__')
  numericKeys.sort(compareVersionsDesc)

  const sortedKeys: string[] = [...numericKeys]
  if (map['__snapshot__']) sortedKeys.push('__snapshot__')
  if (map['__other__']) sortedKeys.push('__other__')

  return sortedKeys.map((k, index) => {
    let title = k
    if (k === '__snapshot__') {
      title = t('comp.files.snapshotGroup')
    } else if (k === '__other__') {
      title = t('comp.files.otherVersions')
    }
    const groupFiles = (map[k] || []).slice().sort((a, b) => {
      const ta = new Date(a.releaseDate).getTime() || 0
      const tb = new Date(b.releaseDate).getTime() || 0
      return tb - ta
    })
    return {
      id: k,
      title,
      files: groupFiles,
      open: index === 0,
    }
  })
})

function openExternalUrl(url?: string | null) {
  if (url) {
    import('@tauri-apps/plugin-opener').then((m) => {
      m.openUrl(url)
    })
  }
}

async function handleToggleFavorite(project: CompProject) {
  const isFav = compStore.isFavorite(project.platform, project.id)
  await compStore.toggleFavorite(project)
  if (!isFav) {
    toast.success(t('comp.favorites.added'))
  } else {
    toast.info(t('comp.favorites.removed'))
  }
}

function openDownloadDialog(file: CompFile) {
  if (compStore.currentProject?.compType === 'modpack') {
    modpackTargetFile.value = file
    const projTitle = compStore.currentProject?.title || 'Modpack'
    modpackInstanceName.value = projTitle.replace(/[/\\?%*:|"<>]/g, '_').trim()
    modpackInstallDialogOpen.value = true
    return
  }

  selectedTargetFile.value = file
  dependencies.value = []
  selectedDepFiles.value = {}

  const kind = compStore.currentProject?.compType || 'mod'
  const sessionTarget = sessionTargetByKind[kind]
  if (sessionTarget && instancesStore.instances.some((i) => i.id === sessionTarget)) {
    selectedTargetInstance.value = sessionTarget
  } else {
    const instId = route.query.instance as string
    if (instId && instancesStore.instances.some((i) => i.id === instId)) {
      selectedTargetInstance.value = instId
    } else if (instancesStore.instances.length > 0 && instancesStore.instances[0]?.id) {
      selectedTargetInstance.value = instancesStore.instances[0].id
    }
  }

  downloadDialogOpen.value = true
  resolveDependenciesForSelectedInstance()
}

async function resolveDependenciesForSelectedInstance() {
  if (!selectedTargetFile.value || compStore.currentProject?.compType !== 'mod') {
    return
  }
  const inst = instancesStore.instances.find((i) => i.id === selectedTargetInstance.value)
  if (!inst) return

  resolvingDependencies.value = true
  try {
    const resolved = await ipcClient.compResolveDependencies(
      selectedTargetFile.value,
      inst.id,
      inst.mcVersion,
      inst.loaders?.[0] || 'fabric',
    )
    dependencies.value = resolved
    selectedDepFiles.value = {}
    for (const dep of resolved) {
      if (!dep.installed) {
        selectedDepFiles.value[dep.file.id] = (dep as { required?: boolean }).required ?? true
      }
    }
  } catch (err) {
    console.warn('Failed to resolve dependencies', err)
  } finally {
    resolvingDependencies.value = false
  }
}

async function startDownload() {
  if (!selectedTargetFile.value || !selectedTargetInstance.value) return

  const uninstalledRequired = dependencies.value.filter((d) => d.required && !d.installed)
  if (uninstalledRequired.length > 0) {
    const unselectedRequired = uninstalledRequired.filter((d) => !selectedDepFiles.value[d.file.id])
    if (unselectedRequired.length > 0) {
      downloadDialogOpen.value = false
      depConfirmDialogOpen.value = true
      return
    }
  }

  await doDownloadFiles(true)
}

async function doDownloadFiles(withSelectedDeps = true) {
  if (!selectedTargetFile.value || !selectedTargetInstance.value) return

  downloading.value = true
  const kind = compStore.currentProject?.compType || 'mod'
  sessionTargetByKind[kind] = selectedTargetInstance.value

  try {
    const filesToDownload: CompFile[] = [selectedTargetFile.value]
    if (withSelectedDeps) {
      for (const dep of dependencies.value) {
        if (!dep.installed && selectedDepFiles.value[dep.file.id]) {
          filesToDownload.push(dep.file)
        }
      }
    }

    const downloadedNames: string[] = []
    for (const file of filesToDownload) {
      await compStore.downloadFile(
        file,
        selectedTargetInstance.value,
        compStore.currentProject?.compType || 'mod',
      )
      downloadedNames.push(file.fileName)
    }

    toast.success(
      t('comp.downloadDialog.success', {
        name: selectedTargetFile.value.fileName,
        instance:
          instancesStore.instances.find((i) => i.id === selectedTargetInstance.value)?.name ||
          selectedTargetInstance.value,
      }),
    )

    downloadDialogOpen.value = false
    depConfirmDialogOpen.value = false
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : t('comp.downloadDialog.failed')
    toast.danger(msg)
  } finally {
    downloading.value = false
  }
}

async function startModpackInstall() {
  if (!modpackTargetFile.value || !modpackInstanceName.value.trim()) return

  installingModpack.value = true
  try {
    const taskId = await ipcClient.modpackInstallRemote(
      modpackTargetFile.value,
      modpackInstanceName.value.trim(),
    )
    modpackInstallDialogOpen.value = false
    // Queued, not finished: unpack + loader + base game still run in the task.
    const outcome = await awaitTask(taskId)
    if (outcome.state === 'failed' || outcome.state === 'canceled') {
      toast.danger(formatError(outcome.error) || t('comp.modpackInstallDialog.failed'))
      return
    }
    if (outcome.state === 'success') {
      await instancesStore.fetchInstances(true)
    }
    toast.success(
      t('comp.modpackInstallDialog.success', { name: modpackInstanceName.value.trim() }),
    )
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : t('comp.modpackInstallDialog.failed')
    toast.danger(msg)
  } finally {
    installingModpack.value = false
  }
}

onMounted(async () => {
  if (instancesStore.instances.length === 0) {
    await instancesStore.fetchInstances()
  }
  await compStore.loadFavorites()
  if (platform.value && projectId.value) {
    await compStore.openProject(platform.value, projectId.value)
  }
})
</script>

<template>
  <div class="m-page">
    <!-- Back Navigation -->
    <div class="flex items-center space-x-3">
      <MIconButton icon="i-lucide-arrow-left" size="sm" @click="router.back()" />
      <span class="text-sm font-semibold text-fg-muted">
        {{ t('comp.nav') }}
      </span>
    </div>

    <!-- Loading State -->
    <div v-if="compStore.loading" class="space-y-6">
      <MCard class="p-6">
        <div class="flex flex-col md:flex-row md:items-center justify-between gap-4">
          <div class="flex items-start space-x-4">
            <MSkeleton class="w-20 h-20 rounded-2xl shrink-0" />
            <div class="space-y-2">
              <div class="flex items-center space-x-3">
                <MSkeleton class="h-7 w-48" />
                <MSkeleton class="h-5 w-16 rounded-full" />
                <MSkeleton class="h-5 w-16 rounded-full" />
                <MSkeleton class="h-5 w-16 rounded-full" />
              </div>
              <MSkeleton class="h-4 w-32" />
              <div class="flex items-center gap-3 mt-2">
                <MSkeleton class="h-3.5 w-20 shrink-0" />
                <MSkeleton class="h-3.5 w-20 shrink-0" />
                <MSkeleton class="h-3.5 w-28 shrink-0" />
              </div>
            </div>
          </div>
          <div class="flex items-center space-x-3 self-end md:self-auto">
            <MSkeleton class="h-9 w-24 rounded-lg" />
            <MSkeleton class="h-9 w-24 rounded-lg" />
          </div>
        </div>
      </MCard>

      <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <div class="lg:col-span-2 space-y-4">
          <MSkeleton class="h-5 w-40" />
          <MCard class="p-6 space-y-3">
            <MSkeleton class="h-4 w-full" />
            <MSkeleton class="h-4 w-[90%]" />
            <MSkeleton class="h-4 w-[95%]" />
            <MSkeleton class="h-4 w-[60%]" />
          </MCard>
        </div>

        <div class="space-y-4">
          <MSkeleton class="h-5 w-24" />
          <div class="flex items-center space-x-2">
            <MSkeleton class="h-9 flex-1 rounded-lg" />
            <MSkeleton class="h-9 flex-1 rounded-lg" />
          </div>
          <div class="space-y-3">
            <MCard v-for="n in 4" :key="n" class="p-4 space-y-3">
              <div class="flex items-center justify-between">
                <div class="flex items-center space-x-2">
                  <MSkeleton class="h-5 w-16 rounded-full" />
                  <MSkeleton class="h-4 w-40" />
                </div>
                <MSkeleton class="h-3 w-16" />
              </div>
              <div class="flex items-center space-x-1">
                <MSkeleton class="h-5 w-16 rounded-full" />
                <MSkeleton class="h-5 w-16 rounded-full" />
              </div>
              <div class="pt-2 border-t border-line flex items-center justify-end">
                <MSkeleton class="h-8 w-20 rounded-lg" />
              </div>
            </MCard>
          </div>
        </div>
      </div>
    </div>

    <!-- Project Content -->
    <div v-else-if="compStore.currentProject" class="space-y-6">
      <CompProjectHeader
        :project="compStore.currentProject"
        :is-favorite="
          compStore.isFavorite(compStore.currentProject.platform, compStore.currentProject.id)
        "
        @toggle-favorite="handleToggleFavorite"
        @open-website="openExternalUrl"
      />

      <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <!-- Description Column -->
        <div class="lg:col-span-2 space-y-4">
          <h3 class="text-lg font-bold">{{ t('comp.project.description') }}</h3>
          <MCard class="p-6">
            <MMarkdown
              v-if="compStore.currentProject?.description"
              :source="compStore.currentProject.description"
              :format="compStore.currentProject.platform === 'curseforge' ? 'html' : 'markdown'"
            />
            <div v-else class="text-fg-muted py-8 text-center text-sm">
              {{ t('comp.project.noDescription') }}
            </div>
          </MCard>
        </div>

        <!-- Files Column -->
        <CompVersionsTab
          v-model:selected-mc-version="selectedMcVersion"
          v-model:selected-loader="selectedLoader"
          :files-loading="compStore.filesLoading"
          :filtered-files-count="filteredFiles.length"
          :file-groups="fileGroups"
          :mc-version-filter-options="mcVersionFilterOptions"
          :loader-filter-options="loaderFilterOptions"
          @download="openDownloadDialog"
          @open-website="openExternalUrl(compStore.currentProject?.websiteUrl)"
        />
      </div>
    </div>

    <CompDependencyDialogs
      v-model:download-dialog-open="downloadDialogOpen"
      v-model:modpack-install-dialog-open="modpackInstallDialogOpen"
      v-model:resolve-failed-dialog-open="resolveFailedDialogOpen"
      v-model:cannot-install-dialog-open="cannotInstallDialogOpen"
      v-model:dep-confirm-dialog-open="depConfirmDialogOpen"
      v-model:selected-target-instance="selectedTargetInstance"
      v-model:selected-dep-files="selectedDepFiles"
      v-model:modpack-instance-name="modpackInstanceName"
      :instances-count="instancesStore.instances.length"
      :instance-options="instanceOptions"
      :comp-type="compStore.currentProject?.compType"
      :resolving-dependencies="resolvingDependencies"
      :dependencies="dependencies"
      :downloading="downloading"
      :installing-modpack="installingModpack"
      @resolve-dependencies="resolveDependenciesForSelectedInstance"
      @start-download="startDownload"
      @start-modpack-install="startModpackInstall"
      @do-download-files="doDownloadFiles"
    />
  </div>
</template>

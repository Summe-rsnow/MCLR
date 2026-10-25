<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import MarkdownIt from 'markdown-it'
import DOMPurify from 'dompurify'
import { useCompStore } from './store'
import { useInstancesStore } from '../instances/store'
import {
  ipcClient,
  type CompPlatform,
  type CompProject,
  type CompFile,
  type CompType,
  type ResolvedDependencyItem,
} from '../../ipc/client'
import {
  MButton,
  MIconButton,
  MCard,
  MBadge,
  MDialog,
  MCollapse,
  MSelect,
  MSkeleton,
  MEmpty,
  MCheckbox,
  MInput,
  MBrandIcon,
  compTypeMeta,
  type CompTypeKey,
  useToast,
} from '../../ui'
const route = useRoute()
const router = useRouter()
const { t } = useI18n()
const toast = useToast()
const compStore = useCompStore()
const instancesStore = useInstancesStore()
async function handleToggleFavorite(project: CompProject) {
  const isFav = compStore.isFavorite(project.platform, project.id)
  await compStore.toggleFavorite(project)
  if (!isFav) {
    toast.success(t('comp.favorite.added'))
  } else {
    toast.info(t('comp.favorite.removed'))
  }
}

const md = new MarkdownIt({
  html: false,
  linkify: true,
  breaks: true,
})

const platform = computed(() => route.params.platform as CompPlatform)
const projectId = computed(() => route.params.id as string)

const selectedMcVersion = ref<string>('')
const selectedLoader = ref<string>('')

// Download Dialog state
const downloadDialogOpen = ref(false)
const selectedFile = ref<CompFile | null>(null)
const selectedTargetInstance = ref<string>('')
const downloading = ref(false)

// Dependencies state
const dependencies = ref<ResolvedDependencyItem[]>([])
const selectedDepFiles = ref<Record<string, boolean>>({})
const resolvingDependencies = ref(false)

const resolveFailed = ref(false)
const depConfirmDialogOpen = ref(false)
const cannotInstallDialogOpen = ref(false)
const resolveFailedDialogOpen = ref(false)
const sessionTargetByKind: Record<string, string> = {}
// Modpack Install Dialog state
const modpackInstallDialogOpen = ref(false)
const modpackTargetFile = ref<CompFile | null>(null)
const modpackInstanceName = ref('')
const installingModpack = ref(false)

const renderedDescription = computed(() => {
  const desc = compStore.currentProject?.description
  if (!desc) return ''
  const rawHtml = md.render(desc)
  return DOMPurify.sanitize(rawHtml)
})

const filteredFiles = computed(() => {
  let files = compStore.projectFiles
  if (selectedMcVersion.value) {
    files = files.filter((f) => f.mcVersions.includes(selectedMcVersion.value))
  }
  if (selectedLoader.value) {
    files = files.filter((f) => f.loaders.includes(selectedLoader.value))
  }
  return files
})

const mcVersionFilterOptions = computed(() => {
  const versions = new Set<string>()
  for (const f of compStore.projectFiles) {
    for (const v of f.mcVersions) {
      versions.add(v)
    }
  }
  return [
    { label: t('comp.search.allMcVersions'), value: '' },
    ...Array.from(versions).map((v) => ({ label: v, value: v })),
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

function formatFileSize(bytes: number): string {
  if (bytes >= 1024 * 1024) {
    return (bytes / (1024 * 1024)).toFixed(2) + ' MB'
  }
  if (bytes >= 1024) {
    return (bytes / 1024).toFixed(1) + ' KB'
  }
  return bytes + ' B'
}

function formatDate(dateStr: string): string {
  try {
    const d = new Date(dateStr)
    return d.toLocaleDateString()
  } catch {
    return dateStr
  }
}

function getChannelVariant(channel: string): 'primary' | 'warning' | 'danger' {
  if (channel === 'release') return 'primary'
  if (channel === 'beta') return 'warning'
  return 'danger'
}

async function resolveDependenciesForSelectedInstance(): Promise<void> {
  if (!selectedFile.value || !selectedTargetInstance.value) {
    dependencies.value = []
    selectedDepFiles.value = {}
    return
  }
  if (compStore.currentProject?.compType !== 'mod') {
    dependencies.value = []
    selectedDepFiles.value = {}
    return
  }
  const targetInst = instancesStore.instances.find(
    (i) => i.id === selectedTargetInstance.value,
  )
  const mcVer = targetInst?.mcVersion || selectedFile.value.mcVersions[0] || '1.20.1'
  const loader = targetInst?.loaders[0] || selectedFile.value.loaders[0] || 'fabric'

  resolvingDependencies.value = true
  try {
    const res = await ipcClient.compResolveDependencies(
      selectedFile.value,
      selectedTargetInstance.value,
      mcVer,
      loader,
    )
    dependencies.value = res
    const initialChecks: Record<string, boolean> = {}
    for (const item of res) {
      if (!item.installed) {
        initialChecks[item.file.id] = true
      }
    }
    selectedDepFiles.value = initialChecks
  } catch (err) {
    console.error('Failed to resolve dependencies', err)
    dependencies.value = []
    resolveFailed.value = true
  } finally {
    resolvingDependencies.value = false
  }
}

async function openDownloadDialog(file: CompFile): Promise<void> {
  selectedFile.value = file
  if (compStore.currentProject?.compType === 'modpack') {
    modpackTargetFile.value = file
    modpackInstanceName.value = compStore.currentProject.title || file.name.replace(/\.[^/.]+$/, '')
    modpackInstallDialogOpen.value = true
    return
  }

  const pType: CompType = compStore.currentProject?.compType || 'mod'
  const preferred = sessionTargetByKind[pType] || (route.query.instance as string) || null

  try {
    const pickRes = await ipcClient.compPickTarget(
      pType,
      preferred,
      file.mcVersions,
      file.loaders,
    )
    if (pickRes.instanceId) {
      selectedTargetInstance.value = pickRes.instanceId
      sessionTargetByKind[pType] = pickRes.instanceId
    } else if (instancesStore.instances.length > 0) {
      const match = instancesStore.instances.find((i) =>
        file.mcVersions.includes(i.mcVersion),
      )
      selectedTargetInstance.value = match ? match.id : instancesStore.instances[0].id
    } else {
      selectedTargetInstance.value = ''
    }
  } catch {
    selectedTargetInstance.value = ''
  }

  downloadDialogOpen.value = true
  await resolveDependenciesForSelectedInstance()
}

async function startDownload(): Promise<void> {
  if (!selectedFile.value || !selectedTargetInstance.value) return

  if (compStore.currentProject?.compType === 'mod') {
    if (resolveFailed.value) {
      resolveFailedDialogOpen.value = true
      return
    }

    const uninstalled = dependencies.value.filter(
      (d) => !d.installed,
    )
    if (uninstalled.length > 0) {
      const cannotInstall = uninstalled.some(
        (d) => d.file.distributionDisabled || d.file.downloadUrls.length === 0,
      )
      if (cannotInstall) {
        cannotInstallDialogOpen.value = true
        return
      }
      depConfirmDialogOpen.value = true
      return
    }
  }

  await doDownloadFiles(true)
}

async function doDownloadFiles(withDeps = true): Promise<void> {
  if (!selectedFile.value || !selectedTargetInstance.value) return
  downloading.value = true
  depConfirmDialogOpen.value = false
  cannotInstallDialogOpen.value = false
  resolveFailedDialogOpen.value = false

  try {
    const targetKind: CompType = compStore.currentProject?.compType || 'mod'
    await compStore.downloadFile(
      selectedFile.value,
      selectedTargetInstance.value,
      targetKind,
    )

    if (withDeps) {
      for (const dep of dependencies.value) {
        if (selectedDepFiles.value[dep.file.id]) {
          await compStore.downloadFile(
            dep.file,
            selectedTargetInstance.value,
            'mod',
          )
        }
      }
    }

    toast.show({
      message: t('comp.downloadDialog.success'),
      type: 'success',
    })
    downloadDialogOpen.value = false
  } catch (err: unknown) {
    toast.show({
      message: `${t('comp.downloadDialog.failed')}: ${err instanceof Error ? err.message : String(err)}`,
      type: 'danger',
    })
  } finally {
    downloading.value = false
  }
}

interface FileGroup {
  id: string
  title: string
  open: boolean
  files: CompFile[]
}

const fileGroups = ref<FileGroup[]>([])

watch(
  [filteredFiles, selectedTargetInstance, () => route.query.instance],
  () => {
    const all = filteredFiles.value
    if (all.length === 0) {
      fileGroups.value = []
      return
    }

    const targetInstId = selectedTargetInstance.value || (route.query.instance as string)
    const targetInst = instancesStore.instances.find((i) => i.id === targetInstId)

    const groups: FileGroup[] = []

    if (targetInst) {
      const mc = targetInst.mcVersion
      const loader = targetInst.loaders[0] || ''
      const pinned = all.filter(
        (f) =>
          f.mcVersions.includes(mc) &&
          (f.loaders.length === 0 || !loader || f.loaders.some((l) => l.toLowerCase() === loader.toLowerCase())),
      )
      if (pinned.length > 0) {
        groups.push({
          id: 'pinned',
          title: t('comp.files.selectedVersionGroup', {
            loader: loader ? loader.toUpperCase() : '',
            mc,
          }),
          open: true,
          files: pinned,
        })
      }
    }

    const pinnedIds = new Set(groups.flatMap((g) => g.files.map((f) => f.id)))
    const remaining = all.filter((f) => !pinnedIds.has(f.id))

    const byVer = new Map<string, CompFile[]>()
    for (const f of remaining) {
      const v = f.mcVersions[0] || t('comp.files.otherVersions')
      if (!byVer.has(v)) byVer.set(v, [])
      byVer.get(v)!.push(f)
    }

    let isFirst = groups.length === 0
    for (const [ver, files] of byVer.entries()) {
      groups.push({
        id: ver,
        title: ver,
        open: isFirst,
        files,
      })
      isFirst = false
    }

    fileGroups.value = groups
  },
  { immediate: true },
)

async function startModpackInstall(): Promise<void> {
  if (!modpackTargetFile.value || !modpackInstanceName.value.trim()) return
  installingModpack.value = true
  try {
    await ipcClient.modpackInstallRemote(
      modpackTargetFile.value,
      modpackInstanceName.value.trim(),
    )
    toast.show({
      message: t('comp.modpackInstallDialog.success'),
      type: 'success',
    })
    modpackInstallDialogOpen.value = false
  } catch (err: unknown) {
    toast.show({
      message: `${t('comp.modpackInstallDialog.failed')}: ${err instanceof Error ? err.message : String(err)}`,
      type: 'danger',
    })
  } finally {
    installingModpack.value = false
  }
}

function openExternalUrl(url?: string | null): void {
  if (url) {
    window.open(url, '_blank')
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
      <MIconButton
        icon="i-lucide-arrow-left"
        size="sm"
        @click="router.back()"
      />
      <span class="text-sm font-semibold text-fg-muted">
        {{ t('comp.nav') }}
      </span>
    </div>
    <!-- Loading State -->
    <div v-if="compStore.loading" class="space-y-4">
      <MSkeleton class="h-28 rounded-xl" />
      <MSkeleton class="h-64 rounded-xl" />
    </div>

    <!-- Project Content -->
    <div v-else-if="compStore.currentProject" class="space-y-6">
      <!-- Header Card -->
      <MCard class="p-6">
        <div class="flex flex-col md:flex-row md:items-center justify-between gap-4">
          <div class="flex items-start space-x-4">
            <img
              v-if="compStore.currentProject.iconUrl"
              :src="compStore.currentProject.iconUrl"
              class="w-20 h-20 rounded-xl object-cover bg-sunken shrink-0"
              alt=""
             loading="lazy" decoding="async"/>
            <div
              v-else
              class="w-20 h-20 rounded-xl bg-primary-soft text-primary-soft-fg flex items-center justify-center shrink-0"
            >
              <span :class="compTypeMeta[compStore.currentProject.compType as CompTypeKey]?.icon || 'i-lucide-box'" class="w-10 h-10 inline-block" />
            </div>

            <div>
              <div class="flex items-center space-x-3">
                <h2 class="text-2xl font-bold text-fg">{{ compStore.currentProject.title }}</h2>
                <MBadge variant="neutral">
                  <MBrandIcon v-if="compStore.currentProject.platform === 'modrinth' || compStore.currentProject.platform === 'curseforge'" :name="compStore.currentProject.platform" class="w-3.5 h-3.5 mr-1" />
                  {{ compStore.currentProject.platform }}
                </MBadge>
                <MBadge variant="neutral">{{ compStore.currentProject.compType }}</MBadge>
              </div>

              <div class="text-sm text-fg-muted mt-1">
                {{ t('comp.project.author', { author: compStore.currentProject.author }) }}
              </div>

              <div class="flex items-center space-x-4 mt-2 text-xs text-fg-muted">
                <span>{{ t('comp.project.downloads', { count: compStore.currentProject.downloads.toString() }) }}</span>
                <span>{{ t('comp.project.follows', { count: compStore.currentProject.follows.toString() }) }}</span>
                <span>{{ t('comp.project.updated', { date: formatDate(compStore.currentProject.updatedAt) }) }}</span>
              </div>
            </div>
          </div>
          <div class="flex items-center space-x-3 self-end md:self-auto">
            <MButton
              v-if="compStore.currentProject.websiteUrl"
              variant="tonal"
              @click="openExternalUrl(compStore.currentProject.websiteUrl)"
            >
              <i class="i-lucide-external-link mr-1.5" />
              {{ t('comp.project.website') }}
            </MButton>

            <MIconButton
              :icon="
                compStore.isFavorite(compStore.currentProject.platform, compStore.currentProject.id)
                  ? 'i-lucide-heart fill-current text-rose-500'
                  : 'i-lucide-heart text-fg-muted hover:text-rose-500'
              "
              :class="
                compStore.isFavorite(compStore.currentProject.platform, compStore.currentProject.id)
                  ? 'i-lucide-heart fill-current text-rose-500'
                  : 'i-lucide-heart text-fg-muted hover:text-rose-500'
              "
              @click="handleToggleFavorite(compStore.currentProject)"
            />
          </div>
        </div>
      </MCard>

      <!-- Main Layout: Description + Files -->
      <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <!-- Description Column -->
        <div class="lg:col-span-2 space-y-4">
          <h3 class="text-lg font-bold">{{ t('comp.project.description') }}</h3>
          <MCard class="p-6">
            <div
              v-if="renderedDescription"
              class="prose prose-sm dark:prose-invert max-w-none break-words"
              v-html="renderedDescription"
            />
            <div v-else class="text-fg-muted py-8 text-center text-sm">
              {{ t('comp.project.noDescription') }}
            </div>
          </MCard>
        </div>

        <!-- Files Column -->
        <div class="space-y-4">
          <div class="flex items-center justify-between">
            <h3 class="text-lg font-bold">{{ t('comp.project.files') }}</h3>
          </div>

          <!-- Filters -->
          <div class="flex items-center space-x-2">
            <div class="flex-1">
              <MSelect
                v-model="selectedMcVersion"
                :options="mcVersionFilterOptions"
              />
            </div>
            <div class="flex-1">
              <MSelect
                v-model="selectedLoader"
                :options="loaderFilterOptions"
              />
            </div>
          </div>

          <!-- Files List -->
          <div class="space-y-3">
            <div v-if="compStore.filesLoading" class="space-y-2 py-4">
              <MSkeleton v-for="n in 3" :key="n" class="h-20 rounded-xl" />
            </div>

            <div
              v-else-if="filteredFiles.length === 0"
              class="py-12"
            >
              <MEmpty :title="t('comp.project.noFiles')" />
            </div>

            <div v-else class="space-y-3">
              <MCard
                v-for="group in fileGroups"
                :key="group.id"
                class="overflow-hidden"
              >
                <button
                  class="w-full p-3.5 flex items-center justify-between text-left font-medium text-xs text-fg hover:bg-sunken/40 transition"
                  @click="group.open = !group.open"
                >
                  <div class="flex items-center space-x-2">
                    <span
                      class="i-lucide-chevron-right w-4 h-4 text-fg-muted transition-transform duration-200"
                      :class="{ 'rotate-90': group.open }"
                    />
                    <span>{{ group.title }}</span>
                    <MBadge variant="neutral" size="sm">{{ group.files.length }}</MBadge>
                  </div>
                </button>
                <MCollapse :open="group.open">
                  <div class="p-3.5 pt-0 space-y-3">
                    <MCard
                      v-for="file in group.files"
                      :key="file.id"
                      variant="well"
                      hoverable
                      class="p-4 space-y-3"
                    >
                      <div>
                        <div class="flex items-center space-x-2">
                          <MBadge :variant="getChannelVariant(file.channel)">
                            {{ t(`comp.files.channel.${file.channel}`) }}
                          </MBadge>
                          <span class="font-medium text-sm truncate max-w-[200px]" :title="file.name">
                            {{ file.name }}
                          </span>
                        </div>
                        <div class="text-xs text-fg-muted mt-1">
                          {{ formatDate(file.releaseDate) }} • {{ formatFileSize(file.size) }}
                        </div>
                      </div>

                      <div class="flex flex-wrap gap-1">
                        <MBadge
                          v-for="v in file.mcVersions"
                          :key="v"
                          variant="neutral"
                        >
                          {{ v }}
                        </MBadge>
                        <MBadge
                          v-for="l in file.loaders"
                          :key="l"
                          variant="neutral"
                        >
                          {{ l }}
                        </MBadge>
                      </div>

                      <div class="pt-2 border-t border-line flex items-center justify-end">
                        <div v-if="file.distributionDisabled" class="flex items-center space-x-2">
                          <span class="text-xs text-danger">{{ t('comp.files.distributionDisabled') }}</span>
                          <MButton
                            variant="tonal"
                            size="sm"
                            @click="openExternalUrl(compStore.currentProject?.websiteUrl)"
                          >
                            {{ t('comp.files.openWebsite') }}
                          </MButton>
                        </div>
                        <MButton
                          v-else
                          variant="primary"
                          size="sm"
                          @click="openDownloadDialog(file)"
                        >
                          <span class="i-lucide-download mr-1 w-3.5 h-3.5" />
                          {{ t('comp.files.download') }}
                        </MButton>
                      </div>
                    </MCard>
                  </div>
                </MCollapse>
              </MCard>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Download Instance Selection Dialog -->
    <MDialog
      :open="downloadDialogOpen"
      :title="t('comp.downloadDialog.title')"
      @update:open="downloadDialogOpen = $event"
      @cancel="downloadDialogOpen = false"
    >
      <div class="space-y-4 py-2">
        <div v-if="instancesStore.instances.length === 0" class="text-sm text-danger">
          {{ t('comp.downloadDialog.noInstances') }}
        </div>
        <div v-else class="space-y-3">
          <label class="text-xs font-semibold text-fg-muted block">
            {{ t('comp.downloadDialog.selectInstance') }}
          </label>
          <MSelect
            v-model="selectedTargetInstance"
            :options="instanceOptions"
            @update:model-value="resolveDependenciesForSelectedInstance"
          />
          <div class="text-xs text-fg-muted">
            {{ t('comp.downloadDialog.targetSubfolder', { folder: compStore.currentProject?.compType || 'mods' }) }}
          </div>

          <!-- Dependencies Checklist -->
          <div v-if="compStore.currentProject?.compType === 'mod' && (resolvingDependencies || dependencies.length > 0)" class="pt-2 border-t border-line space-y-2">
            <div class="text-xs font-semibold text-fg">
              {{ t('comp.downloadDialog.dependenciesTitle') }}
            </div>
            <div class="text-[11px] text-fg-muted">
              {{ t('comp.downloadDialog.dependenciesDesc') }}
            </div>

            <div v-if="resolvingDependencies" class="space-y-1.5 py-1">
              <MSkeleton class="h-6 rounded" />
              <MSkeleton class="h-6 rounded" />
            </div>

            <div v-else class="space-y-1.5 max-h-40 overflow-y-auto">
              <div
                v-for="dep in dependencies"
                :key="dep.file.id"
                class="flex items-center justify-between p-2 rounded-lg bg-sunken/30 text-xs"
              >
                <div class="flex items-center space-x-2 min-w-0">
                  <MCheckbox
                    v-if="!dep.installed"
                    v-model="selectedDepFiles[dep.file.id]"
                  />
                  <span v-else class="i-lucide-check text-success shrink-0 w-4 h-4 ml-0.5" />
                  <span class="font-medium text-fg truncate">
                    {{ dep.project.title }}
                  </span>
                  <span class="text-fg-muted font-mono text-[11px]">v{{ dep.file.name }}</span>
                </div>

                <div class="shrink-0">
                  <MBadge
                    :variant="dep.installed ? 'success' : 'neutral'"
                    size="sm"
                  >
                    {{ dep.installed ? t('comp.downloadDialog.depInstalled') : t('comp.downloadDialog.depRequired') }}
                  </MBadge>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <template #footer>
        <div class="flex items-center justify-end space-x-3">
          <MButton
            variant="text"
            @click="downloadDialogOpen = false"
          >
            {{ t('comp.downloadDialog.cancel') }}
          </MButton>
          <MButton
            variant="primary"
            :loading="downloading"
            :disabled="instancesStore.instances.length === 0 || !selectedTargetInstance"
            @click="startDownload"
          >
            {{ t('comp.downloadDialog.confirm') }}
          </MButton>
        </div>
      </template>
    </MDialog>

    <!-- Modpack Install Dialog -->
    <MDialog
      :open="modpackInstallDialogOpen"
      :title="t('comp.modpackInstallDialog.title')"
      @update:open="modpackInstallDialogOpen = $event"
      @cancel="modpackInstallDialogOpen = false"
    >
      <div class="space-y-4 py-2">
        <div class="space-y-2">
          <label class="text-xs font-semibold text-fg-muted block">
            {{ t('comp.modpackInstallDialog.instanceName') }}
          </label>
          <MInput
            v-model="modpackInstanceName"
            :placeholder="t('comp.modpackInstallDialog.instanceNamePlaceholder')"
          />
        </div>
      </div>

      <template #footer>
        <div class="flex items-center justify-end space-x-3">
          <MButton
            variant="text"
            @click="modpackInstallDialogOpen = false"
          >
            {{ t('comp.downloadDialog.cancel') }}
          </MButton>
          <MButton
            variant="primary"
            :loading="installingModpack"
            :disabled="!modpackInstanceName.trim()"
            @click="startModpackInstall"
          >
            {{ t('comp.modpackInstallDialog.install') }}
          </MButton>
        </div>
      </template>
    </MDialog>

    <!-- Resolve Failed Dialog -->
    <MDialog
      :open="resolveFailedDialogOpen"
      :title="t('comp.dependency.resolveFailedTitle')"
      @update:open="resolveFailedDialogOpen = $event"
      @cancel="resolveFailedDialogOpen = false"
    >
      <div class="py-2 text-xs text-fg">
        {{ t('comp.dependency.resolveFailedTitle') }}
      </div>
      <template #footer>
        <div class="flex items-center justify-end space-x-3">
          <MButton variant="text" @click="resolveFailedDialogOpen = false">
            {{ t('comp.downloadDialog.cancel') }}
          </MButton>
          <MButton variant="primary" @click="doDownloadFiles(false)">
            {{ t('comp.dependency.continueDownload') }}
          </MButton>
        </div>
      </template>
    </MDialog>

    <!-- Cannot Install Required Dependencies Dialog -->
    <MDialog
      :open="cannotInstallDialogOpen"
      :title="t('comp.dependency.cannotInstallTitle')"
      @update:open="cannotInstallDialogOpen = $event"
      @cancel="cannotInstallDialogOpen = false"
    >
      <div class="py-2 text-xs text-fg">
        {{ t('comp.dependency.cannotInstallTitle') }}
      </div>
      <template #footer>
        <div class="flex items-center justify-end space-x-3">
          <MButton variant="text" @click="cannotInstallDialogOpen = false">
            {{ t('comp.downloadDialog.cancel') }}
          </MButton>
          <MButton variant="primary" @click="doDownloadFiles(false)">
            {{ t('comp.dependency.continueDownload') }}
          </MButton>
        </div>
      </template>
    </MDialog>

    <!-- Confirm Dependencies Dialog -->
    <MDialog
      :open="depConfirmDialogOpen"
      :title="t('comp.dependency.confirmTitle')"
      @update:open="depConfirmDialogOpen = $event"
      @cancel="depConfirmDialogOpen = false"
    >
      <div class="py-2 text-xs text-fg">
        {{ t('comp.dependency.confirmTitle') }}
      </div>
      <template #footer>
        <div class="flex items-center justify-end space-x-2">
          <MButton variant="text" @click="depConfirmDialogOpen = false">
            {{ t('comp.downloadDialog.cancel') }}
          </MButton>
          <MButton variant="tonal" @click="doDownloadFiles(false)">
            {{ t('comp.dependency.installModOnly') }}
          </MButton>
          <MButton variant="primary" @click="doDownloadFiles(true)">
            {{ t('comp.dependency.installWithDeps') }}
          </MButton>
        </div>
      </template>
    </MDialog>
  </div>
</template>

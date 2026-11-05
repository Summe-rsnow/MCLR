import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { isLoaderSupported, fromLoaderKind } from '@/shared/loader'
import {
  ipcClient,
  type VersionManifestList,
  type VersionManifestEntry,
  type LoaderKind,
  type LoaderVersionEntry,
} from '../../ipc/client'
import { awaitTask } from '../tasks/awaitTask'
import { useInstancesStore } from '../instances/store'
import { usePresetsStore } from '../presets/store'
export type VersionFilterType = 'all' | 'release' | 'snapshot' | 'old'
export type SelectedLoader = 'none' | LoaderKind

export interface LoaderOption {
  kind: SelectedLoader
  labelKey: string
}

export const useInstallStore = defineStore('install', () => {
  const manifest = ref<VersionManifestList>({
    latestRelease: '',
    latestSnapshot: '',
    versions: [],
  })
  const loading = ref(false)
  const searchQuery = ref('')
  const pendingPresetIds = ref<string[]>([])
  const filterType = ref<VersionFilterType>('release')
  const selectedVersion = ref<VersionManifestEntry | null>(null)
  const instanceName = ref('')
  const installDialogOpen = ref(false)

  const selectedLoader = ref<SelectedLoader>('none')
  const loaderVersions = ref<LoaderVersionEntry[]>([])
  const selectedLoaderVersion = ref<string>('')
  const loadingLoaderVersions = ref(false)

  // OptiFine combo
  const optifineComboVersions = ref<LoaderVersionEntry[]>([])
  const selectedOptifineVersion = ref<string>('')
  const loadingOptifineVersions = ref(false)

  // Validation: ^[^\\/:*?"<>|!;]+$
  const nameRegex = /^[^\\/:*?"<>|!;]+$/

  const isNameValid = computed(() => {
    const trimmed = instanceName.value.trim()
    if (!trimmed) return false
    return nameRegex.test(trimmed)
  })

  const filteredVersions = computed(() => {
    let list = manifest.value.versions

    if (filterType.value === 'release') {
      list = list.filter((v) => v.versionType === 'release')
    } else if (filterType.value === 'snapshot') {
      list = list.filter((v) => v.versionType === 'snapshot')
    } else if (filterType.value === 'old') {
      list = list.filter((v) => v.versionType.startsWith('old_'))
    }

    const q = searchQuery.value.trim().toLowerCase()
    if (q) {
      list = list.filter((v) => v.id.toLowerCase().includes(q))
    }

    return list
  })
  const compatibleLoaders = computed<SelectedLoader[]>(() => {
    const mc = selectedVersion.value?.id || ''
    const allLoaders: SelectedLoader[] = [
      'none',
      'fabric',
      'forge',
      'neoForge',
      'optiFine',
      'legacyFabric',
      'liteLoader',
      'labyMod',
      'cleanroom',
    ]
    return allLoaders.filter(
      (loader) => loader === 'none' || isLoaderSupported(fromLoaderKind(loader), mc),
    )
  })

  async function fetchLoaderVersions(loader: SelectedLoader, mcVersion: string) {
    if (loader === 'none' || !mcVersion) {
      loaderVersions.value = []
      selectedLoaderVersion.value = ''
      return
    }
    loadingLoaderVersions.value = true
    try {
      const list = await ipcClient.loadersVersions(loader, mcVersion)
      loaderVersions.value = list
      if (list.length > 0) {
        const stable = list.find((v) => v.stable)
        selectedLoaderVersion.value = stable ? stable.version : list[0].version
      } else {
        selectedLoaderVersion.value = ''
      }
    } catch {
      loaderVersions.value = []
      selectedLoaderVersion.value = ''
    } finally {
      loadingLoaderVersions.value = false
    }
  }

  async function fetchOptifineComboVersions(mcVersion: string) {
    if (!mcVersion) {
      optifineComboVersions.value = []
      selectedOptifineVersion.value = ''
      return
    }
    loadingOptifineVersions.value = true
    try {
      const list = await ipcClient.loadersVersions('optiFine', mcVersion)
      optifineComboVersions.value = list
    } catch {
      optifineComboVersions.value = []
    } finally {
      loadingOptifineVersions.value = false
    }
  }

  async function selectLoader(loader: SelectedLoader) {
    selectedLoader.value = loader
    selectedOptifineVersion.value = ''
    const mc = selectedVersion.value?.id || ''
    await fetchLoaderVersions(loader, mc)
    if (loader === 'fabric' || loader === 'forge') {
      await fetchOptifineComboVersions(mc)
    } else {
      optifineComboVersions.value = []
    }
  }

  async function fetchManifest(forceRefresh = false) {
    loading.value = true
    try {
      manifest.value = await ipcClient.installManifest(forceRefresh)
    } finally {
      loading.value = false
    }
  }

  function openInstallDialog(v: VersionManifestEntry) {
    selectedVersion.value = v
    instanceName.value = v.id
    selectedLoader.value = 'none'
    loaderVersions.value = []
    selectedLoaderVersion.value = ''
    optifineComboVersions.value = []
    selectedOptifineVersion.value = ''
    installDialogOpen.value = true
    pendingPresetIds.value = []
  }

  async function install() {
    if (!selectedVersion.value || !isNameValid.value) return null
    let taskId: string
    if (selectedLoader.value === 'none') {
      taskId = await ipcClient.installVanilla(selectedVersion.value.id, instanceName.value.trim())
    } else {
      const optifine = selectedOptifineVersion.value ? selectedOptifineVersion.value : null
      taskId = await ipcClient.loadersInstall(
        selectedVersion.value.id,
        instanceName.value.trim(),
        selectedLoader.value,
        selectedLoaderVersion.value,
        optifine,
      )
    }
    installDialogOpen.value = false
    if (pendingPresetIds.value.length > 0) {
      const presetIdsToApply = [...pendingPresetIds.value]
      const targetName = instanceName.value.trim()
      awaitTask(taskId)
        .then(async () => {
          const instancesStore = useInstancesStore()
          await instancesStore.fetchInstances()
          const created = instancesStore.instances.find((i) => i.name === targetName)
          if (created) {
            const presetsStore = usePresetsStore()
            await presetsStore.applyPresets({
              instanceId: created.id,
              presetIds: presetIdsToApply,
              datapackWorld: null,
              includeDependencies: true,
              onOtherVersion: 'skip',
            })
          }
        })
        .catch(() => {})
    }
    return taskId
  }

  return {
    manifest,
    loading,
    searchQuery,
    filterType,
    selectedVersion,
    instanceName,
    installDialogOpen,
    isNameValid,
    filteredVersions,
    selectedLoader,
    loaderVersions,
    selectedLoaderVersion,
    loadingLoaderVersions,
    optifineComboVersions,
    selectedOptifineVersion,
    loadingOptifineVersions,
    compatibleLoaders,
    fetchManifest,
    fetchLoaderVersions,
    fetchOptifineComboVersions,
    selectLoader,
    openInstallDialog,
    install,
    pendingPresetIds,
  }
})

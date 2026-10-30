import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import {
  ipcClient,
  type VersionManifestList,
  type VersionManifestEntry,
  type LoaderKind,
  type LoaderVersionEntry,
} from '../../ipc/client'

export type VersionFilterType = 'all' | 'release' | 'snapshot' | 'old'
export type SelectedLoader = 'none' | LoaderKind

export interface LoaderOption {
  kind: SelectedLoader
  labelKey: string
}

export function compareMcVersions(a: string, b: string): number {
  const pa = a.split('.').map((p) => parseInt(p, 10) || 0)
  const pb = b.split('.').map((p) => parseInt(p, 10) || 0)
  const len = Math.max(pa.length, pb.length)
  for (let i = 0; i < len; i++) {
    const na = pa[i] ?? 0
    const nb = pb[i] ?? 0
    if (na !== nb) return na - nb
  }
  return 0
}

export function isLoaderCompatible(loader: SelectedLoader, mcVersion: string): boolean {
  if (loader === 'none') return true
  if (!mcVersion) return true

  switch (loader) {
    case 'cleanroom':
      // Cleanroom is strictly for 1.12.2
      return mcVersion === '1.12.2'
    case 'neoForge':
      // NeoForge started from 1.20.1 (>= 1.20.1)
      return compareMcVersions(mcVersion, '1.20.1') >= 0
    case 'legacyFabric':
      // LegacyFabric is for <= 1.13.2
      return compareMcVersions(mcVersion, '1.13.2') <= 0
    case 'liteLoader':
      // LiteLoader ended around 1.12.2 (<= 1.12.2)
      return compareMcVersions(mcVersion, '1.12.2') <= 0
    case 'fabric':
      // Fabric is >= 1.14
      return compareMcVersions(mcVersion, '1.14') >= 0
    case 'forge':
      // Forge exists up to ~1.20.x, not restricted here
      return true
    case 'optiFine':
      return true
    case 'labyMod':
      return compareMcVersions(mcVersion, '1.8') >= 0
    default:
      return true
  }
}

export const useInstallStore = defineStore('install', () => {
  const manifest = ref<VersionManifestList>({
    latestRelease: '',
    latestSnapshot: '',
    versions: [],
  })
  const loading = ref(false)
  const searchQuery = ref('')
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
    return allLoaders.filter((loader) => isLoaderCompatible(loader, mc))
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
  }
})

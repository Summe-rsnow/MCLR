import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import {
  ipcClient,
  type VersionManifestList,
  type VersionManifestEntry,
} from '../../ipc/client'

export type VersionFilterType = 'all' | 'release' | 'snapshot' | 'old'

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
    installDialogOpen.value = true
  }

  async function install() {
    if (!selectedVersion.value || !isNameValid.value) return null
    const taskId = await ipcClient.installVanilla(
      selectedVersion.value.id,
      instanceName.value.trim(),
    )
    installDialogOpen.value = false
    return taskId
  }

  async function repair(instanceId: string) {
    const taskId = await ipcClient.installRepair(instanceId)
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
    fetchManifest,
    openInstallDialog,
    install,
    repair,
  }
})

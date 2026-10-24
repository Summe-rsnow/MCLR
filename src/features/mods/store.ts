import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import {
  ipcClient,
  type LocalModInfo,
  type JarInJarNode,
  type CompFile,
} from '../../ipc/client'

export const useModsStore = defineStore('mods', () => {
  const selectedInstanceId = ref<string>('')
  const mods = ref<LocalModInfo[]>([])
  const loading = ref<boolean>(false)
  const searchQuery = ref<string>('')
  const filterState = ref<'all' | 'enabled' | 'disabled' | 'updatable'>('all')

  const jarInJarTarget = ref<LocalModInfo | null>(null)
  const jarInJarTree = ref<JarInJarNode[]>([])
  const jarInJarLoading = ref<boolean>(false)

  const filteredMods = computed(() => {
    let list = mods.value

    // Search query filter
    const query = searchQuery.value.trim().toLowerCase()
    if (query) {
      list = list.filter(
        (m) =>
          m.name.toLowerCase().includes(query) ||
          m.modId.toLowerCase().includes(query) ||
          m.fileName.toLowerCase().includes(query) ||
          m.authors.some((a) => a.toLowerCase().includes(query)) ||
          (m.description && m.description.toLowerCase().includes(query)),
      )
    }

    // State filter
    if (filterState.value === 'enabled') {
      list = list.filter((m) => m.enabled)
    } else if (filterState.value === 'disabled') {
      list = list.filter((m) => !m.enabled)
    } else if (filterState.value === 'updatable') {
      list = list.filter((m) => !!m.updateFile)
    }

    return list
  })

  async function loadMods(instanceId: string): Promise<void> {
    selectedInstanceId.value = instanceId
    if (!instanceId) {
      mods.value = []
      return
    }
    loading.value = true
    try {
      mods.value = await ipcClient.localModsList(instanceId)
    } finally {
      loading.value = false
    }
  }

  async function toggleMod(mod: LocalModInfo): Promise<void> {
    if (!selectedInstanceId.value) return
    const targetState = !mod.enabled
    const updated = await ipcClient.localModToggle(
      selectedInstanceId.value,
      mod.fileName,
      targetState,
    )
    Object.assign(mod, updated)
    const idx = mods.value.findIndex((m) => m.modId === mod.modId || m.fileName === mod.fileName)
    if (idx !== -1) {
      mods.value[idx] = updated
    }
  }

  async function deleteMod(mod: LocalModInfo): Promise<void> {
    if (!selectedInstanceId.value) return
    await ipcClient.localModDelete(selectedInstanceId.value, mod.fileName)
    mods.value = mods.value.filter((m) => m.fileName !== mod.fileName)
  }

  async function checkUpdates(mcVersion = '1.20.1', loader = 'fabric'): Promise<void> {
    if (!selectedInstanceId.value) return
    loading.value = true
    try {
      const updatedList = await ipcClient.localModsCheckUpdates(
        selectedInstanceId.value,
        mcVersion,
        loader,
      )
      mods.value = updatedList
    } finally {
      loading.value = false
    }
  }

  async function updateMod(mod: LocalModInfo, file?: CompFile): Promise<string | null> {
    if (!selectedInstanceId.value) return null
    const targetFile = file || mod.updateFile
    if (!targetFile) return null
    const taskId = await ipcClient.localModUpdate(
      selectedInstanceId.value,
      mod.fileName,
      targetFile,
    )
    mod.updateFile = null
    return taskId
  }

  async function updateAllMods(): Promise<number> {
    if (!selectedInstanceId.value) return 0
    let count = 0
    for (const mod of mods.value) {
      if (mod.updateFile) {
        await updateMod(mod)
        count++
      }
    }
    return count
  }

  async function openJarInJar(mod: LocalModInfo): Promise<void> {
    if (!selectedInstanceId.value) return
    jarInJarTarget.value = mod
    jarInJarLoading.value = true
    try {
      jarInJarTree.value = await ipcClient.localModJarInJar(
        selectedInstanceId.value,
        mod.fileName,
      )
    } finally {
      jarInJarLoading.value = false
    }
  }

  function closeJarInJar(): void {
    jarInJarTarget.value = null
    jarInJarTree.value = []
  }

  return {
    selectedInstanceId,
    mods,
    loading,
    searchQuery,
    filterState,
    filteredMods,
    jarInJarTarget,
    jarInJarTree,
    jarInJarLoading,
    loadMods,
    toggleMod,
    deleteMod,
    checkUpdates,
    updateMod,
    updateAllMods,
    openJarInJar,
    closeJarInJar,
  }
})

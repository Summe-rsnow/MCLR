import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import {
  ipcClient,
  type InstanceInfo,
  type GameFolderEntry,
} from '../../ipc/client'

export const useInstancesStore = defineStore('instances', () => {
  const instances = ref<InstanceInfo[]>([])
  const folders = ref<GameFolderEntry[]>([
    { name: 'Default', path: 'C:\\Users\\User\\AppData\\Roaming\\MCLR\\.minecraft' },
  ])
  const currentFolder = ref<string>('C:\\Users\\User\\AppData\\Roaming\\MCLR\\.minecraft')
  const loading = ref(false)
  const searchQuery = ref('')
  const filterType = ref<'all' | 'pinned' | 'vanilla' | 'modded'>('all')
  const sortBy = ref<'name' | 'version' | 'category'>('name')
  const filteredInstances = computed(() => {
    let result = instances.value

    // Search query filter
    const query = searchQuery.value.trim().toLowerCase()
    if (query) {
      result = result.filter(
        (i) =>
          i.name.toLowerCase().includes(query) ||
          i.mcVersion.toLowerCase().includes(query) ||
          i.loaders.some((l) => l.toLowerCase().includes(query)) ||
          i.kind.toLowerCase().includes(query) ||
          (i.config?.category && i.config.category.toLowerCase().includes(query)),
      )
    }

    // Type / tab filter
    if (filterType.value === 'pinned') {
      result = result.filter((i) => i.config?.pinned)
    } else if (filterType.value === 'vanilla') {
      result = result.filter((i) => i.kind === 'vanilla' || i.kind === 'snapshot' || i.kind === 'oldBeta' || i.kind === 'oldAlpha')
    } else if (filterType.value === 'modded') {
      result = result.filter((i) => i.kind !== 'vanilla' && i.kind !== 'snapshot' && i.kind !== 'oldBeta' && i.kind !== 'oldAlpha')
    }

    // Sort: pinned first, then according to sortBy
    return [...result].sort((a, b) => {
      const pinA = a.config?.pinned ? 1 : 0
      const pinB = b.config?.pinned ? 1 : 0
      if (pinA !== pinB) return pinB - pinA
      if (sortBy.value === 'version') {
        return b.mcVersion.localeCompare(a.mcVersion, undefined, { numeric: true })
      }
      if (sortBy.value === 'category') {
        const catA = a.config?.category || 'Default'
        const catB = b.config?.category || 'Default'
        const catCmp = catA.localeCompare(catB)
        if (catCmp !== 0) return catCmp
      }
      return a.name.localeCompare(b.name)
    })
  })

  // Grouped by pinned and then categories
  const groupedInstances = computed(() => {
    const pinned: InstanceInfo[] = []
    const categories = new Map<string, InstanceInfo[]>()

    for (const inst of filteredInstances.value) {
      if (filterType.value !== 'pinned' && inst.config?.pinned) {
        pinned.push(inst)
      } else {
        const cat = inst.config?.category || 'Default'
        if (!categories.has(cat)) {
          categories.set(cat, [])
        }
        categories.get(cat)!.push(inst)
      }
    }

    return {
      pinned,
      categories: Array.from(categories.entries()).map(([name, items]) => ({
        name,
        items,
      })),
    }
  })

  async function loadSettings() {
    try {
      const settings = await ipcClient.settingsGet()
      if (settings.game?.folders && settings.game.folders.length > 0) {
        folders.value = settings.game.folders
      }
      if (settings.game?.currentFolder) {
        currentFolder.value = settings.game.currentFolder
      }
      if (settings.game?.sortBy && ['name', 'version', 'category'].includes(settings.game.sortBy)) {
        sortBy.value = settings.game.sortBy as 'name' | 'version' | 'category'
      }
    } catch {
      // Fallback
    }
  }

  async function fetchInstances() {
    loading.value = true
    try {
      instances.value = await ipcClient.instancesList()
    } catch {
      // Fallback
    } finally {
      loading.value = false
    }
  }

  async function rename(id: string, newName: string) {
    const updated = await ipcClient.instanceRename(id, newName)
    const idx = instances.value.findIndex((i) => i.id === id)
    if (idx >= 0) {
      instances.value[idx] = updated
    }
  }

  async function duplicate(id: string, newName: string) {
    const copy = await ipcClient.instanceDuplicate(id, newName)
    instances.value.push(copy)
  }

  async function remove(id: string) {
    await ipcClient.instanceDelete(id)
    instances.value = instances.value.filter((i) => i.id !== id)
  }

  async function togglePin(id: string) {
    const inst = instances.value.find((i) => i.id === id)
    if (!inst) return
    const newPinned = !inst.config?.pinned
    const updated = await ipcClient.instanceUpdateConfig(id, {
      ...inst.config,
      pinned: newPinned,
    })
    const idx = instances.value.findIndex((i) => i.id === id)
    if (idx >= 0) {
      instances.value[idx] = updated
    }
  }

  async function openFolder(id: string) {
    await ipcClient.instanceOpenFolder(id)
  }

  async function switchFolder(path: string) {
    currentFolder.value = path
    try {
      await ipcClient.settingsPatch({
        game: {
          currentFolder: path,
        },
      })
    } catch {
      // Fallback
    }
    await fetchInstances()
  }

  async function addFolder(name: string, path: string) {
    const newEntry: GameFolderEntry = { name, path }
    folders.value.push(newEntry)
    currentFolder.value = path
    try {
      await ipcClient.settingsPatch({
        game: {
          folders: folders.value,
          currentFolder: path,
        },
      })
    } catch {
      // Fallback
    }
    await fetchInstances()
  }

  return {
    instances,
    folders,
    currentFolder,
    loading,
    searchQuery,
    filterType,
    sortBy,
    filteredInstances,
    groupedInstances,
    loadSettings,
    fetchInstances,
    rename,
    duplicate,
    remove,
    togglePin,
    openFolder,
    switchFolder,
    addFolder,
  }
})

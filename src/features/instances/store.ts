import { defineStore } from 'pinia'
import { ref, computed, watch } from 'vue'
import {
  ipcClient,
  type InstanceInfo,
  type GameFolderEntry,
  type CardKind,
  type InstanceConfig,
} from '../../ipc/client'
import { createResource } from '../../composables/useResource'

export type InstanceSort = 'name' | 'version' | 'category' | 'lastPlayed'

export interface InstanceCardGroup {
  key: CardKind
  titleKey: string
  items: InstanceInfo[]
}

function lastPlayedMs(instance: InstanceInfo): number {
  const raw = instance.config?.lastPlayedAt
  if (!raw) return 0
  const ms = Date.parse(raw)
  return Number.isNaN(ms) ? 0 : ms
}

/** Sorts a copy of the list; the backend order is the PCL card order, never a user choice. */
function sortInstances(list: InstanceInfo[], sort: InstanceSort): InstanceInfo[] {
  const copy = [...list]
  switch (sort) {
    case 'version':
      copy.sort((a, b) => b.mcVersion.localeCompare(a.mcVersion, undefined, { numeric: true }))
      break
    case 'category':
      copy.sort((a, b) => {
        const byCategory = (a.config?.category ?? '').localeCompare(b.config?.category ?? '')
        return byCategory !== 0 ? byCategory : a.name.localeCompare(b.name)
      })
      break
    case 'lastPlayed':
      copy.sort((a, b) => lastPlayedMs(b) - lastPlayedMs(a) || a.name.localeCompare(b.name))
      break
    case 'name':
    default:
      copy.sort((a, b) => a.name.localeCompare(b.name))
  }
  return copy
}

const CARD_GROUP_CONFIG: Array<{ key: CardKind; titleKey: string }> = [
  { key: 'star', titleKey: 'instances.groups.star' },
  { key: 'api', titleKey: 'instances.groups.api' },
  { key: 'regular', titleKey: 'instances.groups.regular' },
  { key: 'rubbish', titleKey: 'instances.groups.rubbish' },
  { key: 'fool', titleKey: 'instances.groups.fool' },
  { key: 'error', titleKey: 'instances.groups.error' },
  { key: 'hidden', titleKey: 'instances.groups.hidden' },
]

export const useInstancesStore = defineStore('instances', () => {
  const resource = createResource<InstanceInfo[]>({
    key: 'instances',
    fetcher: () => ipcClient.instancesList(),
    ttlMs: 10_000,
  })

  const instances = ref<InstanceInfo[]>([])
  const folders = ref<GameFolderEntry[]>([])
  const currentFolder = ref<string>('')
  const searchQuery = ref('')
  const filterType = ref<'all' | 'pinned' | 'vanilla' | 'modded'>('all')
  const sortBy = ref<InstanceSort>('name')
  const showHidden = ref(false)

  // Sync resource data to local reactive array
  watch(
    resource.data,
    (val) => {
      if (val) {
        instances.value = [...val]
      }
    },
    { immediate: true },
  )

  const initialized = resource.initialized
  const loading = resource.loading
  const refreshing = resource.refreshing

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
      result = result.filter((i) => i.config?.starred || i.config?.pinned)
    } else if (filterType.value === 'vanilla') {
      result = result.filter(
        (i) =>
          i.kind === 'vanilla' ||
          i.kind === 'snapshot' ||
          i.kind === 'oldBeta' ||
          i.kind === 'oldAlpha',
      )
    } else if (filterType.value === 'modded') {
      result = result.filter(
        (i) =>
          i.kind !== 'vanilla' &&
          i.kind !== 'snapshot' &&
          i.kind !== 'oldBeta' &&
          i.kind !== 'oldAlpha',
      )
    }

    return sortInstances(result, sortBy.value)
  })

  // PCL card groups
  const cardGroups = computed<InstanceCardGroup[]>(() => {
    const list = filteredInstances.value
    const groups: InstanceCardGroup[] = []

    for (const conf of CARD_GROUP_CONFIG) {
      if (conf.key === 'hidden' && !showHidden.value) {
        continue
      }
      const items = list.filter((i) => {
        if (!showHidden.value && i.card === 'hidden') return false
        return i.card === conf.key
      })
      if (items.length > 0) {
        groups.push({
          key: conf.key,
          titleKey: conf.titleKey,
          items,
        })
      }
    }

    return groups
  })

  // Legacy groupedInstances for compatibility
  const groupedInstances = computed(() => {
    const pinned: InstanceInfo[] = []
    const categories = new Map<string, InstanceInfo[]>()

    for (const inst of filteredInstances.value) {
      if (filterType.value !== 'pinned' && (inst.config?.starred || inst.config?.pinned)) {
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
      groups: cardGroups.value,
    }
  })

  async function loadSettings() {
    try {
      const settings = await ipcClient.settingsGet()
      folders.value = settings.game?.folders ?? []
      currentFolder.value = settings.game?.currentFolder ?? ''
      const saved = settings.game?.sortBy
      if (
        saved === 'name' ||
        saved === 'version' ||
        saved === 'category' ||
        saved === 'lastPlayed'
      ) {
        sortBy.value = saved
      }
    } catch {
      // Fallback
    }
  }

  async function fetchInstances(force = true) {
    // Re-read the sort preference so a change made in settings applies on the next load.
    await loadSettings()
    await resource.refresh(force)
    if (resource.data.value) {
      instances.value = [...resource.data.value]
    }
  }

  async function setSort(next: InstanceSort) {
    sortBy.value = next
    try {
      await ipcClient.settingsPatch({ game: { sortBy: next } })
    } catch {
      // Persisting the preference must never block sorting
    }
  }

  async function rename(id: string, newName: string) {
    const updated = await ipcClient.instanceRename(id, newName)
    const idx = instances.value.findIndex((i) => i.id === id)
    if (idx >= 0) {
      instances.value[idx] = updated
    }
    resource.set([...instances.value])
    return updated
  }

  async function duplicate(id: string, newName: string) {
    const copy = await ipcClient.instanceDuplicate(id, newName)
    instances.value.push(copy)
    resource.set([...instances.value])
    return copy
  }

  async function remove(id: string, permanent = false) {
    await ipcClient.instanceDelete(id, permanent)
    instances.value = instances.value.filter((i) => i.id !== id)
    resource.set([...instances.value])
  }

  async function toggleStar(id: string) {
    const inst = instances.value.find((i) => i.id === id)
    if (!inst) return
    const newStar = !(inst.config?.starred || inst.config?.pinned)
    const updated = await ipcClient.instanceUpdateConfig(id, {
      ...inst.config,
      starred: newStar,
      pinned: newStar,
    })
    const idx = instances.value.findIndex((i) => i.id === id)
    if (idx >= 0) {
      instances.value[idx] = updated
    }
    resource.set([...instances.value])
    return updated
  }

  async function togglePin(id: string) {
    return toggleStar(id)
  }

  async function toggleHide(id: string) {
    const inst = instances.value.find((i) => i.id === id)
    if (!inst) return
    const isHidden = inst.config?.cardType === 'hidden' || inst.card === 'hidden'
    const updated = await ipcClient.instanceUpdateConfig(id, {
      ...inst.config,
      cardType: isHidden ? null : 'hidden',
    })
    const idx = instances.value.findIndex((i) => i.id === id)
    if (idx >= 0) {
      instances.value[idx] = updated
    }
    resource.set([...instances.value])
    return updated
  }
  async function updateConfig(id: string, patch: Partial<InstanceConfig>) {
    const inst = instances.value.find((i) => i.id === id)
    if (!inst) return
    const updated = await ipcClient.instanceUpdateConfig(id, {
      ...inst.config,
      ...patch,
    })
    const idx = instances.value.findIndex((i) => i.id === id)
    if (idx >= 0) {
      instances.value[idx] = updated
    }
    resource.set([...instances.value])
    return updated
  }

  async function openFolder(id: string, sub = '') {
    await ipcClient.instanceOpenPath(id, sub)
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
    await fetchInstances(true)
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
    await fetchInstances(true)
  }

  async function removeFolder(path: string) {
    folders.value = folders.value.filter((f) => f.path !== path)
    if (currentFolder.value === path) {
      currentFolder.value = folders.value.length > 0 ? folders.value[0].path : ''
    }
    try {
      await ipcClient.settingsPatch({
        game: {
          folders: folders.value,
          currentFolder: currentFolder.value,
        },
      })
    } catch {
      // Fallback
    }
    if (!currentFolder.value) {
      instances.value = []
      resource.set([])
    } else {
      await fetchInstances(true)
    }
  }

  async function renameFolder(path: string, name: string) {
    const folder = folders.value.find((f) => f.path === path)
    if (!folder) return
    folder.name = name
    try {
      await ipcClient.settingsPatch({
        game: {
          folders: folders.value,
          currentFolder: currentFolder.value,
        },
      })
    } catch {
      // Fallback
    }
  }

  return {
    instances,
    folders,
    currentFolder,
    loading,
    refreshing,
    initialized,
    searchQuery,
    filterType,
    sortBy,
    setSort,
    showHidden,
    filteredInstances,
    cardGroups,
    groupedInstances,
    loadSettings,
    fetchInstances,
    rename,
    duplicate,
    remove,
    toggleStar,
    togglePin,
    toggleHide,
    openFolder,
    switchFolder,
    addFolder,
    removeFolder,
    renameFolder,
    updateConfig,
  }
})

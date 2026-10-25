import { defineStore } from 'pinia'
import { ref } from 'vue'
import {
  ipcClient,
  type CompPlatform,
  type CompType,
  type CompSort,
  type CompProject,
  type CompFile,
  type CompFavoriteEntry,
} from '../../ipc/client'

export type CompPlatformFilter = CompPlatform | 'all'

export const useCompStore = defineStore('comp', () => {
  const platform = ref<CompPlatform | 'all'>('all')
  const compType = ref<CompType>('mod')
  const activeTab = ref<CompType | 'favorites'>('mod')
  const query = ref('')
  const mcVersion = ref<string | null>(null)
  const loader = ref<string | null>(null)
  const category = ref<string | null>(null)
  const sort = ref<CompSort>('relevance')

  const hits = ref<CompProject[]>([])
  const totalHits = ref(0)
  const offset = ref(0)
  const limit = ref(20)

  const loading = ref(false)
  const error = ref<string | null>(null)

  const favorites = ref<CompFavoriteEntry[]>([])
  const currentProject = ref<CompProject | null>(null)
  const projectFiles = ref<CompFile[]>([])
  const filesLoading = ref(false)

  async function loadFavorites(): Promise<void> {
    try {
      const settings = await ipcClient.settingsGet()
      if (settings.comp && Array.isArray(settings.comp.favorites)) {
        favorites.value = settings.comp.favorites
      } else {
        favorites.value = []
      }
    } catch (e) {
      console.error('Failed to load favorites from settings', e)
    }
  }

  function isFavorite(pPlatform: CompPlatform | string, idOrSlug: string): boolean {
    return favorites.value.some(
      (f) =>
        f.platform === pPlatform &&
        (f.projectId === idOrSlug || f.slug === idOrSlug),
    )
  }

  async function toggleFavorite(project: CompProject | CompFavoriteEntry): Promise<boolean> {
    const isProject = 'downloads' in project
    const pPlatform = project.platform
    const projectId = isProject ? (project as CompProject).id : project.projectId
    const slug = project.slug
    const title = project.title
    const iconUrl = project.iconUrl ?? null
    const cType = isProject ? (project as CompProject).compType : (project as CompFavoriteEntry).compType

    const existingIndex = favorites.value.findIndex(
      (f) =>
        f.platform === pPlatform &&
        (f.projectId === projectId || f.slug === slug),
    )

    let updated: CompFavoriteEntry[] = []
    let added = false
    if (existingIndex >= 0) {
      updated = favorites.value.filter((_, idx) => idx !== existingIndex)
      added = false
    } else {
      updated = [
        ...favorites.value,
        {
          platform: pPlatform,
          projectId,
          slug,
          title,
          iconUrl,
          compType: cType,
        },
      ]
      added = true
    }
    favorites.value = updated
    try {
      await ipcClient.settingsPatch({
        comp: {
          favorites: updated,
        },
      })
    } catch (e) {
      console.error('Failed to patch favorites settings', e)
    }
    return added
  }

  async function search(resetOffset = true): Promise<void> {
    if (resetOffset) {
      offset.value = 0
    }
    loading.value = true
    error.value = null

    try {
      if (platform.value === 'all') {
        const queryLimit = Math.max(10, Math.ceil(limit.value / 2))
        const queryOffset = Math.floor(offset.value / 2)
        const [mrRes, cfRes] = await Promise.allSettled([
          ipcClient.compSearch({
            platform: 'modrinth',
            compType: compType.value,
            query: query.value,
            mcVersion: mcVersion.value || null,
            loader: loader.value || null,
            category: category.value || null,
            sort: sort.value,
            offset: queryOffset,
            limit: queryLimit,
          }),
          ipcClient.compSearch({
            platform: 'curseforge',
            compType: compType.value,
            query: query.value,
            mcVersion: mcVersion.value || null,
            loader: loader.value || null,
            category: category.value || null,
            sort: sort.value,
            offset: queryOffset,
            limit: queryLimit,
          }),
        ])

        const mrHits = mrRes.status === 'fulfilled' ? mrRes.value.hits : []
        const cfHits = cfRes.status === 'fulfilled' ? cfRes.value.hits : []
        const mrTotal = mrRes.status === 'fulfilled' ? mrRes.value.totalHits : 0
        const cfTotal = cfRes.status === 'fulfilled' ? cfRes.value.totalHits : 0

        const combinedHits: CompProject[] = []
        const maxLen = Math.max(mrHits.length, cfHits.length)
        for (let i = 0; i < maxLen; i++) {
          if (i < mrHits.length) combinedHits.push(mrHits[i])
          if (i < cfHits.length) combinedHits.push(cfHits[i])
        }

        if (resetOffset) {
          hits.value = combinedHits
        } else {
          hits.value = [...hits.value, ...combinedHits]
        }
        totalHits.value = mrTotal + cfTotal
      } else {
        const res = await ipcClient.compSearch({
          platform: platform.value,
          compType: compType.value,
          query: query.value,
          mcVersion: mcVersion.value || null,
          loader: loader.value || null,
          category: category.value || null,
          sort: sort.value,
          offset: offset.value,
          limit: limit.value,
        })
        if (resetOffset) {
          hits.value = res.hits
        } else {
          hits.value = [...hits.value, ...res.hits]
        }
        totalHits.value = res.totalHits
      }
    } catch (err: unknown) {
      error.value = err instanceof Error ? err.message : String(err)
    } finally {
      loading.value = false
    }
  }

  async function loadMore(): Promise<void> {
    if (loading.value || hits.value.length >= totalHits.value) return
    offset.value += limit.value
    await search(false)
  }

  async function openProject(pPlatform: CompPlatform, idOrSlug: string): Promise<void> {
    loading.value = true
    error.value = null
    currentProject.value = null
    projectFiles.value = []

    try {
      const proj = await ipcClient.compProject(pPlatform, idOrSlug)
      currentProject.value = proj
      await fetchProjectFiles(pPlatform, proj.id)
    } catch (err: unknown) {
      error.value = err instanceof Error ? err.message : String(err)
    } finally {
      loading.value = false
    }
  }

  async function fetchProjectFiles(
    pPlatform: CompPlatform,
    projectId: string,
    pMcVersion?: string | null,
    pLoader?: string | null,
  ): Promise<void> {
    filesLoading.value = true
    try {
      const files = await ipcClient.compFiles(
        pPlatform,
        projectId,
        pMcVersion ?? null,
        pLoader ?? null,
      )
      projectFiles.value = files
    } catch (err) {
      console.error('Failed to load project files', err)
      projectFiles.value = []
    } finally {
      filesLoading.value = false
    }
  }

  async function downloadFile(
    file: CompFile,
    targetInstance: string,
    kind: CompType,
  ): Promise<string> {
    return await ipcClient.compDownload(file, targetInstance, kind)
  }

  return {
    platform,
    compType,
    activeTab,
    query,
    mcVersion,
    loader,
    category,
    sort,
    hits,
    totalHits,
    offset,
    limit,
    loading,
    error,
    favorites,
    currentProject,
    projectFiles,
    filesLoading,
    loadFavorites,
    isFavorite,
    toggleFavorite,
    search,
    loadMore,
    openProject,
    fetchProjectFiles,
    downloadFile,
  }
})

import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import {
  getLoaderDefsForType,
  isCategorySupportedOnPlatform,
  isPlatformSupportedForType,
  isSortSupportedOnPlatform,
  resolveCategoryForPlatform,
} from './constants'
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

/** One fully resolved results page: everything the view needs for a single `page`. */
interface CompPageSnapshot {
  hits: CompProject[]
  totalHits: number
  pageCount: number
  partialError: string | null
}

export const useCompStore = defineStore('comp', () => {
  const platform = ref<CompPlatform | 'all'>('all')
  const compType = ref<CompType>('mod')
  const activeTab = ref<CompType | 'favorites'>('mod')
  const query = ref('')
  const mcVersion = ref<string | null>(null)
  const loader = ref<string | null>(null)
  const category = ref<string | null>(null)
  const sort = ref<CompSort>('relevance')

  // `CompBrowserPage` is unmounted when the user leaves it, so everything that must survive a
  // return trip lives here with the filters: the target instance and whether the user already
  // answered "which instance".
  const targetInstanceId = ref<string>('')
  const targetInstanceChosen = ref(false)

  const hits = ref<CompProject[]>([])
  const totalHits = ref(0)
  const offset = ref(0)
  // 24 divides evenly by the 1-, 2- and 3-column card grids and splits into 12 + 12 for the
  // dual-platform "all" view, so no row is ever left half-filled.
  const limit = ref(24)

  const loading = ref(false)
  const error = ref<string | null>(null)
  const partialError = ref<string | null>(null)
  const pageCount = ref(1)
  const page = computed(() => Math.floor(offset.value / limit.value) + 1)

  async function goToPage(p: number): Promise<void> {
    const target = Math.max(1, Math.min(pageCount.value, p))
    if (loading.value) return
    offset.value = (target - 1) * limit.value
    await search(false)
    document.querySelector('main')?.scrollTo({ top: 0 })
  }
  async function loadMore(): Promise<void> {
    if (loading.value || hits.value.length >= totalHits.value) return
    await goToPage(page.value + 1)
  }
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

  // Favourite lookup set: one key per (platform, projectId) and per (platform, slug).
  // Membership is then O(1) instead of a linear scan per rendered row, and is exactly
  // equivalent to the previous `f.projectId === x || f.slug === x` comparison.
  const favoriteKeys = computed(() => {
    const keys = new Set<string>()
    for (const f of favorites.value) {
      keys.add(`${f.platform}:${f.projectId}`)
      keys.add(`${f.platform}:${f.slug}`)
    }
    return keys
  })

  function hasFavorite(pPlatform: CompPlatform | string, idOrSlug: string): boolean {
    return favoriteKeys.value.has(`${pPlatform}:${idOrSlug}`)
  }

  function isFavorite(pPlatform: CompPlatform | string, idOrSlug: string): boolean {
    return hasFavorite(pPlatform, idOrSlug)
  }

  async function toggleFavorite(project: CompProject | CompFavoriteEntry): Promise<boolean> {
    const isProject = 'downloads' in project
    const pPlatform = project.platform
    const projectId = isProject ? (project as CompProject).id : project.projectId
    const slug = project.slug
    const title = project.title
    const iconUrl = project.iconUrl ?? null
    const cType = isProject
      ? (project as CompProject).compType
      : (project as CompFavoriteEntry).compType

    const existingIndex = favorites.value.findIndex(
      (f) => f.platform === pPlatform && (f.projectId === projectId || f.slug === slug),
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

  // --- Paging cache -------------------------------------------------------------------
  // Only the current filter set is held: a cache key includes every filter, so a stale entry
  // can never be served for a different query. Filtering in the toolbar is far rarer than
  // paging, and page-adjacent prefetching makes paging instant.
  let activeFilterKey = ''
  const pageCache = new Map<number, CompPageSnapshot>()
  const inFlightPages = new Map<string, Promise<CompPageSnapshot>>()

  function filterKey(): string {
    return [
      platform.value,
      compType.value,
      query.value.trim(),
      mcVersion.value ?? '',
      loader.value ?? '',
      category.value ?? '',
      sort.value,
    ].join('|')
  }

  /**
   * Drops filters the current tab/platform cannot honour. Without this, a category picked on
   * the Mod tab would silently survive a switch to Worlds and empty the result list, and a
   * Modrinth-only shader loader would be sent to CurseForge.
   */
  function sanitizeFiltersForContext(): void {
    if (compType.value === 'world' && platform.value === 'modrinth') {
      platform.value = 'curseforge'
    }
    if (!isSortSupportedOnPlatform(sort.value, platform.value)) {
      sort.value = 'relevance'
    }

    const supportedLoaders = getLoaderDefsForType(compType.value).filter(
      (def) => platform.value === 'all' || def.platforms.includes(platform.value),
    )
    if (!loader.value || !supportedLoaders.some((def) => def.value === loader.value)) {
      loader.value = null
    }

    if (
      category.value &&
      !isCategorySupportedOnPlatform(compType.value, category.value, platform.value)
    ) {
      category.value = null
    }
  }

  function applySnapshot(snapshot: CompPageSnapshot): void {
    hits.value = snapshot.hits
    totalHits.value = snapshot.totalHits
    pageCount.value = snapshot.pageCount
    partialError.value = snapshot.partialError
  }

  function queryPlatform(pPlatform: CompPlatform, targetPage: number, pageSize: number) {
    return ipcClient.compSearch({
      platform: pPlatform,
      compType: compType.value,
      query: query.value,
      mcVersion: mcVersion.value || null,
      loader: loader.value || null,
      category: resolveCategoryForPlatform(compType.value, category.value, pPlatform),
      sort: sort.value,
      offset: (targetPage - 1) * pageSize,
      limit: pageSize,
    })
  }

  function pageCountFor(total: number, pageSize: number): number {
    return Math.min(500, Math.max(1, Math.ceil(total / pageSize)))
  }

  function sortCombined(combined: CompProject[]): void {
    if (sort.value === 'updated' || sort.value === 'newest') {
      combined.sort((a, b) => {
        const ta = new Date(a.updatedAt).getTime() || 0
        const tb = new Date(b.updatedAt).getTime() || 0
        return tb - ta
      })
    } else if (sort.value === 'downloads') {
      combined.sort((a, b) => (b.downloads || 0) - (a.downloads || 0))
    } else if (sort.value === 'follows') {
      combined.sort((a, b) => (b.follows || 0) - (a.follows || 0))
    }
  }

  /** Resolves one page for the current filters, querying one or both backends as required. */
  async function loadPageSnapshot(targetPage: number): Promise<CompPageSnapshot> {
    if (platform.value !== 'all') {
      const res = await queryPlatform(platform.value, targetPage, limit.value)
      return {
        hits: res.hits,
        totalHits: res.totalHits,
        pageCount: pageCountFor(res.totalHits, limit.value),
        partialError: null,
      }
    }

    const activeCategory = category.value
    const canQueryModrinth =
      isPlatformSupportedForType(compType.value, 'modrinth') &&
      (!activeCategory || isCategorySupportedOnPlatform(compType.value, activeCategory, 'modrinth'))
    const canQueryCurseforge =
      isPlatformSupportedForType(compType.value, 'curseforge') &&
      (!activeCategory ||
        isCategorySupportedOnPlatform(compType.value, activeCategory, 'curseforge')) &&
      // Shader loaders are Modrinth-only facets; CurseForge would just ignore them.
      !(compType.value === 'shader' && loader.value)

    if (!canQueryModrinth && !canQueryCurseforge) {
      throw new Error('No supported platform matches the current filters')
    }

    // A filter combination only one platform understands is queried alone, at full page size:
    // halving it there would show half-empty pages.
    if (canQueryModrinth !== canQueryCurseforge) {
      const onlyPlatform: CompPlatform = canQueryModrinth ? 'modrinth' : 'curseforge'
      const res = await queryPlatform(onlyPlatform, targetPage, limit.value)
      return {
        hits: res.hits,
        totalHits: res.totalHits,
        pageCount: pageCountFor(res.totalHits, limit.value),
        partialError: null,
      }
    }

    const perPlatformLimit = Math.max(10, Math.ceil(limit.value / 2))
    const [mrRes, cfRes] = await Promise.allSettled([
      queryPlatform('modrinth', targetPage, perPlatformLimit),
      queryPlatform('curseforge', targetPage, perPlatformLimit),
    ])

    if (mrRes.status === 'rejected' && cfRes.status === 'rejected') {
      const err = mrRes.reason || cfRes.reason
      throw err instanceof Error ? err : new Error(String(err))
    }

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
    sortCombined(combinedHits)

    const partialError =
      mrRes.status === 'rejected' ? 'Modrinth' : cfRes.status === 'rejected' ? 'CurseForge' : null

    return {
      hits: combinedHits,
      totalHits: mrTotal + cfTotal,
      pageCount: Math.min(
        500,
        Math.max(
          1,
          Math.max(Math.ceil(mrTotal / perPlatformLimit), Math.ceil(cfTotal / perPlatformLimit)),
        ),
      ),
      partialError,
    }
  }

  function fetchPageSnapshot(key: string, targetPage: number): Promise<CompPageSnapshot> {
    const inFlightKey = `${key}:${targetPage}`
    const pending = inFlightPages.get(inFlightKey)
    if (pending) return pending

    const request = loadPageSnapshot(targetPage).finally(() => {
      inFlightPages.delete(inFlightKey)
    })
    inFlightPages.set(inFlightKey, request)
    return request
  }

  /** Warms the neighbouring pages so `goToPage` renders from memory instead of waiting. */
  function prefetchAdjacentCompPages(key: string, centerPage: number, totalPages: number): void {
    for (const adjPage of [centerPage - 1, centerPage + 1]) {
      if (adjPage < 1 || adjPage > totalPages) continue
      if (pageCache.has(adjPage) || inFlightPages.has(`${key}:${adjPage}`)) continue
      void fetchPageSnapshot(key, adjPage)
        .then((snapshot) => {
          if (activeFilterKey === key && snapshot.hits.length > 0) {
            pageCache.set(adjPage, snapshot)
          }
        })
        .catch(() => {
          // A failed prefetch must stay invisible: the page is fetched again on demand.
        })
    }
  }

  async function search(resetOffset = true, forceRefresh = false): Promise<void> {
    sanitizeFiltersForContext()
    if (resetOffset) {
      offset.value = 0
    }
    loading.value = true
    error.value = null
    partialError.value = null

    const currentFilterKey = filterKey()
    if (forceRefresh || currentFilterKey !== activeFilterKey) {
      // A different filter set is a different result set: nothing cached survives it.
      pageCache.clear()
      inFlightPages.clear()
      activeFilterKey = currentFilterKey
    }

    const targetPage = Math.floor(offset.value / limit.value) + 1

    if (!forceRefresh) {
      const cached = pageCache.get(targetPage)
      if (cached) {
        applySnapshot(cached)
        loading.value = false
        prefetchAdjacentCompPages(currentFilterKey, targetPage, cached.pageCount)
        return
      }
    }

    try {
      const snapshot = await fetchPageSnapshot(currentFilterKey, targetPage)
      // A slower response for a filter set the user already left must not overwrite the view.
      if (activeFilterKey !== currentFilterKey) return
      applySnapshot(snapshot)
      if (snapshot.hits.length > 0) pageCache.set(targetPage, snapshot)
      prefetchAdjacentCompPages(currentFilterKey, targetPage, snapshot.pageCount)
    } catch (err: unknown) {
      if (activeFilterKey !== currentFilterKey) return
      error.value = err instanceof Error ? err.message : String(err)
      hits.value = []
      totalHits.value = 0
      pageCount.value = 1
    } finally {
      if (activeFilterKey === currentFilterKey) loading.value = false
    }
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
    targetInstanceId,
    targetInstanceChosen,
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
    hasFavorite,
    toggleFavorite,
    search,
    partialError,
    pageCount,
    page,
    goToPage,
    loadMore,
    openProject,
    fetchProjectFiles,
    downloadFile,
  }
})

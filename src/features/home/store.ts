import { defineStore } from 'pinia'
import { ref } from 'vue'
import { ipcClient, type NewsItem, type AnnouncementItem } from '../../ipc/client'

export const useHomeStore = defineStore('home', () => {
  const news = ref<NewsItem[]>([])
  const page = ref(1)
  const loadingNews = ref(false)
  const newsError = ref<string | null>(null)
  const emptyNews = ref(false)
  const announcements = ref<AnnouncementItem[]>([])
  const loadingAnnouncements = ref(false)

  const helpQuery = ref('')
  const selectedHelpId = ref<string>('getting-started')

  // Paging is the only navigation the news feed has, so an already-fetched page is served
  // straight from memory and neighbours are warmed in the background. Both maps live outside
  // the reactive state: they are transport bookkeeping, not view data.
  const newsPageCache = new Map<number, NewsItem[]>()
  const inFlightNews = new Map<number, Promise<NewsItem[]>>()

  function fetchNewsPage(targetPage: number, forceRefresh: boolean): Promise<NewsItem[]> {
    if (!forceRefresh) {
      const cached = newsPageCache.get(targetPage)
      if (cached) return Promise.resolve(cached)
      const pending = inFlightNews.get(targetPage)
      if (pending) return pending
    }

    const request = ipcClient
      .newsFetch(targetPage, forceRefresh)
      .then((items) => {
        if (items.length > 0) newsPageCache.set(targetPage, items)
        return items
      })
      .finally(() => {
        inFlightNews.delete(targetPage)
      })
    inFlightNews.set(targetPage, request)
    return request
  }

  function prefetchAdjacentNews(centerPage: number) {
    for (const adjPage of [centerPage - 1, centerPage + 1]) {
      if (adjPage < 1) continue
      if (newsPageCache.has(adjPage) || inFlightNews.has(adjPage)) continue
      void fetchNewsPage(adjPage, false).catch(() => {
        // A failed prefetch must stay invisible: the page is fetched again on demand.
      })
    }
  }

  async function loadNews(targetPage: number = 1, forceRefresh: boolean = false) {
    if (forceRefresh) newsPageCache.clear()

    if (!forceRefresh) {
      const cached = newsPageCache.get(targetPage)
      if (cached) {
        news.value = cached
        page.value = targetPage
        newsError.value = null
        emptyNews.value = false
        loadingNews.value = false
        prefetchAdjacentNews(targetPage)
        return
      }
    }

    loadingNews.value = true
    newsError.value = null
    emptyNews.value = false
    try {
      const items = await fetchNewsPage(targetPage, forceRefresh)
      news.value = items
      page.value = targetPage
      if (items.length === 0 && targetPage > 1) {
        emptyNews.value = true
      }
      if (items.length > 0) prefetchAdjacentNews(targetPage)
    } catch (err: unknown) {
      newsError.value = err instanceof Error ? err.message : String(err)
    } finally {
      loadingNews.value = false
    }
  }

  async function loadAnnouncements() {
    loadingAnnouncements.value = true
    try {
      const items = await ipcClient.announcementsFetch()
      announcements.value = items
    } catch {
      announcements.value = []
    } finally {
      loadingAnnouncements.value = false
    }
  }

  async function dismissAnnouncement(id: string) {
    try {
      const settings = await ipcClient.settingsGet()
      const currentDismissed = settings.states?.dismissedAnnouncements ?? []
      if (!currentDismissed.includes(id)) {
        const nextDismissed = [...currentDismissed, id]
        await ipcClient.settingsPatch({
          states: {
            dismissedAnnouncements: nextDismissed,
          },
        })
      }
      announcements.value = announcements.value.filter((item) => item.id !== id)
    } catch {
      announcements.value = announcements.value.filter((item) => item.id !== id)
    }
  }

  return {
    news,
    page,
    loadingNews,
    newsError,
    emptyNews,
    announcements,
    loadingAnnouncements,
    helpQuery,
    selectedHelpId,
    loadNews,
    loadAnnouncements,
    dismissAnnouncement,
  }
})

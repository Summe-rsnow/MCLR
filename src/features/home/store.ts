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

  async function loadNews(targetPage: number = 1, forceRefresh: boolean = false) {
    loadingNews.value = true
    newsError.value = null
    emptyNews.value = false
    try {
      const items = await ipcClient.newsFetch(targetPage, forceRefresh)
      news.value = items
      page.value = targetPage
      if (items.length === 0 && targetPage > 1) {
        emptyNews.value = true
      }
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

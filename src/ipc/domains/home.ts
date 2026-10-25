import { invokeCommand } from '../client'

export interface NewsItem {
  id: string
  title: string
  summary: string
  date: string
  category: string
  imageUrl?: string | null
  articleUrl: string
}

export interface AnnouncementButton {
  text: string
  url?: string | null
}

export interface AnnouncementItem {
  id: string
  title: string
  detail: string
  btn1?: AnnouncementButton | null
  btn2?: AnnouncementButton | null
}

export const homeIpc = {
  newsFetch: (page?: number, forceRefresh?: boolean): Promise<NewsItem[]> =>
    invokeCommand<NewsItem[]>('news_fetch', {
      page: page ?? 0,
      forceRefresh: forceRefresh ?? false,
    }),
  announcementsFetch: (): Promise<AnnouncementItem[]> =>
    invokeCommand<AnnouncementItem[]>('announcements_fetch'),
}

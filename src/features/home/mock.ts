import type { NewsItem, AnnouncementItem } from '../../ipc/client'

export const mockNews: NewsItem[] = [
  {
    id: 'news-1',
    title: 'Minecraft 1.21.4 Release',
    summary: 'A new release is out now with various tweaks, bug fixes, and technical improvements!',
    date: '2026-02-15',
    category: 'Release',
    imageUrl: 'https://www.minecraft.net/content/dam/games/minecraft/key-art/1-21-4.jpg',
    articleUrl: 'https://www.minecraft.net/zh-hans/article/minecraft-java-edition-1-21-4',
  },
  {
    id: 'news-2',
    title: 'Minecraft Snapshot 26w08a',
    summary: 'Explore upcoming experimental features and test our new world generation settings.',
    date: '2026-02-20',
    category: 'Snapshot',
    imageUrl: null,
    articleUrl: 'https://www.minecraft.net/zh-hans/article/minecraft-snapshot-26w08a',
  },
]

export const mockAnnouncements: AnnouncementItem[] = [
  {
    id: 'announcement-v0011',
    title: 'MCLR v0.0.11 Available',
    detail:
      'Welcome to MCLR v0.0.11! Personalization, home news, and built-in help center are now ready.',
    btn1: {
      text: 'View Help',
      url: null,
    },
    btn2: {
      text: 'Release Notes',
      url: 'https://www.minecraft.net/zh-hans/articles',
    },
  },
]

const handlers = {
  news_fetch: (_args?: Record<string, unknown>) => {
    return mockNews
  },
  announcements_fetch: () => {
    return mockAnnouncements
  },
}

export default handlers

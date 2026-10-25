import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import { createRouter, createWebHistory, type Router } from 'vue-router'
import type { Settings } from '../../../ipc/client'
import HomePage from '../HomePage.vue'
import { useHomeStore } from '../store'
import { ipcClient } from '../../../ipc/client'
import { setupMockIpc } from '../../../ipc/mock'
import homeLocale from '../../../locales/zh-CN/home.json'
import launchLocale from '../../../locales/zh-CN/launch.json'
import commonLocale from '../../../locales/zh-CN/common.json'
import settingsGameLocale from '../../../locales/zh-CN/settingsGame.json'
import instancesLocale from '../../../locales/zh-CN/instances.json'

const i18n = createI18n({
  legacy: false,
  locale: 'zh-CN',
  messages: {
    'zh-CN': {
      home: homeLocale,
      launch: launchLocale,
      common: commonLocale,
      settingsGame: settingsGameLocale,
      instances: instancesLocale,
    },
  },
})

describe('HomePage & Home Feature', () => {
  let router: Router

  beforeEach(() => {
    setActivePinia(createPinia())
    setupMockIpc()
    try {
      localStorage.clear()
    } catch {
      // ignore
    }
    router = createRouter({
      history: createWebHistory(),
      routes: [
        { path: '/home', component: HomePage },
        { path: '/launch', component: HomePage },
        { path: '/', redirect: '/launch' },
        { path: '/download', component: { template: '<div>download</div>' } },
        { path: '/settings/game', component: { template: '<div>settings</div>' } },
        { path: '/logs', component: { template: '<div>logs</div>' } },
      ],
    })
  })

  it('renders unified launch view with news toggle default OFF', async () => {
    const wrapper = mount(HomePage, {
      global: {
        plugins: [i18n, router],
      },
    })

    await flushPromises()
    await router.isReady()

    // Top section: Launch hero and selectors
    expect(wrapper.text()).toContain('游戏启动')
    expect(wrapper.text()).toContain('选择实例')

    // Below: News section with toggle switch and compact disabled card by default
    expect(wrapper.text()).toContain('官方动态与资讯')
    expect(wrapper.text()).toContain('显示官方新闻与公告')
    expect(wrapper.text()).toContain('新闻资讯已关闭，点击开关以加载官方公告与更新动态')
    expect(wrapper.find('[data-testid="news-compact-off-card"]').exists()).toBe(true)
  })

  it('renders news items and announcements when showNews is enabled', async () => {
    const mockNewsList = [
      {
        id: 'news-test-1',
        title: 'Minecraft Test Update',
        summary: 'Testing summary description',
        date: '2026-03-01',
        category: 'Release',
        imageUrl: null,
        articleUrl: 'https://www.minecraft.net/zh-hans/article/test',
      },
    ]

    const mockAnnouncementList = [
      {
        id: 'ann-1',
        title: 'Important System Announcement',
        detail: 'Launcher maintenance scheduled',
        btn1: { text: 'Details', url: null },
        btn2: null,
      },
    ]

    vi.spyOn(ipcClient, 'newsFetch').mockResolvedValue(mockNewsList)
    vi.spyOn(ipcClient, 'announcementsFetch').mockResolvedValue(mockAnnouncementList)

    const homeStore = useHomeStore()
    await homeStore.loadNews(1)
    await homeStore.loadAnnouncements()

    const wrapper = mount(HomePage, {
      global: {
        plugins: [i18n, router],
      },
    })
    const enableBtn = wrapper.find('[data-testid="news-enable-btn"]')
    if (enableBtn.exists()) {
      await enableBtn.trigger('click')
      await flushPromises()
    }

    expect(wrapper.find('[data-testid="news-container-on"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('Minecraft Test Update')
    expect(wrapper.text()).toContain('Important System Announcement')

    // Verify masonry columns and pagination
    const newsCols = wrapper.find('.columns-1')
    expect(newsCols.exists()).toBe(true)
    const pager = wrapper.find('nav[role="navigation"]')
    expect(pager.exists()).toBe(true)
  })

  it('dismisses announcement and patches settings', async () => {
    const mockAnnouncementList = [
      {
        id: 'ann-to-dismiss',
        title: 'Dismiss Me',
        detail: 'This will be dismissed',
        btn1: null,
        btn2: null,
      },
    ]

    vi.spyOn(ipcClient, 'settingsGet').mockResolvedValue({
      schemaVersion: 1,
      states: { dismissedAnnouncements: [] },
    } as unknown as Settings)

    const patchSpy = vi
      .spyOn(ipcClient, 'settingsPatch')
      .mockResolvedValue({} as unknown as Settings)

    const homeStore = useHomeStore()
    homeStore.announcements = [...mockAnnouncementList]

    await homeStore.dismissAnnouncement('ann-to-dismiss')

    expect(homeStore.announcements.length).toBe(0)
    expect(patchSpy).toHaveBeenCalledWith({
      states: {
        dismissedAnnouncements: ['ann-to-dismiss'],
      },
    })
  })
})

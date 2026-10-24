import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import { createRouter, createWebHistory, type Router } from 'vue-router'
import type { Settings } from '../../../ipc/client'
import HomePage from '../HomePage.vue'
import { useHomeStore } from '../store'
import { ipcClient } from '../../../ipc/client'
import homeLocale from '../../../locales/zh-CN/home.json'

const i18n = createI18n({
  legacy: false,
  locale: 'zh-CN',
  messages: {
    'zh-CN': {
      home: homeLocale,
    },
  },
})

describe('HomePage & Home Feature', () => {
  let router: Router

  beforeEach(() => {
    setActivePinia(createPinia())
    router = createRouter({
      history: createWebHistory(),
      routes: [
        { path: '/home', component: HomePage },
        { path: '/', redirect: '/home' },
      ],
    })
  })

  it('renders news items and fetches announcements', async () => {
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

    await router.isReady()
    expect(wrapper.text()).toContain('Minecraft Test Update')
    expect(wrapper.text()).toContain('Important System Announcement')
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

    const patchSpy = vi.spyOn(ipcClient, 'settingsPatch').mockResolvedValue({} as unknown as Settings)

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

  it('filters help center articles and renders markdown content', async () => {
    const wrapper = mount(HomePage, {
      global: {
        plugins: [i18n, router],
      },
    })

    await router.isReady()

    // Switch to Help tab
    const tabs = wrapper.findAll('button')
    const helpTabBtn = tabs.find((btn) => btn.text().includes('帮助中心'))
    expect(helpTabBtn).toBeDefined()
    await helpTabBtn!.trigger('click')

    const homeStore = useHomeStore()
    expect(wrapper.text()).toContain('新手起步指南')

    // Search query filter
    homeStore.helpQuery = 'Java'
    await wrapper.vm.$nextTick()

    // Left sidebar list
    const sidebarButtons = wrapper.findAll('[data-test="help-article-item"]')
    const sidebarText = sidebarButtons.map((b) => b.text()).join(' ')
    expect(sidebarText).toContain('Java 配置与选择')
    expect(sidebarText).not.toContain('新手起步指南')
    // Select article and verify markdown renders into prose HTML container
    homeStore.selectedHelpId = 'java-guide'
    await wrapper.vm.$nextTick()
    const proseDiv = wrapper.find('.prose')
    expect(proseDiv.exists()).toBe(true)
    expect(proseDiv.text()).toContain('Minecraft 运行于 Java 虚拟机之上')
    expect(proseDiv.html()).toContain('<h2>版本对应关系推荐</h2>')
  })
})

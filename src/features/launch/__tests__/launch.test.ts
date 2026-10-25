import { describe, it, expect, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { mount, flushPromises } from '@vue/test-utils'
import { createRouter, createWebHistory } from 'vue-router'
import { createI18n } from 'vue-i18n'
import { useLaunchStore } from '../store'
import { useInstancesStore } from '../../instances/store'
import { useAccountsStore } from '../../accounts/store'
import LaunchPage from '../LaunchPage.vue'
import { setupMockIpc } from '../../../ipc/mock'
import { resetMockLaunchData } from '../mock'
import zhCnLaunch from '../../../locales/zh-CN/launch.json'
import zhCnCommon from '../../../locales/zh-CN/common.json'
import zhCnSettingsGame from '../../../locales/zh-CN/settingsGame.json'
import zhCnInstances from '../../../locales/zh-CN/instances.json'
import { ipcClient } from '../../../ipc/client'
const i18n = createI18n({
  legacy: false,
  locale: 'zh-CN',
  messages: {
    'zh-CN': {
      launch: zhCnLaunch,
      common: zhCnCommon,
      settingsGame: zhCnSettingsGame,
      instances: zhCnInstances,
    },
  },
})

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', component: { template: '<div>home</div>' } },
    { path: '/download', component: { template: '<div>download</div>' } },
    { path: '/settings/game', component: { template: '<div>settings</div>' } },
    { path: '/logs', component: { template: '<div>logs</div>' } },
  ],
})

describe('Launch feature', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    resetMockLaunchData()
    setupMockIpc()
  })

  it('initializes store and runs startLaunch successfully', async () => {
    const instancesStore = useInstancesStore()
    const accountsStore = useAccountsStore()
    await instancesStore.fetchInstances()
    await accountsStore.fetchAccounts()

    const store = useLaunchStore()
    await store.fetchProcesses()

    const result = await store.startLaunch()
    expect(result).toBeDefined()
    expect(result?.taskId).toContain('task-launch')
    expect(store.currentStage).toBe('running')
    expect(store.runningProcesses.length).toBeGreaterThan(0)
  })

  it('handles Java missing error and triggers recovery download', async () => {
    const store = useLaunchStore()
    expect(store.javaMissingMajor).toBeNull()

    const taskId = await store.downloadJavaRuntime('java-runtime-gamma')
    expect(taskId).toContain('task-java-java-runtime-gamma')
  })

  it('renders LaunchPage correctly and displays instance & account info', async () => {
    const wrapper = mount(LaunchPage, {
      global: {
        plugins: [i18n, router],
      },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('游戏启动')
    expect(wrapper.text()).toContain('启动游戏')
    expect(wrapper.text()).toContain('Vanilla 1.20.1')
    expect(wrapper.text()).toContain('StevePro')
  })

  it('renders LaunchPage in empty state when no instances exist', async () => {
    vi.spyOn(ipcClient, 'instancesList').mockResolvedValue([])
    const instancesStore = useInstancesStore()
    const launchStore = useLaunchStore()
    instancesStore.instances = []
    launchStore.selectedInstanceId = null

    const wrapper = mount(LaunchPage, {
      global: {
        plugins: [i18n, router],
      },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('未找到可用实例')
    expect(wrapper.text()).toContain('下载游戏')
  })

  it('defaults showNews to false, renders compact disabled card and does not fetch news', async () => {
    const newsFetchSpy = vi.spyOn(ipcClient, 'newsFetch')
    const announcementsFetchSpy = vi.spyOn(ipcClient, 'announcementsFetch')

    try {
      localStorage.clear()
    } catch {
      // ignore
    }

    const wrapper = mount(LaunchPage, {
      global: {
        plugins: [i18n, router],
      },
    })

    await flushPromises()

    expect(wrapper.find('[data-testid="news-compact-off-card"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('新闻资讯已关闭，点击开关以加载官方公告与更新动态')
    expect(wrapper.find('[data-testid="news-container-on"]').exists()).toBe(false)
    expect(newsFetchSpy).not.toHaveBeenCalled()
    expect(announcementsFetchSpy).not.toHaveBeenCalled()
  })

  it('enables news when toggle switch is clicked, persisting to localStorage and fetching', async () => {
    const newsFetchSpy = vi.spyOn(ipcClient, 'newsFetch').mockResolvedValue([
      {
        id: 'news-item-1',
        title: 'New Snapshot Available',
        summary: 'Try the newest Minecraft snapshot features',
        date: '2026-03-02',
        category: 'Snapshot',
        imageUrl: null,
        articleUrl: 'https://minecraft.net',
      },
    ])
    const announcementsFetchSpy = vi.spyOn(ipcClient, 'announcementsFetch').mockResolvedValue([])

    try {
      localStorage.clear()
    } catch {
      // ignore
    }

    const wrapper = mount(LaunchPage, {
      global: {
        plugins: [i18n, router],
      },
    })

    await flushPromises()

    const enableBtn = wrapper.find('[data-testid="news-enable-btn"]')
    expect(enableBtn.exists()).toBe(true)
    await enableBtn.trigger('click')
    await flushPromises()

    expect(newsFetchSpy).toHaveBeenCalled()
    expect(announcementsFetchSpy).toHaveBeenCalled()
    expect(wrapper.find('[data-testid="news-container-on"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('New Snapshot Available')
  })
})

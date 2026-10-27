import { describe, it, expect, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { mount, flushPromises } from '@vue/test-utils'
import { createRouter, createWebHistory } from 'vue-router'
import { createI18n } from 'vue-i18n'
import { useLaunchStore } from '../store'
import { useInstancesStore } from '../../instances/store'
import { useAccountsStore } from '../../accounts/store'
import { useHomeStore } from '../../home/store'
import LaunchPage from '../LaunchPage.vue'
import { setupMockIpc } from '../../../ipc/mock'
import { resetMockLaunchData } from '../mock'
import zhCnLaunch from '../../../locales/zh-CN/launch.json'
import zhCnCommon from '../../../locales/zh-CN/common.json'
import zhCnSettingsGame from '../../../locales/zh-CN/settingsGame.json'
import zhCnInstances from '../../../locales/zh-CN/instances.json'
import zhCnAccounts from '../../../locales/zh-CN/accounts.json'
import zhCnLogs from '../../../locales/zh-CN/logs.json'
import { ipcClient } from '../../../ipc/client'
import { useDialog } from '../../../ui/useDialog'
const i18n = createI18n({
  legacy: false,
  locale: 'zh-CN',
  messages: {
    'zh-CN': {
      launch: zhCnLaunch,
      common: zhCnCommon,
      settingsGame: zhCnSettingsGame,
      instances: zhCnInstances,
      accounts: zhCnAccounts,
      logs: zhCnLogs,
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
  let storage: Record<string, string> = {}

  beforeEach(() => {
    vi.restoreAllMocks()
    setActivePinia(createPinia())
    resetMockLaunchData()
    setupMockIpc()
    storage = {}
    const mockStorage = {
      getItem: (k: string) => (k in storage ? storage[k] : null),
      setItem: (k: string, v: string) => {
        storage[k] = String(v)
      },
      removeItem: (k: string) => {
        delete storage[k]
      },
      clear: () => {
        storage = {}
      },
    }
    Object.defineProperty(window, 'localStorage', {
      value: mockStorage,
      writable: true,
      configurable: true,
    })
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

  it('keeps the persisted instance selection while the instance list is still empty', () => {
    // Regression: the heal watcher used to clear the stored id on first tick,
    // before `fetchInstances()` resolved, so a restart never restored the
    // last-used instance.
    storage['mclr.launch.instanceId'] = '1.20.1-vanilla'

    useInstancesStore()
    const store = useLaunchStore()

    expect(store.selectedInstanceId).toBe('1.20.1-vanilla')
    expect(storage['mclr.launch.instanceId']).toBe('1.20.1-vanilla')
  })

  it('restores the persisted instance once the list loads after restart', async () => {
    storage['mclr.launch.instanceId'] = '1.20.1-vanilla'

    const instancesStore = useInstancesStore()
    const store = useLaunchStore()
    expect(store.selectedInstanceId).toBe('1.20.1-vanilla')

    await instancesStore.fetchInstances()

    expect(store.selectedInstance?.id).toBe('1.20.1-vanilla')
    expect(storage['mclr.launch.instanceId']).toBe('1.20.1-vanilla')
  })

  it('heals a dangling persisted id only after the list has loaded', async () => {
    storage['mclr.launch.instanceId'] = 'deleted-instance'

    const instancesStore = useInstancesStore()
    const store = useLaunchStore()

    await instancesStore.fetchInstances()

    expect(store.selectedInstance?.id).not.toBe('deleted-instance')
    expect(store.selectedInstanceId).toBe(store.selectedInstance?.id)
    expect(storage['mclr.launch.instanceId']).toBe(store.selectedInstance?.id)
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
    expect(wrapper.text()).toContain('微软')
    expect(wrapper.text()).not.toContain('microsoft')
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
    // Exactly one "安装新版本" in header action, none in row
    const createButtons = wrapper.findAll('button').filter((b) => b.text().includes('安装新版本'))
    expect(createButtons).toHaveLength(1)
    expect(wrapper.text()).toContain('还没有实例，点击右上角“安装新版本”创建第一个实例')
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

    // Masonry columns check
    const newsContainer = wrapper.find('.columns-1')
    expect(newsContainer.exists()).toBe(true)
    expect(newsContainer.classes()).toContain('columns-1')

    // Floating pager check
    const paginationNav = wrapper.find('nav[role="navigation"]')
    expect(paginationNav.exists()).toBe(true)
    const nextBtn = paginationNav.find('button[title="下一页"]')
    expect(nextBtn.exists()).toBe(true)
    await nextBtn.trigger('click')
    await flushPromises()
    expect(newsFetchSpy).toHaveBeenCalledWith(2, false)
  })

  it('renders matching multi-column container classes for news skeleton when loading', async () => {
    // Mock pending promise so loadingNews remains true
    vi.spyOn(ipcClient, 'newsFetch').mockImplementation(() => new Promise(() => {}))
    vi.spyOn(ipcClient, 'announcementsFetch').mockResolvedValue([])

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

    const skeleton = wrapper.find('[data-testid="news-skeleton"]')
    expect(skeleton.exists()).toBe(true)
    expect(skeleton.classes()).toContain('columns-1')
    expect(skeleton.classes()).toContain('md:columns-2')
    expect(skeleton.classes()).toContain('lg:columns-3')
    expect(skeleton.classes()).toContain('gap-4')
    const cards = skeleton.findAll('.break-inside-avoid')
    expect(cards.length).toBe(6)
  })

  it('renders fallback empty state when page > 1 returns no news, and clicking button loads page 1', async () => {
    localStorage.setItem('mclr.launch.showNews', 'true')
    const newsFetchSpy = vi.spyOn(ipcClient, 'newsFetch').mockResolvedValue([])
    vi.spyOn(ipcClient, 'announcementsFetch').mockResolvedValue([])

    const wrapper = mount(LaunchPage, {
      global: {
        plugins: [i18n, router],
      },
    })
    await flushPromises()

    const homeStore = useHomeStore()
    await homeStore.loadNews(2)
    await flushPromises()

    expect(wrapper.text()).toContain('该页暂无资讯内容')
    expect(wrapper.text()).toContain('所选页码超出资讯列表范围')

    const backBtn = wrapper.findAll('button').find((b) => b.text().includes('返回第一页'))
    expect(backBtn?.exists()).toBe(true)
    await backBtn?.trigger('click')
    await flushPromises()

    expect(newsFetchSpy).toHaveBeenCalledWith(1, false)
  })

  it('opens jump dialog and navigates to target page', async () => {
    localStorage.setItem('mclr.launch.showNews', 'true')
    const newsFetchSpy = vi.spyOn(ipcClient, 'newsFetch').mockResolvedValue([
      {
        id: 'news-1',
        title: 'Title 1',
        summary: 'Summary 1',
        date: '2026-10-25',
        category: 'Official',
        articleUrl: 'https://example.com/1',
        imageUrl: null,
      },
    ])
    vi.spyOn(ipcClient, 'announcementsFetch').mockResolvedValue([])

    const wrapper = mount(LaunchPage, {
      global: {
        plugins: [i18n, router],
      },
    })
    await flushPromises()

    const jumpBtn = wrapper.find('[data-testid="news-jump-btn"]')
    expect(jumpBtn.exists()).toBe(true)
    await jumpBtn.trigger('click')
    await flushPromises()
    const input = document.body.querySelector<HTMLInputElement>(
      '[data-testid="jump-page-input"] input',
    )
    expect(input).not.toBeNull()
    if (input) {
      input.value = '5'
      input.dispatchEvent(new Event('input'))
    }
    await flushPromises()
    const confirmBtn = Array.from(document.body.querySelectorAll('button')).find(
      (b) => b.textContent?.trim() === '确认',
    )
    expect(confirmBtn).toBeDefined()
    confirmBtn?.click()
    await flushPromises()
    expect(newsFetchSpy).toHaveBeenCalledWith(5, false)
  })

  it('renders InstancePreviewCard with badges, path, and action buttons', async () => {
    const wrapper = mount(LaunchPage, {
      global: {
        plugins: [i18n, router],
      },
    })
    await flushPromises()

    const previewCard = wrapper.find('[data-testid="instance-preview-card"]')
    expect(previewCard.exists()).toBe(true)
    // Default selected instance is the pinned one ('Fabric 1.20.4')
    expect(previewCard.text()).toContain('Fabric 1.20.4')
    expect(previewCard.text()).toContain('1.20.4')
    expect(previewCard.text()).toContain('Fabric 0.15.11')
    expect(previewCard.text()).toContain('隔离')
    expect(previewCard.text()).not.toContain('versions/1.20.4-fabric')
    expect(wrapper.find('[data-testid="instance-starred-icon"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="instance-open-folder-btn"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="instance-manage-btn"]').exists()).toBe(true)

    // Select vanilla instance and verify vanilla preview card
    const launchStore = useLaunchStore()
    launchStore.selectInstance('1.20.1-vanilla')
    await flushPromises()

    expect(previewCard.text()).toContain('Vanilla 1.20.1')
    expect(previewCard.text()).toContain('1.20.1')
    expect(previewCard.text()).toContain('原版')
    expect(previewCard.text()).not.toContain('versions/1.20.1-vanilla')
  })

  it('gives instance dropdown options an instance icon like accounts show avatars', async () => {
    const instancesStore = useInstancesStore()
    await instancesStore.fetchInstances()
    useLaunchStore()

    const wrapper = mount(LaunchPage, {
      global: { plugins: [i18n, router] },
    })
    await flushPromises()

    const select = wrapper.findAllComponents({ name: 'MSelect' }).find((s) => {
      const options = s.props('options') as Array<{ value: string; icon?: string }>
      return options.some((o) => o.value === '1.20.1-vanilla')
    })
    expect(select).toBeTruthy()

    const options = select!.props('options') as Array<{
      label: string
      value: string
      icon?: string
      avatar?: string
      badge?: string
    }>
    const vanilla = options.find((o) => o.value === '1.20.1-vanilla')!
    expect(vanilla.icon || vanilla.avatar).toBeTruthy()
    expect(vanilla.badge).toBeTruthy()

    // The trigger renders the selected instance icon next to its name.
    const trigger = select!.find('button')
    expect(trigger.exists()).toBe(true)
    const iconEl = trigger.findAll('span, img').filter((el) => {
      const cls = el.attributes('class') ?? ''
      return (
        cls.includes('i-lucide') || cls.includes('i-simple-icons') || el.element.tagName === 'IMG'
      )
    })
    expect(iconEl.length).toBeGreaterThan(0)
  })

  it('renders AccountPreviewCard with username and manage button', async () => {
    const wrapper = mount(LaunchPage, {
      global: {
        plugins: [i18n, router],
      },
    })
    await flushPromises()

    const previewCard = wrapper.find('[data-testid="account-preview-card"]')
    expect(previewCard.exists()).toBe(true)
    expect(previewCard.text()).toContain('StevePro')
    expect(previewCard.text()).toContain('微软')
    expect(wrapper.find('[data-testid="account-manage-btn"]').exists()).toBe(true)
  })

  it('renders running actions and triggers confirmDanger when force kill is clicked', async () => {
    const launchStore = useLaunchStore()
    const killSpy = vi.spyOn(launchStore, 'killProcess').mockResolvedValue()

    const wrapper = mount(LaunchPage, {
      global: {
        plugins: [i18n, router],
      },
    })
    await flushPromises()

    // Mock running process
    launchStore.runningProcesses = [
      {
        processId: 2026,
        instanceId: '1.20.1-vanilla',
        accountName: 'StevePro',
        startedAt: new Date().toISOString(),
        running: true,
        exitCode: null,
      },
    ]
    await flushPromises()

    const killBtn = wrapper.find('[data-testid="hero-force-kill-btn"]')
    expect(killBtn.exists()).toBe(true)
    expect(killBtn.text()).toContain('强制结束')

    // Mock dialog confirmation
    const { handleConfirm, activeDialog } = useDialog()
    const killPromise = killBtn.trigger('click')
    await flushPromises()

    expect(activeDialog.value?.open).toBe(true)
    expect(activeDialog.value?.severity).toBe('danger')
    handleConfirm()
    await killPromise
    await flushPromises()

    expect(killSpy).toHaveBeenCalledWith(2026)
  })
})

describe('Launch store instance selection persistence', () => {
  let storage: Record<string, string> = {}

  beforeEach(() => {
    setActivePinia(createPinia())
    setupMockIpc()
    resetMockLaunchData()
    storage = {}
    Object.defineProperty(window, 'localStorage', {
      value: {
        getItem: (k: string) => (k in storage ? storage[k]! : null),
        setItem: (k: string, v: string) => {
          storage[k] = String(v)
        },
        removeItem: (k: string) => {
          delete storage[k]
        },
        clear: () => {
          storage = {}
        },
      },
      writable: true,
      configurable: true,
    })
  })

  it('persists the chosen instance id and restores it in a fresh store', async () => {
    const instancesStore = useInstancesStore()
    await instancesStore.fetchInstances()
    expect(instancesStore.instances.length).toBeGreaterThan(1)

    const target = instancesStore.instances[1]!
    const launchStore = useLaunchStore()
    launchStore.selectInstance(target.id)
    expect(storage['mclr.launch.instanceId']).toBe(target.id)

    // Fresh pinia = fresh store, mimicking an app restart.
    setActivePinia(createPinia())
    const restartedInstances = useInstancesStore()
    await restartedInstances.fetchInstances()
    const restartedLaunch = useLaunchStore()
    expect(restartedLaunch.selectedInstance?.id).toBe(target.id)
  })

  it('falls back to the first instance when the persisted id no longer exists', async () => {
    storage['mclr.launch.instanceId'] = 'deleted-outside-the-app'
    setActivePinia(createPinia())

    const instancesStore = useInstancesStore()
    await instancesStore.fetchInstances()
    const launchStore = useLaunchStore()

    expect(instancesStore.instances.length).toBeGreaterThan(0)
    // Fallback order: pinned instance first, otherwise the first instance.
    const expected =
      instancesStore.instances.find((i) => i.config?.pinned)?.id ?? instancesStore.instances[0]!.id
    expect(launchStore.selectedInstance?.id).toBe(expected)
    expect(launchStore.lastUsedInstanceId).toBe(expected)
    // The dangling pointer is healed on disk.
    await flushPromises()
    expect(storage['mclr.launch.instanceId']).toBe(expected)
  })

  it('stacks the instance and account selectors full width instead of a fixed 224px row', async () => {
    const instancesStore = useInstancesStore()
    await instancesStore.fetchInstances()
    const accountsStore = useAccountsStore()
    await accountsStore.fetchAccounts()

    const wrapper = mount(LaunchPage, {
      global: { plugins: [i18n, router] },
    })
    await flushPromises()

    // Regression: `class="w-56"` inside a half-width card overflowed the panel.
    for (const id of ['launch-instance-select', 'launch-account-select']) {
      const select = wrapper.find(`[data-testid="${id}"]`)
      expect(select.exists(), id).toBe(true)
      expect(select.classes(), id).toContain('w-full')
      expect(select.classes(), id).not.toContain('w-56')
    }
  })
})

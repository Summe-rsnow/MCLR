import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { mount, flushPromises } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createRouter, createWebHistory } from 'vue-router'
import { useLogsStore } from '../store'
import LogsPage from '../LogsPage.vue'
import { useDialog } from '../../../ui/useDialog'
import { setupMockIpc } from '../../../ipc/mock'
import { ipcClient } from '../../../ipc/client'
import { resetMockLaunchData, mockRunningProcesses } from '../../launch/mock'
import zhCnLogs from '../../../locales/zh-CN/logs.json'
import zhCnCommon from '../../../locales/zh-CN/common.json'
import zhCnTasks from '../../../locales/zh-CN/tasks.json'

const i18n = createI18n({
  legacy: false,
  locale: 'zh-CN',
  messages: {
    'zh-CN': {
      logs: zhCnLogs,
      common: zhCnCommon,
      tasks: zhCnTasks,
    },
  },
})
const router = createRouter({
  history: createWebHistory(),
  routes: [{ path: '/logs', component: LogsPage }],
})

describe('Logs feature', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    for (const proc of mockRunningProcesses) {
      proc.running = true
      proc.exitCode = null
    }
    resetMockLaunchData()
    setupMockIpc()
  })

  it('loads logs and filters by level and search text', async () => {
    const store = useLogsStore()
    await store.fetchLogs()

    expect(store.logs.length).toBeGreaterThan(0)
    expect(store.filteredLogs.length).toBe(store.logs.length)

    store.levelFilter = 'warn'
    expect(store.filteredLogs.every((l) => l.level === 'warn')).toBe(true)

    store.levelFilter = 'all'
    store.searchQuery = 'GLFW'
    expect(store.filteredLogs.length).toBe(1)
    expect(store.filteredLogs[0].text).toContain('GLFW')

    store.clearLogs()
    expect(store.logs.length).toBe(0)
  })

  it('can kill active running process', async () => {
    const store = useLogsStore()
    await store.fetchLogs()

    expect(store.activeProcess).not.toBeNull()
    const pid = store.activeProcess!.processId

    await store.killRunningProcess(pid)
    expect(store.activeProcess).toBeNull()
  })

  it('renders LogsPage with virtualized entries', async () => {
    await router.push('/logs')
    const wrapper = mount(LogsPage, {
      global: {
        plugins: [i18n, router],
      },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('游戏日志')
    expect(wrapper.text()).toContain('启动器日志 (2)')
    expect(wrapper.text()).toContain('任务列表')
    expect(wrapper.text()).toContain('Loading Minecraft 1.20.1')
  })

  it('prompts confirmDanger when clicking clear logs button', async () => {
    await router.push('/logs')
    const { activeDialog, handleConfirm } = useDialog()
    const wrapper = mount(LogsPage, {
      global: {
        plugins: [i18n, router],
      },
    })
    await flushPromises()

    const clearBtn = wrapper.find('[data-testid="clear-logs-btn"]')
    expect(clearBtn.exists()).toBe(true)

    await clearBtn.trigger('click')
    await flushPromises()

    expect(activeDialog.value?.open).toBe(true)
    expect(activeDialog.value?.severity).toBe('danger')

    handleConfirm()
    await flushPromises()

    const store = useLogsStore()
    expect(store.logs.length).toBe(0)
  })

  it('prompts confirmDanger when clicking kill process button', async () => {
    await router.push('/logs')
    const { activeDialog, handleConfirm } = useDialog()
    const wrapper = mount(LogsPage, {
      global: {
        plugins: [i18n, router],
      },
    })
    await flushPromises()

    const killBtn = wrapper.find('[data-testid="kill-process-btn"]')
    expect(killBtn.exists()).toBe(true)

    await killBtn.trigger('click')
    await flushPromises()

    expect(activeDialog.value?.open).toBe(true)
    expect(activeDialog.value?.severity).toBe('danger')

    handleConfirm()
    await flushPromises()

    const store = useLogsStore()
    expect(store.activeProcess).toBeNull()
  })

  it('renders switch to launcher logs button when logsStore.entries is empty', async () => {
    await router.push('/logs')
    const wrapper = mount(LogsPage, {
      global: {
        plugins: [i18n, router],
      },
    })
    await flushPromises()

    const store = useLogsStore()
    store.clearLogs()
    await flushPromises()

    const switchBtn = wrapper.find('[data-testid="switch-to-launcher-logs-btn"]')
    expect(switchBtn.exists()).toBe(true)
    expect(switchBtn.text()).toContain('查看启动器日志')

    await switchBtn.trigger('click')
    await flushPromises()

    expect(wrapper.find('[data-testid="launcher-log-content"]').exists()).toBe(true)
  })
  it('renders launcher logs tab when route has tab=launcher query', async () => {
    await router.push('/logs?tab=launcher')
    const wrapper = mount(LogsPage, {
      global: {
        plugins: [i18n, router],
      },
    })

    await flushPromises()

    expect(wrapper.find('[data-testid="launcher-log-content"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="launcher-log-content"]').text()).toContain(
      'MCLR initialized',
    )
    expect(wrapper.text()).not.toContain('Loading Minecraft 1.20.1')
  })

  it('renders tasks tab when route has tab=tasks query', async () => {
    await router.push('/logs?tab=tasks')
    const wrapper = mount(LogsPage, {
      global: {
        plugins: [i18n, router],
      },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('批量下载')
    expect(wrapper.text()).not.toContain('Loading Minecraft 1.20.1')
    expect(wrapper.find('.overflow-y-auto.px-2.py-3').exists()).toBe(true)
  })
  it('formats download.group cleanly without empty parentheses when count is missing', async () => {
    vi.spyOn(ipcClient, 'tasksList').mockResolvedValueOnce([
      {
        id: 'task-test-group',
        parentId: null,
        titleKey: 'tasks.item.download.group',
        titleParams: {},
        state: 'running',
        progress: 0.5,
        speedBps: 1000,
      },
    ])

    await router.push('/logs?tab=tasks')
    const wrapper = mount(LogsPage, {
      global: {
        plugins: [i18n, router],
      },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('批量下载')
    expect(wrapper.text()).not.toContain('批量下载 (')
    expect(wrapper.text()).not.toContain('个项目')
  })
})

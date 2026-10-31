import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { mount, flushPromises } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createRouter, createWebHistory } from 'vue-router'
import { useLogsStore } from '../store'
import LogsPage from '../LogsPage.vue'
import LauncherLogsPanel from '../LauncherLogsPanel.vue'
import { useDialog } from '../../../ui/useDialog'
import { useToast } from '../../../ui/useToast'
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

const mockLauncherLogFile = {
  fileName: 'mclr-2026-10-25.log',
  path: 'C:/Users/User/AppData/Roaming/MCLR/logs/mclr-2026-10-25.log',
  size: 2048,
  modifiedAt: '2026-10-25T10:00:00Z',
}

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

  it('virtualizes launcher log output above the 200-line threshold', async () => {
    const lines = Array.from({ length: 250 }, (_, i) => `[INFO] launcher line ${i}`)
    vi.spyOn(ipcClient, 'launcherLogsList').mockResolvedValue([mockLauncherLogFile])
    vi.spyOn(ipcClient, 'launcherLogRead').mockResolvedValue(lines.join('\n'))

    const wrapper = mount(LauncherLogsPanel, { global: { plugins: [i18n] } })
    await flushPromises()

    // The scroller keeps its stable test id so the log panel stays addressable.
    expect(wrapper.find('[data-testid="launcher-log-content"]').exists()).toBe(true)
    // Past the threshold the unified MVirtualList renders the virtualized branch.
    expect(wrapper.find('[data-testid="m-virtual-list"]').exists()).toBe(true)
    const rows = wrapper.findAll('[data-testid="launcher-log-content"] .whitespace-pre-wrap')
    expect(rows.length).toBeLessThan(lines.length)
    vi.restoreAllMocks()
  })

  it('renders short launcher logs plainly without the virtualized wrapper', async () => {
    vi.spyOn(ipcClient, 'launcherLogsList').mockResolvedValue([mockLauncherLogFile])
    vi.spyOn(ipcClient, 'launcherLogRead').mockResolvedValue('first line\nsecond line')

    const wrapper = mount(LauncherLogsPanel, { global: { plugins: [i18n] } })
    await flushPromises()

    expect(wrapper.find('[data-testid="m-virtual-list"]').exists()).toBe(false)
    const rows = wrapper.findAll('[data-testid="launcher-log-content"] .whitespace-pre-wrap')
    expect(rows.length).toBe(2)
    expect(rows[0].text()).toBe('first line')
    expect(rows[1].text()).toBe('second line')
    vi.restoreAllMocks()
  })

  it('prunes and clears launcher logs from the launcher tab', async () => {
    await router.push('/logs?tab=launcher')
    const { activeDialog, handleConfirm } = useDialog()
    const toasts = useToast().toasts
    toasts.value.splice(0)
    const wrapper = mount(LogsPage, {
      global: {
        plugins: [i18n, router],
      },
    })
    await flushPromises()

    const pruneSpy = vi
      .spyOn(ipcClient, 'launcherLogsPrune')
      .mockResolvedValue({ removed: 3, skipped: 0 })
    const clearSpy = vi
      .spyOn(ipcClient, 'launcherLogsClear')
      .mockResolvedValue({ removed: 2, skipped: 1 })

    await wrapper.find('[data-testid="launcher-logs-prune"]').trigger('click')
    await flushPromises()
    expect(pruneSpy).toHaveBeenCalled()
    expect(toasts.value.at(-1)?.message).toBe('已清理 3 个过期日志')

    await wrapper.find('[data-testid="launcher-logs-clear"]').trigger('click')
    await flushPromises()
    // Destructive: it must ask first, then delete on confirm.
    expect(activeDialog.value?.severity).toBe('danger')
    expect(clearSpy).not.toHaveBeenCalled()

    handleConfirm()
    await flushPromises()
    expect(clearSpy).toHaveBeenCalled()
    expect(toasts.value.at(-1)?.message).toBe('已删除 2 个日志文件')
  })

  it('opens the log folder through path_open', async () => {
    await router.push('/logs?tab=launcher')
    const openSpy = vi.spyOn(ipcClient, 'pathOpen').mockResolvedValue()
    const dirSpy = vi.spyOn(ipcClient, 'launcherLogsDir')
    const wrapper = mount(LogsPage, {
      global: {
        plugins: [i18n, router],
      },
    })
    await flushPromises()

    await wrapper.find('[data-testid="launcher-logs-open-folder"]').trigger('click')
    await flushPromises()

    expect(dirSpy).toHaveBeenCalled()
    expect(openSpy).toHaveBeenCalledWith(expect.stringContaining('logs'))
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

  it('flushes 10000 queued lines in under 50ms', async () => {
    const store = useLogsStore()

    store.enqueueLogs(makeLines(0, 10000))
    const start = performance.now()
    store.flushNow()
    const elapsed = performance.now() - start

    expect(store.logs.length).toBe(10000)
    expect(store.filteredLogs.length).toBe(10000)
    expect(elapsed).toBeLessThan(50)
  })

  it('grows filteredLogs by only the new lines on append with an unchanged filter', async () => {
    const store = useLogsStore()

    store.enqueueLogs(makeLines(0, 20))
    store.flushNow()

    const before = store.filteredLogs
    expect(before.length).toBe(20)

    store.enqueueLogs(makeLines(20, 20))
    store.flushNow()

    expect(store.filteredLogs.length).toBe(40)
    // 旧条目按引用保留，不做全量重过滤
    for (let i = 0; i < before.length; i++) {
      expect(store.filteredLogs[i]).toBe(before[i])
    }
  })

  it('caps logs at 10000 and trims the filtered prefix in step', async () => {
    const store = useLogsStore()

    store.enqueueLogs(makeLines(0, 10050))
    store.flushNow()

    expect(store.logs.length).toBe(10000)
    expect(store.filteredLogs.length).toBe(10000)
    expect(store.filteredLogs[0]).toBe(store.logs[0])
  })
})

function makeLines(from: number, count: number) {
  return Array.from({ length: count }, (_, i) => ({
    lineId: from + i,
    processId: 1,
    instanceId: '1.20.1-vanilla',
    level: 'info' as const,
    text: `line ${from + i}`,
    timestamp: '2026-01-01T00:00:00.000Z',
  }))
}

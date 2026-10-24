import { describe, it, expect, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { mount, flushPromises } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { useLogsStore } from '../store'
import LogsPage from '../LogsPage.vue'
import { setupMockIpc } from '../../../ipc/mock'
import { resetMockLaunchData } from '../../launch/mock'
import zhCnLogs from '../../../locales/zh-CN/logs.json'
import zhCnCommon from '../../../locales/zh-CN/common.json'

const i18n = createI18n({
  legacy: false,
  locale: 'zh-CN',
  messages: {
    'zh-CN': {
      logs: zhCnLogs,
      common: zhCnCommon,
    },
  },
})

describe('Logs feature', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
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
    const wrapper = mount(LogsPage, {
      global: {
        plugins: [i18n],
      },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('游戏运行日志')
    expect(wrapper.text()).toContain('Loading Minecraft 1.20.1')
  })
})

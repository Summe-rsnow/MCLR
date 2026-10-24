import { describe, it, expect, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { mount, flushPromises } from '@vue/test-utils'
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

const i18n = createI18n({
  legacy: false,
  locale: 'zh-CN',
  messages: {
    'zh-CN': {
      launch: zhCnLaunch,
      common: zhCnCommon,
      settingsGame: zhCnSettingsGame,
    },
  },
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
        plugins: [i18n],
      },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('游戏启动')
    expect(wrapper.text()).toContain('启动游戏')
    expect(wrapper.text()).toContain('Vanilla 1.20.1')
    expect(wrapper.text()).toContain('StevePro')
  })
})

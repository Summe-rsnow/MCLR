import { describe, it, expect, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { mount, flushPromises } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import SettingsGamePage from '../SettingsGamePage.vue'
import { setupMockIpc } from '../../../ipc/mock'
import { ipcClient } from '../../../ipc/client'
import zhCnSettingsGame from '../../../locales/zh-CN/settingsGame.json'
import zhCnCommon from '../../../locales/zh-CN/common.json'

const i18n = createI18n({
  legacy: false,
  locale: 'zh-CN',
  messages: {
    'zh-CN': {
      settingsGame: zhCnSettingsGame,
      common: zhCnCommon,
    },
  },
})

describe('Settings Game feature', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    setupMockIpc()
  })

  it('renders SettingsGamePage with launch settings without java tab', async () => {
    const wrapper = mount(SettingsGamePage, {
      global: {
        plugins: [i18n],
      },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('游戏运行与环境设置')
    expect(wrapper.text()).toContain('内存分配')
    expect(wrapper.text()).toContain('游戏窗口与显示')
    expect(wrapper.find('[data-testid="tab-launch"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="tab-java"]').exists()).toBe(false)
  })

  it('patches launch settings correctly', async () => {
    const updated = await ipcClient.settingsPatch({
      launch: {
        ramMode: 'custom',
        ramCustomMb: 8192,
        fullscreen: true,
      },
    })

    expect(updated.launch?.ramMode).toBe('custom')
    expect(updated.launch?.ramCustomMb).toBe(8192)
    expect(updated.launch?.fullscreen).toBe(true)
  })
})

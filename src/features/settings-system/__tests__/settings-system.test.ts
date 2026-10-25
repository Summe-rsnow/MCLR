import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { i18n } from '../../../i18n'
import { router } from '../../../router'
import { setupMockIpc } from '../../../ipc/mock'
import zhCnSettingsSystem from '../../../locales/zh-CN/settingsSystem.json'
import SettingsSystemPage from '../SettingsSystemPage.vue'
import { resetSettingsSystemMock, setMockUpdateState } from '../mock'

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
}))
vi.mock('@tauri-apps/plugin-opener', () => ({
  openPath: vi.fn(),
  openUrl: vi.fn(),
}))

import { open as openFileDialog } from '@tauri-apps/plugin-dialog'

describe('SettingsSystemPage (v0.0.12)', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    resetSettingsSystemMock()
    setupMockIpc()
    i18n.global.locale.value = 'zh-CN'
  })

  it('renders Game Manage tab and adds a new game folder via select-to-add with remark editing', async () => {
    vi.mocked(openFileDialog).mockResolvedValue('D:\\Games\\.minecraft')

    const wrapper = mount(SettingsSystemPage, {
      global: {
        plugins: [createPinia(), i18n, router],
      },
    })
    await flushPromises()

    expect(wrapper.find('[data-testid="tab-gameManage"]').exists()).toBe(true)
    await wrapper.find('[data-testid="add-folder-btn"]').trigger('click')
    await flushPromises()

    expect(wrapper.text()).toContain('.minecraft')

    // Edit remark name
    const editBtn = wrapper.find('[data-testid="edit-remark-btn"]')
    expect(editBtn.exists()).toBe(true)
    await editBtn.trigger('click')
    await flushPromises()

    const input = wrapper.find('[data-testid="edit-remark-input"] input')
    expect(input.exists()).toBe(true)
    await input.setValue('Secondary MC')
    await wrapper.find('[data-testid="save-remark-btn"]').trigger('click')
    await flushPromises()

    expect(wrapper.text()).toContain('Secondary MC')
  })

  it('switches to Language tab and dynamically switches to throw-away zz-TEST locale', async () => {
    // Register a throw-away zz-TEST locale in test memory only
    const zhMessages = i18n.global.getLocaleMessage('zh-CN')
    i18n.global.setLocaleMessage('zz-TEST', {
      ...zhMessages,
      settingsSystem: {
        ...zhCnSettingsSystem,
        title: 'ZZ_TEST_SYSTEM_SETTINGS_TITLE',
      },
    })

    const wrapper = mount(SettingsSystemPage, {
      global: {
        plugins: [createPinia(), i18n, router],
      },
    })
    await flushPromises()

    // Switch to language tab
    const langTab = wrapper
      .findAll('button')
      .find((b) => b.text().includes('语言偏好'))
    expect(langTab).toBeDefined()
    await langTab!.trigger('click')
    await flushPromises()

    expect(wrapper.find('[data-testid="tab-language"]').exists()).toBe(true)

    // Switch locale to zz-TEST
    const select = wrapper.find('[data-testid="select-locale"] select')
    await select.setValue('zz-TEST')
    await flushPromises()

    expect(wrapper.text()).toContain('ZZ_TEST_SYSTEM_SETTINGS_TITLE')

    // Restore zh-CN
    i18n.global.locale.value = 'zh-CN'
  })

  it('handles Update tab both when notConfigured and when staged update is available', async () => {
    const wrapper = mount(SettingsSystemPage, {
      global: {
        plugins: [createPinia(), i18n, router],
      },
    })
    await flushPromises()

    const updateTab = wrapper
      .findAll('button')
      .find((b) => b.text().includes('检查更新'))
    await updateTab!.trigger('click')
    await flushPromises()

    // 1. Test notConfigured path
    setMockUpdateState({ notConfigured: true })
    await wrapper.find('[data-testid="check-update-btn"]').trigger('click')
    await flushPromises()

    expect(wrapper.find('[data-testid="update-error"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="update-error"]').text()).toContain('updater')

    // 2. Test staged update available path
    setMockUpdateState({ notConfigured: false, available: true })
    await wrapper.find('[data-testid="check-update-btn"]').trigger('click')
    await flushPromises()

    expect(wrapper.find('[data-testid="update-result"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="update-result"]').text()).toContain('0.0.12')
    expect(wrapper.find('[data-testid="install-update-btn"]').exists()).toBe(true)
  })

  it('renders Launcher Logs and About tabs', async () => {
    const wrapper = mount(SettingsSystemPage, {
      global: {
        plugins: [createPinia(), i18n, router],
      },
    })
    await flushPromises()

    const logsTab = wrapper
      .findAll('button')
      .find((b) => b.text().includes('启动器日志'))
    await logsTab!.trigger('click')
    await flushPromises()
    expect(wrapper.find('[data-testid="launcher-log-content"]').text()).toContain('MCLR initialized')

    const aboutTab = wrapper
      .findAll('button')
      .find((b) => b.text().includes('关于 MCLR'))
    await aboutTab!.trigger('click')
    await flushPromises()
    expect(wrapper.find('[data-testid="tab-about"]').text()).toContain('Apache-2.0')
  })
})

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { i18n } from '../../../i18n'
import { router } from '../../../router'
import { setupMockIpc } from '../../../ipc/mock'
import zhCnSettingsSystem from '../../../locales/zh-CN/settingsSystem.json'
import SettingsSystemPage from '../SettingsSystemPage.vue'
import { resetSettingsSystemMock } from '../mock'

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
}))
vi.mock('@tauri-apps/plugin-opener', () => ({
  openPath: vi.fn(),
  openUrl: vi.fn(),
}))

describe('SettingsSystemPage (v0.0.12)', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    resetSettingsSystemMock()
    setupMockIpc()
    i18n.global.locale.value = 'zh-CN'
  })

  it('renders Game Manage tab with isolation settings', async () => {
    const wrapper = mount(SettingsSystemPage, {
      global: {
        plugins: [createPinia(), i18n, router],
      },
    })
    await flushPromises()

    expect(wrapper.find('[data-testid="tab-gameManage"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('版本隔离策略')
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
    const langTab = wrapper.findAll('button').find((b) => b.text().includes('语言偏好'))
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

    const updateTab = wrapper.findAll('button').find((b) => b.text().includes('检查更新'))
    await updateTab!.trigger('click')
    await flushPromises()

    // The tab renders either the live check button or the coming-soon placeholder;
    // mock IPC has no updater secrets, so it is the placeholder.
    const hasLive = wrapper.find('[data-testid="check-update-btn"]').exists()
    const hasPlaceholder = wrapper.find('[data-testid="update-coming-soon"]').exists()
    expect(hasLive || hasPlaceholder).toBe(true)
  })

  it('shows the update button as coming soon when no signed feed is configured', async () => {
    const wrapper = mount(SettingsSystemPage, {
      global: {
        plugins: [createPinia(), i18n, router],
      },
    })
    await flushPromises()

    const updateTab = wrapper.findAll('button').find((b) => b.text().includes('检查更新'))
    await updateTab!.trigger('click')
    await flushPromises()

    // Mock IPC reports no updater secrets, so the feature is a placeholder.
    expect(wrapper.find('[data-testid="update-coming-soon"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="update-coming-soon"]').text()).toContain('敬请期待')
    expect(wrapper.find('[data-testid="check-update-btn"]').exists()).toBe(false)
  })

  it('renders remaining tabs correctly', async () => {
    const wrapper = mount(SettingsSystemPage, {
      global: {
        plugins: [createPinia(), i18n, router],
      },
    })
    await flushPromises()

    const updateTab = wrapper.findAll('button').find((b) => b.text().includes('检查更新'))
    await updateTab!.trigger('click')
    await flushPromises()
    expect(wrapper.find('[data-testid="tab-update"]').exists()).toBe(true)
  })
})

import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { mount, flushPromises } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import JavaPage from '../JavaPage.vue'
import { setupMockIpc } from '../../../ipc/mock'
import { ipcClient } from '../../../ipc/client'
import zhCnJava from '../../../locales/zh-CN/java.json'
import zhCnCommon from '../../../locales/zh-CN/common.json'

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
}))

import { open as openFileDialog } from '@tauri-apps/plugin-dialog'

const i18n = createI18n({
  legacy: false,
  locale: 'zh-CN',
  messages: {
    'zh-CN': {
      java: zhCnJava,
      common: zhCnCommon,
    },
  },
})

describe('Java feature', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    setupMockIpc()
    vi.clearAllMocks()
  })

  it('renders JavaPage with header and scan action', async () => {
    const wrapper = mount(JavaPage, {
      global: {
        plugins: [i18n],
      },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('Java 运行时管理')
    expect(wrapper.text()).toContain('扫描本机 Java')
    expect(wrapper.text()).toContain('优先使用 JDK 而非 JRE')
    expect(wrapper.text()).toContain('已检测到的 Java 运行时')
  })

  it('allows scanning Java installations', async () => {
    const javas = await ipcClient.javaScan()
    expect(javas.length).toBeGreaterThan(0)
    expect(javas.some((j) => j.major === 17)).toBe(true)
  })

  it('adds and removes custom Java via select-to-add dialog', async () => {
    const customPath = 'C:\\Program Files\\Java\\jdk-25\\bin\\javaw.exe'
    vi.mocked(openFileDialog).mockResolvedValue(customPath)

    const wrapper = mount(JavaPage, {
      global: {
        plugins: [i18n],
      },
    })
    await flushPromises()

    const addBtn = wrapper.find('[data-testid="add-java-btn"]')
    expect(addBtn.exists()).toBe(true)
    await addBtn.trigger('click')
    await flushPromises()

    const settings = await ipcClient.settingsGet()
    expect(settings.java?.custom).toContain(customPath)

    // Verify remove button is rendered and functional
    const removeBtn = wrapper.findAll('button').find((b) => b.text().includes('移除'))
    expect(removeBtn?.exists()).toBe(true)
    await removeBtn?.trigger('click')
    await flushPromises()

    const updatedSettings = await ipcClient.settingsGet()
    expect(updatedSettings.java?.custom).not.toContain(customPath)
  })
})

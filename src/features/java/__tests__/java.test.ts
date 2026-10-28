import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { mount, flushPromises } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import JavaPage from '../JavaPage.vue'
import { setupMockIpc } from '../../../ipc/mock'
import { ipcClient } from '../../../ipc/client'
import { useDialog } from '../../../ui/useDialog'
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
  let storage: Record<string, string> = {}

  beforeEach(() => {
    setActivePinia(createPinia())
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
    // Scan is an icon button titled like the instances-page refresh control.
    expect(wrapper.find('[title="扫描本机 Java"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('优先使用 JDK 而非 JRE')
    expect(wrapper.text()).toContain('已检测到的 Java 运行时')
  })

  it('allows scanning Java installations', async () => {
    const javas = await ipcClient.javaScan()
    expect(javas.length).toBeGreaterThan(0)
    expect(javas.some((j) => j.major === 17)).toBe(true)
  })

  it('opens Java parent folder when folder button is clicked', async () => {
    const pathOpenSpy = vi.spyOn(ipcClient, 'pathOpen').mockResolvedValue(undefined)
    const wrapper = mount(JavaPage, {
      global: {
        plugins: [i18n],
      },
    })
    await flushPromises()

    const folderBtn = wrapper.find('button[title="打开所在文件夹"]')
    expect(folderBtn.exists()).toBe(true)
    await folderBtn.trigger('click')
    await flushPromises()

    expect(pathOpenSpy).toHaveBeenCalled()
  })

  it('adds and removes custom Java via select-to-add dialog with confirmation', async () => {
    const customPath = 'C:\\Program Files\\Java\\jdk-25\\bin\\javaw.exe'
    vi.mocked(openFileDialog).mockResolvedValue(customPath)
    const { handleConfirm } = useDialog()

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

    // Verify remove button is rendered and functional with confirmation
    const removeBtns = wrapper.findAll('button[title="移除"]')
    expect(removeBtns.length).toBeGreaterThan(0)
    // The first item in mockJavaInstallations matches customPath
    const customItemRemoveBtn = removeBtns[0]
    await customItemRemoveBtn.trigger('click')
    handleConfirm()
    await flushPromises()

    const updatedSettings = await ipcClient.settingsGet()
    expect(updatedSettings.java?.custom).not.toContain(customPath)
  })

  it('removes detected Java by adding to hiddenJavaPaths', async () => {
    const { handleConfirm } = useDialog()
    const wrapper = mount(JavaPage, {
      global: {
        plugins: [i18n],
      },
    })
    await flushPromises()

    const initialCount = wrapper.findAll('button[title="移除"]').length
    expect(initialCount).toBeGreaterThan(0)

    const firstRemoveBtn = wrapper.findAll('button[title="移除"]')[0]
    await firstRemoveBtn.trigger('click')
    handleConfirm()
    await flushPromises()

    expect(wrapper.findAll('button[title="移除"]').length).toBe(initialCount - 1)
    expect(localStorage.getItem('mclr.hidden_java_paths')).toBeTruthy()
  })

  it('has no global default-Java control any more', async () => {
    const wrapper = mount(JavaPage, {
      global: {
        plugins: [i18n],
      },
    })
    await flushPromises()

    // Java is resolved per version; a single global pin caused 26.x to run on Java 21.
    const labels = wrapper.findAll('button').map((b) => b.text())
    expect(labels.some((l) => l.includes('设为默认'))).toBe(false)
    expect(wrapper.text()).not.toContain('默认 Java')
    expect(wrapper.text()).toContain('Java 25')
  })
})

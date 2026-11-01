import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { mount, flushPromises } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createRouter, createWebHistory, type Router } from 'vue-router'
import { save as saveFileDialog } from '@tauri-apps/plugin-dialog'

vi.mock('@tauri-apps/plugin-dialog', () => ({
  save: vi.fn(),
  open: vi.fn(),
}))
import { ipcClient } from '../../../ipc/client'
import { useDialog } from '../../../ui/useDialog'
import ToolsPage from '../ToolsPage.vue'
import { resetToolsMock, mockSkinResult, lastCreatedShortcut, lastCustomDownload } from '../mock'
import { setupMockIpc } from '../../../ipc/mock'
import zhCnTools from '../../../locales/zh-CN/tools.json'
import zhCnCommon from '../../../locales/zh-CN/common.json'
import zhCnError from '../../../locales/zh-CN/error.json'

const i18n = createI18n({
  legacy: false,
  locale: 'zh-CN',
  messages: {
    'zh-CN': {
      tools: zhCnTools,
      common: zhCnCommon,
      error: zhCnError,
    },
  },
})

describe('Tools Feature', () => {
  let router: Router

  beforeEach(async () => {
    setActivePinia(createPinia())
    setupMockIpc()
    resetToolsMock()

    router = createRouter({
      history: createWebHistory(),
      routes: [
        { path: '/', component: { template: '<div />' } },
        { path: '/tools', component: ToolsPage },
      ],
    })
    await router.push('/tools')
    await router.isReady()
  })

  it('renders ToolsPage with all 4 tool cards', async () => {
    const wrapper = mount(ToolsPage, {
      global: {
        plugins: [i18n, router],
      },
    })
    await flushPromises()

    expect(wrapper.text()).toContain(zhCnTools.title)
    expect(wrapper.text()).toContain(zhCnTools.skin.title)
    expect(wrapper.text()).toContain(zhCnTools.cache.title)
    expect(wrapper.text()).toContain(zhCnTools.download.title)
    expect(wrapper.text()).toContain(zhCnTools.shortcut.title)
  })
  it('tool 1: skin fetcher fetches player skin and displays preview', async () => {
    const wrapper = mount(ToolsPage, {
      global: {
        plugins: [i18n, router],
      },
    })
    await flushPromises()

    const input = wrapper.find('input[placeholder*="Notch"]')
    expect(input.exists()).toBe(true)
    await input.setValue('Notch')

    const fetchBtn = wrapper.find('[data-testid="fetch-skin-btn"]')
    expect(fetchBtn.exists()).toBe(true)
    await fetchBtn.trigger('click')
    await flushPromises()

    expect(wrapper.text()).toContain('Notch')
    expect(wrapper.text()).toContain(mockSkinResult.uuid)
    expect(wrapper.text()).toContain(zhCnTools.skin.modelClassic)
    expect(wrapper.find('img[alt="Notch"]').exists()).toBe(true)

    const downloadBtn = wrapper.find('[data-testid="download-skin-btn"]')
    expect(downloadBtn.exists()).toBe(true)

    // Test downloadSkinPng invoking toolsSaveSkin
    vi.mocked(saveFileDialog).mockResolvedValue('C:/mock/saved_skin.png')
    const saveSkinSpy = vi.spyOn(ipcClient, 'toolsSaveSkin').mockResolvedValue()

    await downloadBtn.trigger('click')
    await flushPromises()

    expect(saveFileDialog).toHaveBeenCalled()
    expect(saveSkinSpy).toHaveBeenCalledWith('C:/mock/saved_skin.png', mockSkinResult.skinDataUrl)
  })
  it('tool 2: cache cleaner displays size and performs cleanup with danger confirm', async () => {
    const { activeDialog, handleConfirm } = useDialog()
    const wrapper = mount(ToolsPage, {
      global: {
        plugins: [i18n, router],
      },
    })
    await flushPromises()

    expect(wrapper.text()).toContain(zhCnTools.cache.cleanNow)
    // Initially has mockCacheInfo total bytes formatted
    expect(wrapper.text()).toContain('MB')

    const cleanBtn = wrapper.find('[data-testid="clean-cache-btn"]')
    expect(cleanBtn.exists()).toBe(true)
    await cleanBtn.trigger('click')
    await flushPromises()

    expect(activeDialog.value?.open).toBe(true)
    expect(activeDialog.value?.severity).toBe('danger')
    expect(activeDialog.value?.confirmVariant).toBe('danger')

    handleConfirm()
    await flushPromises()

    expect(wrapper.text()).toContain(zhCnTools.cache.cleaned)
  })

  it('tool 3: custom downloader starts download task', async () => {
    const wrapper = mount(ToolsPage, {
      global: {
        plugins: [i18n, router],
      },
    })
    await flushPromises()

    const urlInput = wrapper.find('input[placeholder*="example.com"]')
    expect(urlInput.exists()).toBe(true)
    await urlInput.setValue('https://example.com/optifine.jar')

    const destInput = wrapper.find('input[placeholder*="保存路径"]')
    expect(destInput.exists()).toBe(true)
    await destInput.setValue('C:/downloads/optifine.jar')

    const downloadBtn = wrapper.find('[data-testid="custom-download-btn"]')
    expect(downloadBtn.exists()).toBe(true)
    await downloadBtn.trigger('click')
    await flushPromises()

    expect(lastCustomDownload).toEqual({
      url: 'https://example.com/optifine.jar',
      destPath: 'C:/downloads/optifine.jar',
    })
  })

  it('tool 5: failed-task leftovers are their own category and show 0 B once cleaned', async () => {
    resetToolsMock()
    const { handleConfirm } = useDialog()
    const wrapper = mount(ToolsPage, { global: { plugins: [i18n, router] } })
    await flushPromises()

    // Same row shape as the other three, and selected by default.
    const row = wrapper
      .findAll('.m-well')
      .find((el) => el.text().includes(zhCnTools.cache.failedTasks))
    expect(row).toBeDefined()
    expect(row!.text()).toContain('45.8 MB')
    expect(row!.find('input[type="checkbox"]').element).toHaveProperty('checked', true)

    // Only this category selected: the whole-cache wipe must not run.
    const cacheRow = wrapper
      .findAll('.m-well')
      .find((el) => el.text().includes(zhCnTools.cache.itemCache))!
    const cacheBox = cacheRow.find('input[type="checkbox"]')
    await cacheBox.setValue(false)
    await flushPromises()

    const cacheClearSpy = vi.spyOn(ipcClient, 'toolsCacheClear')
    const cleanupSpy = vi.spyOn(ipcClient, 'toolsCleanupFailedTasks')

    await wrapper.find('[data-testid="clean-cache-btn"]').trigger('click')
    await flushPromises()
    handleConfirm()
    await flushPromises()

    expect(cleanupSpy).toHaveBeenCalledTimes(1)
    expect(cacheClearSpy).not.toHaveBeenCalled()
    const reRead = wrapper
      .findAll('.m-well')
      .find((el) => el.text().includes(zhCnTools.cache.failedTasks))!
    expect(reRead.text()).toContain('0 B')

    cacheClearSpy.mockRestore()
    cleanupSpy.mockRestore()
  })

  it('tool 4: desktop shortcut creates .lnk for instance', async () => {
    const wrapper = mount(ToolsPage, {
      global: {
        plugins: [i18n, router],
      },
    })
    await flushPromises()

    const nameInput = wrapper.find('input[placeholder*="Minecraft - 实例名称"]')
    expect(nameInput.exists()).toBe(true)
    await nameInput.setValue('My Vanilla Game')

    const shortcutBtn = wrapper.find('[data-testid="create-shortcut-btn"]')
    expect(shortcutBtn.exists()).toBe(true)
    await shortcutBtn.trigger('click')
    await flushPromises()

    expect(lastCreatedShortcut).toEqual({
      instanceId: '1.20.1-vanilla',
      shortcutName: 'My Vanilla Game',
    })
  })
})

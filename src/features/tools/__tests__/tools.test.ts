import { describe, it, expect, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { mount, flushPromises } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createRouter, createWebHistory, type Router } from 'vue-router'
import ToolsPage from '../ToolsPage.vue'
import {
  resetToolsMock,
  mockSkinResult,
  lastCreatedShortcut,
  lastCustomDownload,
} from '../mock'
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
  })

  it('tool 2: cache cleaner displays size and performs cleanup', async () => {
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

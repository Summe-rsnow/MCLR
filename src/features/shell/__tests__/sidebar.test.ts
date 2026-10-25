import { describe, it, expect, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { mount, flushPromises } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createRouter, createMemoryHistory, type Router } from 'vue-router'
import SidebarNav from '../SidebarNav.vue'
import { featureRoutes } from '../../../router'
import zhCnShell from '../../../locales/zh-CN/shell.json'
import zhCnApp from '../../../locales/zh-CN/app.json'

const i18n = createI18n({
  legacy: false,
  locale: 'zh-CN',
  messages: {
    'zh-CN': {
      shell: zhCnShell,
      app: zhCnApp,
      comp: { nav: '组件库' },
      accounts: { nav: '账号' },
    },
  },
})

describe('SidebarNav component', () => {
  let router: Router
  let storage: Record<string, string> = {}

  beforeEach(() => {
    setActivePinia(createPinia())
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
    router = createRouter({
      history: createMemoryHistory(),
      routes: [...featureRoutes, { path: '/', redirect: '/launch' }],
    })
  })

  it('renders navigation items without dev-ui in nav', async () => {
    router.push('/launch')
    await router.isReady()

    const wrapper = mount(SidebarNav, {
      global: {
        plugins: [i18n, router],
      },
    })
    await flushPromises()

    // dev-ui and tasks should not appear in sidebar
    expect(wrapper.text()).not.toContain('组件调试')
    expect(wrapper.text()).not.toContain('dev-ui')
    expect(wrapper.find('button[title*="任务"]').exists()).toBe(false)
  })

  it('activates comp nav item when on comp subroutes', async () => {
    router.push('/comp/modpack')
    await router.isReady()

    const wrapper = mount(SidebarNav, {
      global: {
        plugins: [i18n, router],
      },
    })
    await flushPromises()

    const buttons = wrapper.findAll('button')
    const compButton = buttons.find(
      (btn) => btn.text().includes('组件') || btn.html().includes('i-lucide-store'),
    )
    expect(compButton?.classes()).toContain('text-primary-soft-fg')
  })

  it('activates settings nav item when on settings subroute', async () => {
    router.push('/settings/appearance')
    await router.isReady()

    const wrapper = mount(SidebarNav, {
      global: {
        plugins: [i18n, router],
      },
    })
    await flushPromises()

    const buttons = wrapper.findAll('button')
    const settingsButton = buttons.find((btn) => btn.html().includes('i-lucide-settings'))
    expect(settingsButton?.classes()).toContain('text-primary-soft-fg')
  })

  it('renders nav groups with gap-1.5 spacing between items', async () => {
    router.push('/launch')
    await router.isReady()

    const wrapper = mount(SidebarNav, {
      global: {
        plugins: [i18n, router],
      },
    })
    await flushPromises()

    const navGroupDivs = wrapper.findAll('.flex-col.gap-1\\.5')
    expect(navGroupDivs.length).toBeGreaterThan(0)
  })

  it('renders MLogo and MCLR text when expanded, and no MCLR text when collapsed', async () => {
    localStorage.setItem('mclr.sidebar', 'expanded')
    router.push('/launch')
    await router.isReady()

    const expandedWrapper = mount(SidebarNav, {
      global: {
        plugins: [i18n, router],
      },
    })
    await flushPromises()

    // Expanded state: SVG logo exists with w-7 h-7 text-primary, text contains MCLR
    const logoSvg = expandedWrapper.find('svg')
    expect(logoSvg.exists()).toBe(true)
    expect(logoSvg.classes()).toContain('w-7')
    expect(logoSvg.classes()).toContain('h-7')
    expect(logoSvg.classes()).toContain('text-primary')
    expect(expandedWrapper.text()).toContain('MCLR')
    // Collapsed state
    localStorage.setItem('mclr.sidebar', 'collapsed')
    const collapsedWrapper = mount(SidebarNav, {
      global: {
        plugins: [i18n, router],
      },
    })
    await flushPromises()

    expect(collapsedWrapper.find('svg').exists()).toBe(true)
    expect(collapsedWrapper.text()).not.toContain('MCLR')
    localStorage.removeItem('mclr.sidebar')
  })
})

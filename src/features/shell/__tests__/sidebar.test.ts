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

  beforeEach(() => {
    setActivePinia(createPinia())
    router = createRouter({
      history: createMemoryHistory(),
      routes: [
        ...featureRoutes,
        { path: '/', redirect: '/launch' },
      ],
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

    // dev-ui should not have nav metadata and should not appear in sidebar
    expect(wrapper.text()).not.toContain('组件调试')
    expect(wrapper.text()).not.toContain('dev-ui')
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
    const compButton = buttons.find((btn) => btn.text().includes('组件') || btn.html().includes('i-lucide-store'))
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
})

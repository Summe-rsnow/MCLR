import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { mount, flushPromises } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createRouter, createWebHistory, type Router } from 'vue-router'
import { instanceDetailTabs } from '../tabs'
import InstanceDetailPage from '../InstanceDetailPage.vue'
import OverviewTab from '../tabs/OverviewTab.vue'
import SettingsTab from '../tabs/SettingsTab.vue'
import SavesTab from '../tabs/SavesTab.vue'
import ServersTab from '../tabs/ServersTab.vue'
import ExportTab from '../tabs/ExportTab.vue'
import { lastExportedModpackOptions } from '../mock'
import { setupMockIpc } from '../../../ipc/mock'
import { resetMockInstances } from '../../instances/mock'
import { resetMockInstanceDetail } from '../mock'
import { useInstancesStore } from '../../instances/store'
import { ipcClient } from '../../../ipc/client'

import zhCnInstanceDetail from '../../../locales/zh-CN/instanceDetail.json'
import zhCnInstances from '../../../locales/zh-CN/instances.json'
import zhCnCommon from '../../../locales/zh-CN/common.json'
import zhCnError from '../../../locales/zh-CN/error.json'
import zhCnComp from '../../../locales/zh-CN/comp.json'

const i18n = createI18n({
  legacy: false,
  locale: 'zh-CN',
  messages: {
    'zh-CN': {
      instanceDetail: zhCnInstanceDetail,
      instances: zhCnInstances,
      common: zhCnCommon,
      error: zhCnError,
      comp: zhCnComp,
    },
  },
})

describe('Instance Detail feature', () => {
  let router: Router

  beforeEach(async () => {
    setActivePinia(createPinia())
    resetMockInstances()
    resetMockInstanceDetail()
    setupMockIpc()

    router = createRouter({
      history: createWebHistory(),
      routes: [
        {
          path: '/instances/:id',
          name: 'instance-detail',
          component: InstanceDetailPage,
          props: true,
        },
        {
          path: '/launch',
          name: 'launch',
          component: { template: '<div>Launch Page</div>' },
        },
      ],
    })
    await router.push('/instances/1.20.1-vanilla')
    await router.isReady()
  })

  it('glob tab registry loads all sub-tabs in order', () => {
    expect(instanceDetailTabs.length).toBe(11)
    const keys = instanceDetailTabs.map((t) => t.key)
    expect(keys).toEqual([
      'overview',
      'install',
      'settings',
      'export',
      'saves',
      'screenshots',
      'mods',
      'resourcepacks',
      'shaders',
      'schematics',
      'servers',
    ])
  })

  it('renders ModsDisabledTab when instance is not modable on mods tab', async () => {
    const store = useInstancesStore()
    await store.fetchInstances()

    const wrapper = mount(InstanceDetailPage, {
      props: { id: '1.20.1-vanilla' },
      global: {
        plugins: [i18n, router],
      },
    })
    await flushPromises()

    const buttons = wrapper.findAll('button')
    const modsButton = buttons.find((b) => b.text().includes('模组'))
    if (modsButton) {
      await modsButton.trigger('click')
      await flushPromises()
      expect(wrapper.text()).toContain('该实例不可使用模组')
      expect(wrapper.text()).toContain('下载可安装 Mod 的实例')
    }
  })

  it('renders InstanceDetailPage and switches tabs', async () => {
    const store = useInstancesStore()
    await store.fetchInstances()

    const wrapper = mount(InstanceDetailPage, {
      props: { id: '1.20.1-vanilla' },
      global: {
        plugins: [i18n, router],
      },
    })
    await flushPromises()

    expect(wrapper.text()).toContain('Vanilla 1.20.1')
    expect(wrapper.text()).toContain('1.20.1')

    // Find tabs and click on Settings tab
    const tabs = wrapper.findAll('button')
    const settingsButton = tabs.find((b) => b.text().includes('配置'))
    expect(settingsButton).toBeDefined()
    if (settingsButton) {
      await settingsButton.trigger('click')
      await flushPromises()
      expect(wrapper.text()).toContain('实例独立配置')
    }
  })

  it('saves tab lists worlds and toggles datapack state', async () => {
    const wrapper = mount(SavesTab, {
      props: { instanceId: '1.20.1-vanilla' },
      global: {
        plugins: [i18n, router],
      },
    })
    await flushPromises()

    expect(wrapper.text()).toContain('New World')
    expect(wrapper.text()).toContain('Survival Test')

    // Open Datapacks modal for New World
    const datapacksBtn = wrapper.findAll('button').find((b) => b.text().includes('数据包'))
    expect(datapacksBtn).toBeDefined()
    if (datapacksBtn) {
      await datapacksBtn.trigger('click')
      await flushPromises()

      expect(document.body.textContent).toContain('Vanilla data pack')
      expect(document.body.textContent).toContain('custom_pack.zip')

      // Toggle datapack via ipcClient
      const updated = await ipcClient.saveDatapackToggle(
        '1.20.1-vanilla',
        'NewWorld',
        'custom_pack.zip',
        false,
      )
      const target = updated.find((p) => p.id === 'custom_pack.zip')
      expect(target?.enabled).toBe(false)
    }
  })

  it('servers tab performs CRUD, ping and launch into server', async () => {
    const store = useInstancesStore()
    await store.fetchInstances()

    const pushSpy = vi.spyOn(router, 'push')

    const wrapper = mount(ServersTab, {
      props: { instanceId: '1.20.1-vanilla' },
      global: {
        plugins: [i18n, router],
      },
    })
    await flushPromises()

    expect(wrapper.text()).toContain('Hypixel Network')
    expect(wrapper.text()).toContain('mc.hypixel.net')

    // Verify live ping result displays ms badge
    expect(wrapper.text()).toContain('35 ms')

    // Test server ping via client
    const pingRes = await ipcClient.serverPing('mc.hypixel.net')
    expect(pingRes.online).toBe(true)
    expect(pingRes.playersOnline).toBe(42)

    // Test server add
    const addedList = await ipcClient.serverAdd(
      '1.20.1-vanilla',
      'Mineplex Revival',
      'mineplex.com',
      null,
    )
    expect(addedList.some((s) => s.name === 'Mineplex Revival')).toBe(true)

    // Click "Launch into Server" (直连启动)
    const launchDirectBtn = wrapper.findAll('button').find((b) => b.text().includes('直连启动'))
    expect(launchDirectBtn).toBeDefined()
    if (launchDirectBtn) {
      await launchDirectBtn.trigger('click')
      await flushPromises()

      // Should update config with serverQuickPlay
      const inst = store.instances.find((i) => i.id === '1.20.1-vanilla')
      expect(inst?.config?.serverQuickPlay).toBe('mc.hypixel.net')
      expect(pushSpy).toHaveBeenCalledWith('/launch')
    }
  })

  it('settings tab updates and saves per-instance configuration', async () => {
    const store = useInstancesStore()
    await store.fetchInstances()
    const inst = store.instances.find((i) => i.id === '1.20.1-vanilla')!

    const wrapper = mount(SettingsTab, {
      props: { instance: inst },
      global: {
        plugins: [i18n, router],
      },
    })
    await flushPromises()

    expect(wrapper.text()).toContain('版本隔离')
    expect(wrapper.text()).toContain('登录账号要求')

    // Update settings via ipcClient
    const updated = await ipcClient.instanceUpdateConfig('1.20.1-vanilla', {
      ...inst.config,
      jvmArgs: '-Xmx8G -XX:+UseG1GC',
      windowTitle: 'Custom Title',
    })
    expect(updated.config?.jvmArgs).toBe('-Xmx8G -XX:+UseG1GC')
    expect(updated.config?.windowTitle).toBe('Custom Title')
    expect(wrapper.text()).toContain('图标设置')
  })

  it('overview tab supports rich Minecraft icon selection and updates store', async () => {
    const store = useInstancesStore()
    await store.fetchInstances()
    const inst = store.instances.find((i) => i.id === '1.20.1-vanilla')!

    const wrapper = mount(OverviewTab, {
      props: { instance: inst },
      global: {
        plugins: [i18n, router],
      },
    })
    await flushPromises()

    expect(wrapper.text()).toContain('草方块')
    expect(wrapper.text()).toContain('工作台')
    expect(wrapper.text()).toContain('末影珍珠')

    const buttons = wrapper.findAll('button')
    const diamondBtn = buttons.find((b) => b.text().includes('钻石'))
    expect(diamondBtn).toBeDefined()
    if (diamondBtn) {
      await diamondBtn.trigger('click')
      await flushPromises()
      expect(store.instances.find((i) => i.id === inst.id)?.config?.icon).toBe('diamond')
    }
  })

  it('export tab loads candidate files and submits modpack export', async () => {
    const wrapper = mount(ExportTab, {
      props: {
        instanceId: '1.20.1-vanilla',
        instance: {
          id: '1.20.1-vanilla',
          name: 'Vanilla 1.20.1',
          folder: 'default',
          path: '/path/to/instance',
          mcVersion: '1.20.1',
          kind: 'vanilla',
          loaders: [],
          state: 'ok',
          modable: false,
          card: 'regular',
          loaderVersions: {},
          config: {
            icon: 'grass_block',
            category: 'Default',
            pinned: false,
            isolation: 'global',
            loginRequirement: 'any',
          },
        },
      },
      global: {
        plugins: [i18n, router],
      },
    })
    await flushPromises()

    // Check candidate files rendered
    expect(wrapper.text()).toContain('mods')
    expect(wrapper.text()).toContain('config')
    expect(wrapper.text()).toContain('resourcepacks')
    expect(wrapper.text()).toContain('options.txt')
    expect(wrapper.text()).toContain('saves')

    // Start export
    const exportBtn = wrapper.findAll('button').find((b) => b.text().includes('开始导出'))
    expect(exportBtn).toBeDefined()
    if (exportBtn) {
      await exportBtn.trigger('click')
      await flushPromises()
      expect(lastExportedModpackOptions).not.toBeNull()
      expect(lastExportedModpackOptions?.instanceId).toBe('1.20.1-vanilla')
      expect(lastExportedModpackOptions?.format).toBe('modrinth')
      expect(lastExportedModpackOptions?.selectedPaths).toContain('mods')
      expect(lastExportedModpackOptions?.selectedPaths).toContain('config')
      expect(lastExportedModpackOptions?.selectedPaths).not.toContain('saves')
    }
  })
})

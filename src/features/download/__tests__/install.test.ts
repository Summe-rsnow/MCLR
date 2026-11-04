import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { mount, flushPromises } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import InstallPage from '../InstallPage.vue'
import InstallConfigurator from '../InstallConfigurator.vue'
import { setupMockIpc } from '../../../ipc/mock'
import { resetMockDownload } from '../mock'
import zhCnDownload from '../../../locales/zh-CN/download.json'
import zhCnCommon from '../../../locales/zh-CN/common.json'
import zhCnError from '../../../locales/zh-CN/error.json'
import type { InstanceInfo } from '../../../ipc/client'
import { useInstancesStore } from '../../instances/store'
import { useInstallStore } from '../store'
import { ipcClient } from '../../../ipc/client'

const i18n = createI18n({
  legacy: false,
  locale: 'zh-CN',
  messages: {
    'zh-CN': {
      download: zhCnDownload,
      common: zhCnCommon,
      error: zhCnError,
    },
  },
})

describe('Download / Install feature', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    resetMockDownload()
    setupMockIpc()
  })

  it('renders InstallPage step 1 with version groups', async () => {
    const wrapper = mount(InstallPage, {
      global: {
        plugins: [i18n],
        stubs: {
          RouterLink: true,
          MVirtualList: {
            props: ['items'],
            template: `
              <div class="virtual-list-stub">
                <div v-for="item in items" :key="item.id" class="item" @click="$emit('item-click', item)">
                  <slot name="item" :item="item" />
                </div>
              </div>
            `,
          },
        },
      },
    })

    await flushPromises()
    expect(wrapper.text()).toContain('选择 Minecraft 版本')
    expect(wrapper.text()).toContain('最新版本')
    expect(wrapper.text()).toContain('正式版 (112)')
    expect(wrapper.text()).toContain('预览版 (825)')
    expect(wrapper.text()).toContain('远古版 (165)')
    expect(wrapper.text()).toContain('愚人节版 (12)')
    expect(wrapper.text()).toContain('1.20.4')
    expect(wrapper.text()).toContain('1.20.1')
    expect(wrapper.text()).toContain('最新正式版, 发布于')
    expect(wrapper.text()).toContain('最新预览版, 发布于')
  })
  it('renders InstallConfigurator in create mode with default naming', async () => {
    const wrapper = mount(InstallConfigurator, {
      props: {
        mode: 'create',
        mcVersion: '1.20.1',
      },
      global: {
        plugins: [i18n],
      },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('1.20.1')
    expect(wrapper.text()).toContain('Fabric')
    expect(wrapper.text()).toContain('Forge')
    expect(wrapper.text()).toContain('NeoForge')
    expect(wrapper.text()).toContain('开始安装')
  })

  it('renders InstallConfigurator in modify mode with diff', async () => {
    const dummyInstance: InstanceInfo = {
      id: 'my-inst',
      name: 'my-inst',
      folder: 'C:\\mc',
      path: 'C:\\mc\\versions\\my-inst',
      mcVersion: '1.20.1',
      kind: 'fabric',
      loaders: ['Fabric 0.15.11'],
      state: 'ok',
      config: {
        icon: 'grass_block',
        category: 'Default',
        pinned: false,
        starred: false,
        cardType: null,
        isolation: 'global',
        loginRequirement: 'any',
      },
      modable: true,
      card: 'regular',
      releaseTime: null,
      loaderVersions: {},
    }

    const wrapper = mount(InstallConfigurator, {
      props: {
        mode: 'modify',
        mcVersion: '1.20.1',
        instance: dummyInstance,
      },
      global: {
        plugins: [i18n],
      },
    })

    await flushPromises()

    // Nothing changed yet: no diff affordance at all.
    expect(wrapper.find('[data-testid="config-diff-hint"]').exists()).toBe(false)
    expect(wrapper.text()).toContain('重置实例')

    // Once a different loader is picked the summary becomes a hover diff.
    const forgeBtn = wrapper.findAll('button').find((b) => b.text().includes('NeoForge'))
    await forgeBtn!.trigger('click')
    await flushPromises()
    // The hint icon is the only hover target; the summary text stays plain.
    const hint = wrapper.find('[data-testid="config-diff-hint"]')
    expect(hint.exists()).toBe(true)
    expect(hint.find('span').classes().join(' ')).toContain('i-lucide-circle-alert')
    expect(wrapper.find('[data-testid="config-diff"]').exists()).toBe(false)
    expect(wrapper.text()).not.toContain('变更预览')
  })

  it('renders quilt warning in modify mode when instance has Quilt', async () => {
    const quiltInstance: InstanceInfo = {
      id: 'quilt-inst',
      name: 'quilt-inst',
      folder: 'C:\\mc',
      path: 'C:\\mc\\versions\\quilt-inst',
      mcVersion: '1.20.1',
      kind: 'quilt',
      loaders: ['Quilt 0.20.0'],
      state: 'ok',
      config: {
        icon: 'grass_block',
        category: 'Default',
        pinned: false,
        starred: false,
        cardType: null,
        isolation: 'global',
        loginRequirement: 'any',
      },
      modable: true,
      card: 'regular',
      releaseTime: null,
      loaderVersions: {},
    }

    const wrapper = mount(InstallConfigurator, {
      props: {
        mode: 'modify',
        mcVersion: '1.20.1',
        instance: quiltInstance,
      },
      global: {
        plugins: [i18n],
      },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('Quilt')
    expect(wrapper.text()).toContain('暂不支持通过配置器修改')
  })

  it('fetches NeoForge and LabyMod versions successfully and keeps stable container height', async () => {
    const wrapper = mount(InstallConfigurator, {
      props: {
        mode: 'create',
        mcVersion: '1.20.1',
      },
      global: {
        plugins: [i18n],
      },
    })

    await flushPromises()
    expect(wrapper.classes()).toContain('min-h-[560px]')

    // Select NeoForge
    const neoforgeBtn = wrapper.findAll('button').find((b) => b.text().includes('NeoForge'))
    expect(neoforgeBtn).toBeDefined()
    await neoforgeBtn!.trigger('click')
    await flushPromises()

    expect(wrapper.text()).toContain('20.4.80-beta')

    // Select LabyMod
    const labymodBtn = wrapper.findAll('button').find((b) => b.text().includes('LabyMod'))
    expect(labymodBtn).toBeDefined()
    await labymodBtn!.trigger('click')
    await flushPromises()

    expect(wrapper.text()).toContain('48')
  })

  it('auto-names a LabyMod instance instead of leaving the bare MC version', async () => {
    const wrapper = mount(InstallConfigurator, {
      props: { mode: 'create', mcVersion: '1.20.1' },
      global: { plugins: [i18n] },
    })
    await flushPromises()

    const nameInput = () =>
      wrapper.find('input[data-testid="instance-name"], input').element as HTMLInputElement
    expect(nameInput().value).toBe('1.20.1')

    const labymodBtn = wrapper.findAll('button').find((b) => b.text().includes('LabyMod'))
    await labymodBtn!.trigger('click')
    await flushPromises()

    expect(nameInput().value).toBe('1.20.1-LabyMod_48')
    // Version picker label and the bottom summary must use display names.
    expect(wrapper.text()).toContain('LabyMod 版本选择')
    expect(wrapper.text()).not.toContain('labymod 版本选择')
    expect(wrapper.text()).toContain('LabyMod 48')
  })

  it('drops the loader suffix when switching back to vanilla', async () => {
    const wrapper = mount(InstallConfigurator, {
      props: { mode: 'create', mcVersion: '1.20.1' },
      global: { plugins: [i18n] },
    })
    await flushPromises()

    const nameInput = () => wrapper.find('input').element as HTMLInputElement
    const loaderByText = (needle: string) =>
      wrapper.findAll('button').find((b) => b.text().includes(needle))

    await loaderByText('LabyMod')!.trigger('click')
    await flushPromises()
    expect(nameInput().value).toBe('1.20.1-LabyMod_48')

    // Regression: returning to vanilla kept the stale `-LabyMod_48` suffix.
    await loaderByText('原版纯净')!.trigger('click')
    await flushPromises()
    expect(nameInput().value).toBe('1.20.1')

    // And switching between two loaders replaces the suffix rather than appending.
    await loaderByText('NeoForge')!.trigger('click')
    await flushPromises()
    expect(nameInput().value).toContain('NeoForge')
    expect(nameInput().value).not.toContain('LabyMod')
  })

  it('renders a translated duplicate-name message instead of the raw i18n key', async () => {
    const wrapper = mount(InstallConfigurator, {
      props: { mode: 'create', mcVersion: '1.20.1' },
      global: { plugins: [i18n] },
    })
    await flushPromises()

    const instancesStore = useInstancesStore()
    instancesStore.instances = [
      ...instancesStore.instances,
      { ...instancesStore.instances[0], id: 'dup-1', name: '1.20.1' },
    ]
    await flushPromises()

    const nameInput = wrapper.find('input')
    await nameInput.setValue('1.20.1')
    await flushPromises()

    const error = wrapper.find('[data-testid="name-error"]')
    expect(error.exists()).toBe(true)
    expect(error.text()).toContain('已存在同名实例')
    expect(error.text()).not.toContain('instance.name.duplicate')
  })

  it('keeps the plain auto name and reports the collision instead of renaming it', async () => {
    const wrapper = mount(InstallConfigurator, {
      props: { mode: 'create', mcVersion: '1.20.1' },
      global: { plugins: [i18n] },
    })
    await flushPromises()

    const instancesStore = useInstancesStore()
    instancesStore.instances = [
      ...instancesStore.instances,
      { ...instancesStore.instances[0], id: 'dup-2', name: '1.20.1' },
    ]
    await flushPromises()

    // The name stays exactly what the user expects; the collision is surfaced.
    const nameInput = wrapper.find('input')
    await nameInput.setValue('1.20.1')
    await flushPromises()
    expect((nameInput.element as HTMLInputElement).value).toBe('1.20.1')
    expect(wrapper.find('[data-testid="name-error"]').text()).toContain('已存在同名实例')
  })

  it('hides incompatible extras and keeps loader cards compact', async () => {
    const wrapper = mount(InstallConfigurator, {
      props: { mode: 'create', mcVersion: '1.21.1' },
      global: { plugins: [i18n] },
    })
    await flushPromises()

    const loaderBtn = wrapper.findAll('button').find((b) => b.text().includes('NeoForge'))
    await loaderBtn!.trigger('click')
    await flushPromises()

    // NeoForge + OptiFine is impossible: the card must be gone, not disabled.
    expect(wrapper.text()).not.toContain('OptiFine 搭配增强')
    expect(wrapper.text()).not.toContain('互不兼容')
    // Selected card keeps its original highlight (outer ring + glow), untouched.
    const selected = wrapper
      .findAll('[data-testid="primary-loader-card"]')
      .find((c) => c.classes().includes('border-primary'))
    expect(selected).toBeTruthy()
    const cls = selected!.classes().join(' ')
    expect(cls).toContain('bg-primary/10')
    expect(cls).toContain('ring-primary')
    expect(cls).toContain('shadow-primary/20')
    expect(cls).not.toContain('ring-inset')

    // Cards share the row evenly (no wrap, no left-aligned dead space).
    const grid = wrapper.find('[data-testid="primary-loader-grid"]')
    expect(grid.classes().join(' ')).toContain('grid-cols-[repeat(auto-fit,minmax(150px,1fr))]')
    expect(grid.classes()).not.toContain('flex-wrap')
  })

  it('install flow with two preset ids calls presets_apply once after install task succeeds', async () => {
    const store = useInstallStore()
    const applySpy = vi.spyOn(ipcClient, 'presetsApply').mockResolvedValue('task-apply-1')
    vi.spyOn(ipcClient, 'installVanilla').mockResolvedValue('task-install-1')
    vi.spyOn(ipcClient, 'tasksList').mockResolvedValue([
      {
        id: 'task-install-1',
        parentId: null,
        titleKey: '',
        titleParams: {},
        state: 'success',
        progress: 1,
        speedBps: 0,
      },
    ])
    vi.spyOn(ipcClient, 'presetsApplyReport').mockResolvedValue({
      instanceId: 'created-id-123',
      gates: [],
      rows: [],
    })
    const instancesStore = useInstancesStore()
    vi.spyOn(instancesStore, 'fetchInstances').mockImplementation(async () => {
      instancesStore.instances = [
        {
          id: 'created-id-123',
          name: '1.20.1',
          folder: 'C:\\mc',
          path: 'C:\\mc\\versions\\1.20.1',
          mcVersion: '1.20.1',
          kind: 'vanilla',
          loaders: [],
          state: 'ok',
          config: {
            icon: 'grass_block',
            category: 'Default',
            pinned: false,
            loginRequirement: 'any',
            isolation: 'global',
          },
          modable: false,
          card: 'regular',
          releaseTime: null,
          loaderVersions: {},
        },
      ]
    })

    store.selectedVersion = { id: '1.20.1', versionType: 'release', url: '', releaseTime: '' }
    store.instanceName = '1.20.1'
    store.pendingPresetIds = ['preset-a', 'preset-b']

    const taskId = await store.install()
    expect(taskId).toBe('task-install-1')
    await flushPromises()

    expect(applySpy).toHaveBeenCalledTimes(1)
    expect(applySpy).toHaveBeenCalledWith({
      instanceId: 'created-id-123',
      presetIds: ['preset-a', 'preset-b'],
      datapackWorld: null,
      includeDependencies: true,
      onOtherVersion: 'skip',
    })
  })

  it('install flow with no presets selected never calls presets_apply', async () => {
    const store = useInstallStore()
    const applySpy = vi.spyOn(ipcClient, 'presetsApply').mockResolvedValue('task-apply-1')
    vi.spyOn(ipcClient, 'installVanilla').mockResolvedValue('task-install-1')

    store.selectedVersion = { id: '1.20.1', versionType: 'release', url: '', releaseTime: '' }
    store.instanceName = '1.20.1'
    store.pendingPresetIds = []

    const taskId = await store.install()
    expect(taskId).toBe('task-install-1')
    await flushPromises()

    expect(applySpy).not.toHaveBeenCalled()
  })
})

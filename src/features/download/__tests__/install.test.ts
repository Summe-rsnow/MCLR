import { describe, it, expect, beforeEach } from 'vitest'
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
    expect(wrapper.text()).toContain('1.20.4')
    expect(wrapper.text()).toContain('1.20.1')
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

    expect(wrapper.text()).toContain('变更预览')
    expect(wrapper.text()).toContain('重置实例')
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
})

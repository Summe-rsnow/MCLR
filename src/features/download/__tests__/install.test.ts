import { describe, it, expect, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { mount, flushPromises } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { useInstallStore } from '../store'
import InstallPage from '../InstallPage.vue'
import LoaderPage from '../LoaderPage.vue'
import { setupMockIpc } from '../../../ipc/mock'
import { resetMockDownload } from '../mock'
import zhCnDownload from '../../../locales/zh-CN/download.json'
import zhCnCommon from '../../../locales/zh-CN/common.json'
import zhCnError from '../../../locales/zh-CN/error.json'

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

  it('loads manifest and filters by version type and query', async () => {
    const store = useInstallStore()
    await store.fetchManifest()

    expect(store.manifest.versions.length).toBeGreaterThan(0)
    expect(store.manifest.latestRelease).toBe('1.20.4')

    // Default filterType is 'release'
    expect(store.filteredVersions.every((v) => v.versionType === 'release')).toBe(true)

    // Switch to snapshot
    store.filterType = 'snapshot'
    expect(store.filteredVersions.every((v) => v.versionType === 'snapshot')).toBe(true)

    // Switch to old
    store.filterType = 'old'
    expect(store.filteredVersions.every((v) => v.versionType.startsWith('old_'))).toBe(true)

    // Search query
    store.filterType = 'release'
    store.searchQuery = '1.16'
    expect(store.filteredVersions.length).toBe(1)
    expect(store.filteredVersions[0].id).toBe('1.16.5')
  })

  it('validates instance name correctly', () => {
    const store = useInstallStore()

    store.instanceName = ''
    expect(store.isNameValid).toBe(false)

    store.instanceName = 'MyInstance'
    expect(store.isNameValid).toBe(true)

    store.instanceName = '1.20.1-Fabric'
    expect(store.isNameValid).toBe(true)

    // Illegal chars: \ / : * ? " < > | ! ;
    const invalidChars = ['\\', '/', ':', '*', '?', '"', '<', '>', '|', '!', ';']
    for (const char of invalidChars) {
      store.instanceName = `Name${char}Test`
      expect(store.isNameValid).toBe(false)
    }
  })

  it('triggers install action', async () => {
    const store = useInstallStore()
    await store.fetchManifest()

    const target = store.manifest.versions[0]
    store.openInstallDialog(target)
    expect(store.installDialogOpen).toBe(true)
    expect(store.instanceName).toBe(target.id)

    const taskId = await store.install()
    expect(taskId).toBeTruthy()
    expect(store.installDialogOpen).toBe(false)
  })

  it('renders InstallPage correctly', async () => {
    const wrapper = mount(InstallPage, {
      global: {
        plugins: [i18n],
        stubs: {
          RouterLink: true,
        },
      },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('安装 Minecraft 原版')
    expect(wrapper.text()).toContain('1.20.4')
    expect(wrapper.text()).toContain('1.20.1')
    expect(wrapper.text()).toContain('Fabric')
    expect(wrapper.text()).toContain('NeoForge')
  })

  it('filters compatible loaders based on chosen MC version', async () => {
    const store = useInstallStore()
    await store.fetchManifest()

    // MC 1.20.1
    const mc1201 = store.manifest.versions.find((v) => v.id === '1.20.1')!
    store.openInstallDialog(mc1201)
    expect(store.compatibleLoaders).toContain('fabric')
    expect(store.compatibleLoaders).toContain('forge')
    expect(store.compatibleLoaders).toContain('neoForge')
    // Cleanroom is 1.12.2 only
    expect(store.compatibleLoaders).not.toContain('cleanroom')
    // LegacyFabric is <= 1.13.2
    expect(store.compatibleLoaders).not.toContain('legacyFabric')

    // Old Alpha
    const oldAlpha = store.manifest.versions.find((v) => v.id === 'a1.2.6')!
    store.openInstallDialog(oldAlpha)
    expect(store.compatibleLoaders).toContain('legacyFabric')
    expect(store.compatibleLoaders).not.toContain('fabric')
    expect(store.compatibleLoaders).not.toContain('neoForge')
  })

  it('fetches loader versions and installs Fabric/NeoForge/Forge loader', async () => {
    const store = useInstallStore()
    await store.fetchManifest()

    const mc1201 = store.manifest.versions.find((v) => v.id === '1.20.1')!
    store.openInstallDialog(mc1201)
    store.instanceName = '1.20.1-Fabric-Instance'

    // Select Fabric
    await store.selectLoader('fabric')
    expect(store.loaderVersions.length).toBeGreaterThan(0)
    expect(store.selectedLoaderVersion).toBe('0.15.11')

    // Install Fabric loader
    const fabricTaskId = await store.install()
    expect(fabricTaskId).toContain('task-install-loader')
    expect(fabricTaskId).toContain('1.20.1-Fabric-Instance')
    expect(fabricTaskId).toContain('0.15.11')

    // Select NeoForge
    store.openInstallDialog(mc1201)
    store.instanceName = '1.20.1-NeoForge-Instance'
    await store.selectLoader('neoForge')
    expect(store.loaderVersions.length).toBeGreaterThan(0)
    expect(store.selectedLoaderVersion).toBe('20.4.80-beta')

    const neoforgeTaskId = await store.install()
    expect(neoforgeTaskId).toContain('task-install-loader')
    expect(neoforgeTaskId).toContain('1.20.1-NeoForge-Instance')
    expect(neoforgeTaskId).toContain('20.4.80-beta')
  })

  it('renders LoaderPage for dedicated loader route', async () => {
    const wrapper = mount(LoaderPage, {
      props: {
        loader: 'fabric',
      },
      global: {
        plugins: [i18n],
        stubs: {
          RouterLink: true,
        },
      },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('Fabric')
    expect(wrapper.text()).toContain('0.15.11')
  })
})

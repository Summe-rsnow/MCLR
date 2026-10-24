import { describe, it, expect, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { mount, flushPromises } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { useModsStore } from '../store'
import ModListPanel from '../ModListPanel.vue'
import { setupMockIpc } from '../../../ipc/mock'
import { resetMockMods } from '../mock'
import zhCnMods from '../../../locales/zh-CN/mods.json'
import zhCnCommon from '../../../locales/zh-CN/common.json'
import zhCnError from '../../../locales/zh-CN/error.json'

const i18n = createI18n({
  legacy: false,
  locale: 'zh-CN',
  messages: {
    'zh-CN': {
      mods: zhCnMods,
      common: zhCnCommon,
      error: zhCnError,
    },
  },
})

describe('Local mod management feature (Mods)', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    setupMockIpc()
    resetMockMods()
  })

  it('loads local mods for an instance', async () => {
    const store = useModsStore()
    await store.loadMods('default')

    expect(store.mods.length).toBe(3)
    expect(store.mods[0].name).toBe('Fabric API')
    expect(store.mods[0].enabled).toBe(true)
    expect(store.mods[2].enabled).toBe(false)
  })

  it('filters mods by search query and state', async () => {
    const store = useModsStore()
    await store.loadMods('default')

    store.searchQuery = 'sodium'
    expect(store.filteredMods.length).toBe(1)
    expect(store.filteredMods[0].modId).toBe('sodium')

    store.searchQuery = ''
    store.filterState = 'disabled'
    expect(store.filteredMods.length).toBe(1)
    expect(store.filteredMods[0].modId).toBe('iris')

    store.filterState = 'enabled'
    expect(store.filteredMods.length).toBe(2)
  })

  it('toggles a mod enabled status', async () => {
    const store = useModsStore()
    await store.loadMods('default')

    const sodium = store.mods.find((m) => m.modId === 'sodium')!
    expect(sodium.enabled).toBe(true)

    await store.toggleMod(sodium)
    expect(sodium.enabled).toBe(false)
    expect(sodium.fileName).toContain('.disabled')

    await store.toggleMod(sodium)
    expect(sodium.enabled).toBe(true)
    expect(sodium.fileName).not.toContain('.disabled')
  })

  it('checks updates and applies update', async () => {
    const store = useModsStore()
    await store.loadMods('default')

    await store.checkUpdates('1.20.1', 'fabric')
    const sodium = store.mods.find((m) => m.modId === 'sodium')!
    expect(sodium.updateFile).not.toBeNull()
    expect(sodium.updateFile?.name).toBe('sodium-fabric-0.5.11+mc1.20.1.jar')

    const taskId = await store.updateMod(sodium)
    expect(taskId).toMatch(/^task-mod-update-/)
    expect(sodium.updateFile).toBeNull()
    expect(sodium.version).toBe('0.5.11+mc1.20.1')
  })

  it('deletes a mod from instance', async () => {
    const store = useModsStore()
    await store.loadMods('default')

    const iris = store.mods.find((m) => m.modId === 'iris')!
    await store.deleteMod(iris)
    expect(store.mods.some((m) => m.modId === 'iris')).toBe(false)
  })

  it('opens Jar-in-Jar drawer and retrieves dependency tree', async () => {
    const store = useModsStore()
    await store.loadMods('default')

    const fapi = store.mods.find((m) => m.modId === 'fabric-api')!
    await store.openJarInJar(fapi)

    expect(store.jarInJarTarget).toEqual(fapi)
    expect(store.jarInJarTree.length).toBe(2)
    expect(store.jarInJarTree[0].id).toBe('fabric-api-base')
    expect(store.jarInJarTree[1].children.length).toBe(1)
    expect(store.jarInJarTree[1].children[0].id).toBe('fabric-resource-conditions-api-v1')

    store.closeJarInJar()
    expect(store.jarInJarTarget).toBeNull()
    expect(store.jarInJarTree.length).toBe(0)
  })

  it('mounts ModListPanel and displays mod items', async () => {
    const wrapper = mount(ModListPanel, {
      props: {
        instanceId: 'default',
      },
      global: {
        plugins: [i18n],
      },
    })

    await flushPromises()
    expect(wrapper.text()).toContain('Fabric API')
    expect(wrapper.text()).toContain('Sodium')
    expect(wrapper.text()).toContain('Iris Shaders')
  })
})

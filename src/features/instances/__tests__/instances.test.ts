import { describe, it, expect, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { mount, flushPromises } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { useInstancesStore } from '../store'
import InstancesPage from '../InstancesPage.vue'
import { setupMockIpc } from '../../../ipc/mock'
import { resetMockInstances } from '../mock'
import zhCnInstances from '../../../locales/zh-CN/instances.json'
import zhCnCommon from '../../../locales/zh-CN/common.json'

const i18n = createI18n({
  legacy: false,
  locale: 'zh-CN',
  messages: {
    'zh-CN': {
      instances: zhCnInstances,
      common: zhCnCommon,
    },
  },
})

describe('Instances feature', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    resetMockInstances()
    setupMockIpc()
  })

  it('loads instances and handles search filtering', async () => {
    const store = useInstancesStore()
    await store.fetchInstances()

    expect(store.instances.length).toBeGreaterThan(0)
    expect(store.filteredInstances.length).toBe(store.instances.length)

    store.searchQuery = 'Fabric'
    expect(store.filteredInstances.length).toBe(1)
    expect(store.filteredInstances[0].name).toBe('Fabric 1.20.4')

    store.searchQuery = ''
    store.filterType = 'pinned'
    expect(store.filteredInstances.every((i) => i.config?.pinned)).toBe(true)
  })

  it('handles pin toggle, rename, duplicate, and remove', async () => {
    const store = useInstancesStore()
    await store.fetchInstances()

    const initialCount = store.instances.length
    const target = store.instances[0]
    const initialPinned = !!target.config?.pinned

    // Toggle pin
    await store.togglePin(target.id)
    expect(store.instances.find((i) => i.id === target.id)?.config?.pinned).toBe(!initialPinned)

    // Rename
    await store.rename(target.id, 'Renamed Instance')
    expect(store.instances.find((i) => i.id === target.id)?.name).toBe('Renamed Instance')

    // Duplicate
    await store.duplicate(target.id, 'Duplicated Instance')
    expect(store.instances.length).toBe(initialCount + 1)
    expect(store.instances.some((i) => i.name === 'Duplicated Instance')).toBe(true)

    // Remove
    await store.remove(target.id)
    expect(store.instances.some((i) => i.id === target.id)).toBe(false)
  })

  it('renders InstancesPage correctly', async () => {
    const wrapper = mount(InstancesPage, {
      global: {
        plugins: [i18n],
      },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('游戏目录')
    expect(wrapper.text()).toContain('Vanilla 1.20.1')
    expect(wrapper.text()).toContain('Fabric 1.20.4')
  })
})

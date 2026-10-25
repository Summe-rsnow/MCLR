import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { mount, flushPromises } from '@vue/test-utils'
import { createRouter, createWebHistory } from 'vue-router'
import { createI18n } from 'vue-i18n'
import { useInstancesStore } from '../store'
import InstancesPage from '../InstancesPage.vue'
import { ipcClient } from '../../../ipc/client'
import GameFoldersDialog from '../GameFoldersDialog.vue'
import { setupMockIpc } from '../../../ipc/mock'
import { resetMockInstances } from '../mock'
import zhCnInstances from '../../../locales/zh-CN/instances.json'
import zhCnCommon from '../../../locales/zh-CN/common.json'
import zhCnComp from '../../../locales/zh-CN/comp.json'

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
}))

import { open as openFileDialog } from '@tauri-apps/plugin-dialog'

const i18n = createI18n({
  legacy: false,
  locale: 'zh-CN',
  messages: {
    'zh-CN': {
      instances: zhCnInstances,
      common: zhCnCommon,
      comp: zhCnComp,
    },
  },
})

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', component: { template: '<div>home</div>' } },
    { path: '/download', component: { template: '<div>download</div>' } },
    { path: '/instances/:id', component: { template: '<div>detail</div>' } },
  ],
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

  it('renders guide card when no game folder is set, and shows instances once folder is configured', async () => {
    const wrapper = mount(InstancesPage, {
      global: {
        plugins: [i18n, router],
      },
    })

    await flushPromises()

    expect(wrapper.find('[data-testid="no-folder-guide"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('尚未设置 Minecraft 游戏主目录')
    expect(wrapper.text()).toContain('选择游戏目录')

    const store = useInstancesStore()
    await store.addFolder('Default', 'C:\\mock\\.minecraft')
    await flushPromises()

    expect(wrapper.find('[data-testid="no-folder-guide"]').exists()).toBe(false)
    expect(wrapper.text()).toContain('Vanilla 1.20.1')
    expect(wrapper.text()).toContain('Fabric 1.20.4')
  })

  it('updates instance config via updateConfig action', async () => {
    const store = useInstancesStore()
    await store.fetchInstances()
    const target = store.instances[0]
    await store.updateConfig(target.id, { icon: 'furnace' })
    expect(store.instances.find((i) => i.id === target.id)?.config?.icon).toBe('furnace')
  })

  it('manages game folders in GameFoldersDialog: list, add, rename, switch, openPath, delete', async () => {
    const store = useInstancesStore()
    store.folders = [
      { name: 'Primary MC', path: 'C:\\Games\\.minecraft' },
      { name: 'Secondary MC', path: 'D:\\Games\\.minecraft' },
    ]
    store.currentFolder = 'C:\\Games\\.minecraft'

    const pathOpenSpy = vi.spyOn(ipcClient, 'pathOpen').mockResolvedValue(undefined)

    mount(GameFoldersDialog, {
      props: { open: true },
      global: {
        plugins: [i18n],
      },
    })
    await flushPromises()

    expect(document.body.innerHTML).toContain('Primary MC')
    expect(document.body.innerHTML).toContain('Secondary MC')

    // Open path
    const openBtns = document.body.querySelectorAll<HTMLButtonElement>(
      '[data-testid="open-folder-btn"]',
    )
    expect(openBtns.length).toBe(2)
    openBtns[0].click()
    await flushPromises()
    expect(pathOpenSpy).toHaveBeenCalledWith('C:\\Games\\.minecraft')

    // Edit remark
    const editBtns = document.body.querySelectorAll<HTMLButtonElement>(
      '[data-testid="edit-remark-btn"]',
    )
    editBtns[0].click()
    await flushPromises()

    const input = document.body.querySelector<HTMLInputElement>(
      '[data-testid="edit-remark-input"] input',
    )
    expect(input).not.toBeNull()
    if (input) {
      input.value = 'Renamed Primary'
      input.dispatchEvent(new Event('input'))
    }
    const saveBtn = document.body.querySelector<HTMLButtonElement>(
      '[data-testid="save-remark-btn"]',
    )
    saveBtn?.click()
    await flushPromises()
    expect(store.folders[0].name).toBe('Renamed Primary')

    // Add folder flow
    vi.mocked(openFileDialog).mockResolvedValue('E:\\Games\\.minecraft')
    const addBtn = document.body.querySelector<HTMLButtonElement>('[data-testid="add-folder-btn"]')
    addBtn?.click()
    await flushPromises()
    expect(store.folders.some((f) => f.path === 'E:\\Games\\.minecraft')).toBe(true)

    // Remove folder
    const removeFolderSpy = vi.spyOn(store, 'removeFolder')
    await store.removeFolder('D:\\Games\\.minecraft')
    expect(removeFolderSpy).toHaveBeenCalledWith('D:\\Games\\.minecraft')
    expect(store.folders.some((f) => f.path === 'D:\\Games\\.minecraft')).toBe(false)
  })

  it('opens GameFoldersDialog when folder-cog button in InstancesPage toolbar is clicked', async () => {
    const wrapper = mount(InstancesPage, {
      global: {
        plugins: [i18n, router],
      },
    })
    await flushPromises()

    const cogBtn = wrapper.find('button[title="管理游戏目录"]')
    expect(cogBtn.exists()).toBe(true)
    await cogBtn.trigger('click')
    await flushPromises()

    const dialog = wrapper.findComponent(GameFoldersDialog)
    expect(dialog.exists()).toBe(true)
    expect(dialog.props('open')).toBe(true)
  })

  it('renders matching grid container classes for instances skeleton when uninitialized', async () => {
    const store = useInstancesStore()
    store.initialized = false
    store.currentFolder = 'D:\\Games\\.minecraft'
    const wrapper = mount(InstancesPage, {
      global: {
        plugins: [i18n, router],
      },
    })
    const skeleton = wrapper.find('[data-testid="instances-skeleton"]')
    expect(skeleton.exists()).toBe(true)
    expect(skeleton.classes()).toContain('grid')
    expect(skeleton.classes()).toContain('grid-cols-1')
    expect(skeleton.classes()).toContain('sm:grid-cols-2')
    expect(skeleton.classes()).toContain('md:grid-cols-3')
    expect(skeleton.classes()).toContain('lg:grid-cols-4')
    expect(skeleton.classes()).toContain('gap-4')
    const cards = skeleton.findAll('.m-card')
    expect(cards.length).toBe(8)
  })
})

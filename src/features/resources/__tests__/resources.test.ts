import { describe, it, expect, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createPinia, setActivePinia } from 'pinia'
import ResourceToolbar from '../components/ResourceToolbar.vue'
import ResourceItemRow from '../components/ResourceItemRow.vue'
import ResourceModals from '../components/ResourceModals.vue'
import ResourceList from '../ResourceList.vue'
import { ipcClient } from '../../../ipc/client'
import { awaitTask } from '../../tasks/awaitTask'
import { useToast } from '../../../ui'
import zhCnResources from '../../../locales/zh-CN/resources.json'
import type { ResourceEntry, ResourcesPatchEvent } from '../../../ipc/client'

// Local artwork is served through the asset protocol in the packaged app.
vi.mock('@tauri-apps/api/core', () => ({
  convertFileSrc: (path: string) => `asset://localhost/${encodeURIComponent(path)}`,
}))

// ResourceList subscribes to `resources://patch` on mount. Keeping the handlers lets a test push a
// batch exactly like the backend does, instead of reaching into component internals.
const patchBus = vi.hoisted(() => ({
  handlers: [] as ((event: { payload: ResourcesPatchEvent }) => void)[],
}))

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn((name: string, handler: (event: { payload: ResourcesPatchEvent }) => void) => {
    if (name === 'resources://patch') patchBus.handlers.push(handler)
    return Promise.resolve(() => {})
  }),
}))

// Mod updates are queued as background tasks, so the outcome is driven per test.
vi.mock('../../tasks/awaitTask', () => ({ awaitTask: vi.fn() }))

describe('Resources feature unit tests', () => {
  const dummyEntries: ResourceEntry[] = [
    {
      path: '/path/to/mods/fabric-api.jar',
      relPath: 'fabric-api.jar',
      fileName: 'fabric-api.jar',
      isDir: false,
      size: 2048000,
      createdMs: 1000,
      modifiedMs: 2000,
      enabled: true,
      state: 'ok',
      title: 'Fabric API',
      description: 'Core API for Fabric',
      version: '0.92.0',
      icon: null,
      tags: ['Fabric'],
      modId: 'fabric-api',
      projectId: 'fabric-api',
      latestFile: null,
      updateAvailable: false,
      dependents: [],
    },
    {
      path: '/path/to/mods/sodium.jar',
      relPath: 'sodium.jar',
      fileName: 'sodium.jar',
      isDir: false,
      size: 1024000,
      createdMs: 2000,
      modifiedMs: 3000,
      enabled: true,
      state: 'ok',
      title: 'Sodium',
      description: 'Rendering engine',
      version: '0.5.8',
      icon: null,
      tags: ['Fabric'],
      modId: 'sodium',
      projectId: 'sodium',
      latestFile: {
        id: 'file-2',
        platform: 'modrinth',
        projectId: 'sodium',
        name: 'sodium-0.5.11.jar',
        fileName: 'sodium-0.5.11.jar',
        channel: 'release',
        releaseDate: '2024-05-01',
        size: 1050000,
        mcVersions: ['1.20.1'],
        loaders: ['fabric'],
        downloadUrls: ['https://example.com/sodium.jar'],
        hashes: { sha1: '123', sha512: null, murmur2: null },
        dependencies: [],
        distributionDisabled: false,
      },
      updateAvailable: true,
      dependents: [],
    },
    {
      path: '/path/to/mods/iris.jar.disabled',
      relPath: 'iris.jar.disabled',
      fileName: 'iris.jar.disabled',
      isDir: false,
      size: 3000000,
      createdMs: 500,
      modifiedMs: 1500,
      enabled: false,
      state: 'ok',
      title: 'Iris Shaders',
      description: 'Shaderpack support',
      version: '1.6.17',
      icon: null,
      tags: ['Fabric'],
      modId: 'iris',
      projectId: 'iris',
      latestFile: null,
      updateAvailable: false,
      dependents: [],
    },
  ]

  function makeEntry(overrides: Partial<ResourceEntry> = {}): ResourceEntry {
    return {
      path: '/mods/x.jar',
      relPath: 'x.jar',
      fileName: 'x.jar',
      isDir: false,
      size: 1024,
      createdMs: null,
      modifiedMs: null,
      enabled: true,
      state: 'ok',
      title: null,
      description: null,
      version: null,
      icon: null,
      tags: [],
      modId: null,
      projectId: null,
      latestFile: null,
      updateAvailable: false,
      dependents: [],
      ...overrides,
    }
  }

  async function mountList(entries: ResourceEntry[]) {
    const listSpy = vi.spyOn(ipcClient, 'resourcesList').mockResolvedValue(entries)
    patchBus.handlers.length = 0
    const i18n = createI18n({
      legacy: false,
      locale: 'zh-CN',
      messages: {
        'zh-CN': {
          resources: zhCnResources,
          error: { unknown: 'Unknown error' },
        },
      },
    })
    const pinia = createPinia()
    setActivePinia(pinia)
    const wrapper = mount(ResourceList, {
      props: { instanceId: 'inst-1', kind: 'mod' },
      global: { plugins: [i18n, pinia] },
    })
    await flushPromises()
    return { wrapper, listSpy }
  }

  it('correctly filters enabled and disabled resources', () => {
    const enabled = dummyEntries.filter((e) => e.enabled)
    const disabled = dummyEntries.filter((e) => !e.enabled)

    expect(enabled.length).toBe(2)
    expect(disabled.length).toBe(1)
    expect(disabled[0].title).toBe('Iris Shaders')
  })

  it('correctly identifies updatable resources', () => {
    const updatable = dummyEntries.filter((e) => e.updateAvailable)
    expect(updatable.length).toBe(1)
    expect(updatable[0].title).toBe('Sodium')
    expect(updatable[0].latestFile?.name).toBe('sodium-0.5.11.jar')
  })

  it('computes weighted search score accurately', () => {
    function computeScore(entry: ResourceEntry, query: string): number {
      if (!query) return 1
      const q = query.toLowerCase()
      let score = 0
      if (entry.title?.toLowerCase().includes(q)) score += 1.0
      if (entry.fileName.toLowerCase().includes(q)) score += 1.0
      if (entry.description?.toLowerCase().includes(q)) score += 0.4
      if (entry.version?.toLowerCase().includes(q)) score += 0.2
      for (const tag of entry.tags) {
        if (tag.toLowerCase().includes(q)) {
          score += 0.2
          break
        }
      }
      return score
    }

    const sodium = dummyEntries[1]
    expect(computeScore(sodium, 'sodium')).toBeGreaterThanOrEqual(1.0)
    expect(computeScore(sodium, 'rendering')).toBeCloseTo(0.4)
    expect(computeScore(sodium, 'fabric')).toBeCloseTo(0.2)
    expect(computeScore(sodium, 'nonexistent')).toBe(0)
  })

  it('sorts entries by size, creation time, and name', () => {
    const sortedBySize = [...dummyEntries].sort((a, b) => b.size - a.size)
    expect(sortedBySize[0].title).toBe('Iris Shaders')
    expect(sortedBySize[2].title).toBe('Sodium')

    const sortedByName = [...dummyEntries].sort((a, b) =>
      (a.title || a.fileName).localeCompare(b.title || b.fileName),
    )
    expect(sortedByName[0].title).toBe('Fabric API')
    expect(sortedByName[1].title).toBe('Iris Shaders')
    expect(sortedByName[2].title).toBe('Sodium')
  })

  it('supports batch toggle logic for all entries', () => {
    // Simulate enable all
    const allEnabled = dummyEntries.map((e) => ({ ...e, enabled: true }))
    expect(allEnabled.every((e) => e.enabled)).toBe(true)

    // Simulate disable all
    const allDisabled = dummyEntries.map((e) => ({ ...e, enabled: false }))
    expect(allDisabled.every((e) => !e.enabled)).toBe(true)

    // Simulate batch toggle on selection
    const selected = new Set([dummyEntries[0].path, dummyEntries[2].path])
    const batchDisabled = dummyEntries.map((e) =>
      selected.has(e.path) ? { ...e, enabled: false } : e,
    )
    expect(batchDisabled.find((e) => e.path === dummyEntries[0].path)?.enabled).toBe(false)
    expect(batchDisabled.find((e) => e.path === dummyEntries[1].path)?.enabled).toBe(true)
  })

  it('asks for the cached verdict on entry but forces a real lookup on the button', async () => {
    const entries = dummyEntries
    const listSpy = vi.spyOn(ipcClient, 'resourcesList').mockResolvedValue(entries)
    // Installed before mount, so the automatic pass on entry is observed too.
    const checkSpy = vi.spyOn(ipcClient, 'localModsCheckUpdates').mockResolvedValue([])

    const i18n = createI18n({
      legacy: false,
      locale: 'zh-CN',
      messages: { 'zh-CN': { resources: zhCnResources, error: { unknown: 'Unknown error' } } },
    })
    const pinia = createPinia()
    setActivePinia(pinia)
    const wrapper = mount(ResourceList, {
      props: { instanceId: 'inst-1', kind: 'mod' },
      global: { plugins: [i18n, pinia] },
    })
    await flushPromises()

    // Entering the list asks once, without bypassing the backend's 6-hour cache.
    expect(checkSpy).toHaveBeenCalledTimes(1)
    expect(checkSpy.mock.calls[0][3]).toBe(false)

    wrapper.findComponent({ name: 'ResourceToolbar' }).vm.$emit('checkUpdates')
    await flushPromises()

    expect(checkSpy).toHaveBeenCalledTimes(2)
    expect(checkSpy.mock.calls[1][3]).toBe(true)

    checkSpy.mockRestore()
    listSpy.mockRestore()
  })

  it('renders "检查更新" button in ResourceToolbar for mod kind and emits checkUpdates', async () => {
    const i18n = createI18n({
      legacy: false,
      locale: 'zh-CN',
      messages: {
        'zh-CN': {
          resources: zhCnResources,
        },
      },
    })

    const wrapper = mount(ResourceToolbar, {
      props: {
        kind: 'mod',
        searchQuery: '',
        activeSort: 'name',
        sortOptions: [{ label: '名称', value: 'name' }],
        activeFilter: 'all',
        filterCounts: {
          all: 3,
          enabled: 2,
          disabled: 1,
          updatable: 1,
          error: 0,
          duplicate: 0,
        },
        checkingUpdates: false,
      },
      global: {
        plugins: [i18n],
      },
    })

    const checkBtn = wrapper.findAll('button').find((b) => b.text().includes('检查更新'))
    expect(checkBtn).toBeDefined()
    await checkBtn!.trigger('click')
    expect(wrapper.emitted('checkUpdates')).toBeTruthy()
  })

  it('correctly maps updated mods to entries with updateAvailable and latestFile', () => {
    const entries = JSON.parse(JSON.stringify(dummyEntries)) as ResourceEntry[]
    const updatedMods = [
      {
        fileName: 'fabric-api.jar',
        path: '/path/to/mods/fabric-api.jar',
        enabled: true,
        modId: 'fabric-api',
        name: 'Fabric API',
        version: '0.92.0',
        description: 'Core API for Fabric',
        authors: [],
        loader: 'fabric',
        sha1: 'abc',
        murmur2: 123,
        size: 2048000,
        projectId: 'fabric-api',
        updateFile: {
          id: 'file-fabric-update',
          platform: 'modrinth' as const,
          projectId: 'fabric-api',
          name: 'fabric-api-0.93.0.jar',
          fileName: 'fabric-api-0.93.0.jar',
          channel: 'release' as const,
          releaseDate: '2024-06-01',
          size: 2100000,
          mcVersions: ['1.20.1'],
          loaders: ['fabric'],
          downloadUrls: ['https://example.com/fabric-api-0.93.0.jar'],
          hashes: { sha1: 'abc2', sha512: null, murmur2: null },
          dependencies: [],
          distributionDisabled: false,
        },
      },
    ]

    let updateCount = 0
    for (const m of updatedMods) {
      const entry = entries.find((e) => e.fileName === m.fileName || e.path === m.path)
      if (entry && m.updateFile) {
        entry.updateAvailable = true
        entry.latestFile = m.updateFile
        updateCount++
      }
    }

    expect(updateCount).toBe(1)
    const updated = entries.find((e) => e.fileName === 'fabric-api.jar')
    expect(updated?.updateAvailable).toBe(true)
    expect(updated?.latestFile?.name).toBe('fabric-api-0.93.0.jar')
  })

  it('emits toggleSelectAll from the toolbar and swaps label with selection state', async () => {
    const i18n = createI18n({
      legacy: false,
      locale: 'zh-CN',
      messages: { 'zh-CN': { resources: zhCnResources } },
    })
    const baseProps = {
      kind: 'mod' as const,
      searchQuery: '',
      activeSort: 'name' as const,
      sortOptions: [{ label: '名称', value: 'name' }],
      activeFilter: 'all' as const,
      filterCounts: { all: 3, enabled: 3, disabled: 0, updatable: 0, error: 0, duplicate: 0 },
      checkingUpdates: false,
      totalCount: 3,
    }

    const partial = mount(ResourceToolbar, {
      props: { ...baseProps, selectedCount: 0 },
      global: { plugins: [i18n] },
    })
    const selectAllBtn = partial.findAll('button').find((b) => b.text().includes('全选'))
    expect(selectAllBtn).toBeDefined()
    await selectAllBtn!.trigger('click')
    expect(partial.emitted('toggleSelectAll')).toHaveLength(1)

    const all = mount(ResourceToolbar, {
      props: { ...baseProps, selectedCount: 3 },
      global: { plugins: [i18n] },
    })
    const clearBtn = all.findAll('button').find((b) => b.text().includes('取消选择'))
    expect(clearBtn).toBeDefined()
    await clearBtn!.trigger('click')
    expect(all.emitted('toggleSelectAll')).toHaveLength(1)
  })

  it('toggles selection by clicking anywhere on the row (no checkbox)', async () => {
    const i18n = createI18n({
      legacy: false,
      locale: 'zh-CN',
      messages: { 'zh-CN': { resources: zhCnResources } },
    })
    const entry = { ...dummyEntries[1]!, icon: null }

    const row = mount(ResourceItemRow, {
      props: { entry, kind: 'mod', selected: false },
      global: { plugins: [i18n] },
    })

    expect(row.find('input[type="checkbox"]').exists()).toBe(false)
    expect(row.classes().join(' ')).toContain('cursor-pointer')

    await row.trigger('click')
    expect(row.emitted('select')?.[0]).toEqual([entry.path])
  })

  it('renders the mod artwork as the row cover', () => {
    const i18n = createI18n({
      legacy: false,
      locale: 'zh-CN',
      messages: { 'zh-CN': { resources: zhCnResources } },
    })
    const entry = { ...dummyEntries[0]!, icon: 'C:/mclr/cache/icons/abc123.png' }

    const row = mount(ResourceItemRow, {
      props: { entry, kind: 'mod', selected: false },
      global: { plugins: [i18n] },
    })

    const img = row.find('img')
    expect(img.exists()).toBe(true)
    expect(img.attributes('src')).toBe(
      `asset://localhost/${encodeURIComponent('C:/mclr/cache/icons/abc123.png')}`,
    )

    // Mods whose jar ships no artwork fall back to their Modrinth project icon, which is a
    // remote URL and must not be run through the asset protocol.
    const remote = mount(ResourceItemRow, {
      props: {
        entry: { ...dummyEntries[0]!, icon: 'https://cdn.modrinth.com/data/AANobbMI/icon.png' },
        kind: 'mod',
        selected: false,
      },
      global: { plugins: [i18n] },
    })
    expect(remote.find('img').attributes('src')).toBe(
      'https://cdn.modrinth.com/data/AANobbMI/icon.png',
    )
  })

  it('reveals batch operations after select-all in ResourceList', async () => {
    const i18n = createI18n({
      legacy: false,
      locale: 'zh-CN',
      messages: { 'zh-CN': { resources: zhCnResources } },
    })
    const mods = dummyEntries.filter((e) => !e.isDir)
    const listSpy = vi.spyOn(ipcClient, 'resourcesList').mockResolvedValue(mods)
    const pinia = createPinia()
    setActivePinia(pinia)

    const wrapper = mount(ResourceList, {
      props: { instanceId: 'inst-1', kind: 'mod' },
      global: { plugins: [i18n, pinia] },
    })
    await flushPromises()
    expect(listSpy).toHaveBeenCalled()
    // Hidden while nothing is selected.
    expect(wrapper.text()).not.toContain('已选择')

    const selectAllBtn = wrapper.findAll('button').find((b) => b.text().includes('全选'))
    expect(selectAllBtn).toBeDefined()
    await selectAllBtn!.trigger('click')
    await flushPromises()

    expect(wrapper.text()).toContain(`已选择 ${mods.length} 项`)
    const actions = wrapper.findAll('button').map((b) => b.text())
    expect(actions.some((t) => t.includes('启用'))).toBe(true)
    expect(actions.some((t) => t.includes('禁用'))).toBe(true)
    expect(actions.some((t) => t.includes('删除'))).toBe(true)
  })

  it('flags duplicates only for entries sharing a modId or projectId', async () => {
    const entries = [
      makeEntry({ path: '/mods/a.jar', fileName: 'a.jar', title: 'Alpha', modId: 'shared-mod' }),
      makeEntry({ path: '/mods/b.jar', fileName: 'b.jar', title: 'Beta', modId: 'shared-mod' }),
      makeEntry({ path: '/mods/e.jar', fileName: 'e.jar', title: 'Epsilon', projectId: 'shared' }),
      makeEntry({ path: '/mods/f.jar', fileName: 'f.jar', title: 'Foxtrot', projectId: 'shared' }),
      // Same display name but no identity: unrelated files, never duplicates.
      makeEntry({ path: '/mods/c.jar', fileName: 'c.jar', title: 'Mystery' }),
      makeEntry({ path: '/mods/d.jar', fileName: 'd.jar', title: 'Mystery' }),
    ]
    const { wrapper } = await mountList(entries)

    const chip = wrapper.findAll('button').find((b) => b.text().includes('重复'))
    expect(chip).toBeDefined()
    expect(chip!.text().replace(/\s+/g, ' ').trim()).toBe('重复 (4)')

    await chip!.trigger('click')
    await flushPromises()

    const rows = wrapper.findAllComponents(ResourceItemRow)
    expect(rows.map((row) => row.props('entry').fileName)).toEqual([
      'a.jar',
      'b.jar',
      'e.jar',
      'f.jar',
    ])
  })

  it('applies a resources://patch batch to already loaded entries', async () => {
    const { wrapper } = await mountList([makeEntry({ title: 'Fabric API' })])
    expect(patchBus.handlers).toHaveLength(1)

    patchBus.handlers[0]!({
      payload: {
        requestId: 'req-1',
        instanceId: 'inst-1',
        patches: [
          { fileName: 'x.jar', title: 'Patched API', tags: ['Fabric'], modId: 'fabric-api' },
          { fileName: 'missing.jar', title: 'Ignored' },
        ],
      },
    })
    await flushPromises()

    expect(wrapper.text()).toContain('Patched API')
    expect(wrapper.text()).not.toContain('Ignored')
  })
  it('does not wipe discovered updateAvailable or latestFile when resources://patch arrives', async () => {
    const latest = dummyEntries[1]!.latestFile!
    const entry = makeEntry({
      fileName: 'sodium.jar',
      title: 'Sodium',
      updateAvailable: true,
      latestFile: latest,
    })
    const { wrapper } = await mountList([entry])

    patchBus.handlers[0]!({
      payload: {
        requestId: 'req-1',
        instanceId: 'inst-1',
        patches: [
          {
            fileName: 'sodium.jar',
            icon: 'https://example.com/icon.webp',
            updateAvailable: false,
            latestFile: null,
          },
        ],
      },
    })
    await flushPromises()

    const item = wrapper.findComponent({ name: 'ResourceItemRow' })
    expect(item.props('entry').updateAvailable).toBe(true)
    expect(item.props('entry').latestFile).toEqual(latest)
  })

  it('waits for the update task before reporting a single update as successful', async () => {
    const latest = dummyEntries[1]!.latestFile!
    const entry = makeEntry({
      path: '/path/to/mods/sodium.jar',
      fileName: 'sodium.jar',
      title: 'Sodium',
      modId: 'sodium',
      updateAvailable: true,
      latestFile: latest,
    })
    const { wrapper, listSpy } = await mountList([entry])

    const updateSpy = vi.spyOn(ipcClient, 'localModUpdate').mockResolvedValue('task-7')
    const taskMock = vi.mocked(awaitTask)
    taskMock.mockReset()
    taskMock.mockResolvedValue({ state: 'success', error: null })
    const { toasts } = useToast()
    toasts.value.splice(0)

    wrapper.findComponent(ResourceModals).vm.$emit('single-update', entry)
    await flushPromises()

    expect(updateSpy).toHaveBeenCalledWith('inst-1', 'sodium.jar', latest)
    expect(taskMock).toHaveBeenCalledWith('task-7')
    // Reloaded once more only after the queued download finished.
    expect(listSpy).toHaveBeenCalledTimes(2)
    expect(toasts.value.some((toast) => toast.type === 'success')).toBe(true)
  })

  it('reports a failed update task instead of a success toast', async () => {
    const entry = makeEntry({
      path: '/path/to/mods/sodium.jar',
      fileName: 'sodium.jar',
      title: 'Sodium',
      updateAvailable: true,
      latestFile: dummyEntries[1]!.latestFile,
    })
    const { wrapper, listSpy } = await mountList([entry])

    vi.spyOn(ipcClient, 'localModUpdate').mockResolvedValue('task-8')
    const taskMock = vi.mocked(awaitTask)
    taskMock.mockReset()
    taskMock.mockResolvedValue({ state: 'failed', error: null })
    const { toasts } = useToast()
    toasts.value.splice(0)

    wrapper.findComponent(ResourceModals).vm.$emit('single-update', entry)
    await flushPromises()

    expect(taskMock).toHaveBeenCalledWith('task-8')
    expect(listSpy).toHaveBeenCalledTimes(1)
    expect(
      toasts.value.some((toast) => toast.type === 'danger' && toast.message.includes('更新失败')),
    ).toBe(true)
  })

  it('waits for the batch update task before reporting success', async () => {
    const latest = dummyEntries[1]!.latestFile!
    const entry = makeEntry({
      path: '/path/to/mods/sodium.jar',
      fileName: 'sodium.jar',
      title: 'Sodium',
      updateAvailable: true,
      latestFile: latest,
    })
    const { wrapper, listSpy } = await mountList([entry])

    const batchSpy = vi.spyOn(ipcClient, 'localModsUpdateMany').mockResolvedValue('task-9')
    const taskMock = vi.mocked(awaitTask)
    taskMock.mockReset()
    taskMock.mockResolvedValue({ state: 'success', error: null })

    wrapper.findComponent(ResourceModals).vm.$emit('batch-update', [entry])
    await flushPromises()

    expect(batchSpy).toHaveBeenCalledWith('inst-1', [
      { currentFileName: 'sodium.jar', newFile: latest },
    ])
    expect(taskMock).toHaveBeenCalledWith('task-9')
    expect(listSpy).toHaveBeenCalledTimes(2)
  })

  it('toggles a selected batch eight files at a time', async () => {
    const entries: ResourceEntry[] = []
    for (let index = 0; index < 20; index++) {
      entries.push(makeEntry({ path: `/mods/m${index}.jar`, title: `Mod ${index}` }))
    }
    const { wrapper } = await mountList(entries)

    let inFlight = 0
    let maxInFlight = 0
    const toggleSpy = vi.spyOn(ipcClient, 'resourceToggle')
    toggleSpy.mockImplementation(async (_id, path) => {
      inFlight++
      maxInFlight = Math.max(maxInFlight, inFlight)
      await Promise.resolve()
      inFlight--
      return `${path}.disabled`
    })

    const selectAllBtn = wrapper.findAll('button').find((b) => b.text().includes('全选'))
    expect(selectAllBtn).toBeDefined()
    await selectAllBtn!.trigger('click')

    const disableBtn = wrapper.findAll('button').find((b) => b.text().trim() === '禁用')
    expect(disableBtn).toBeDefined()
    await disableBtn!.trigger('click')
    await flushPromises()

    expect(toggleSpy).toHaveBeenCalledTimes(20)
    expect(maxInFlight).toBe(8)
  })
})

import { describe, it, expect, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createPinia, setActivePinia } from 'pinia'
import ResourceToolbar from '../components/ResourceToolbar.vue'
import ResourceItemRow from '../components/ResourceItemRow.vue'
import ResourceList from '../ResourceList.vue'
import { ipcClient } from '../../../ipc/client'
import zhCnResources from '../../../locales/zh-CN/resources.json'
import type { ResourceEntry } from '../../../ipc/client'

// Local artwork is served through the asset protocol in the packaged app.
vi.mock('@tauri-apps/api/core', () => ({
  convertFileSrc: (path: string) => `asset://localhost/${encodeURIComponent(path)}`,
}))

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
})

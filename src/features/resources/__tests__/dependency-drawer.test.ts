import { describe, it, expect, vi, afterEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createRouter, createMemoryHistory } from 'vue-router'
import { defineComponent, h } from 'vue'
import ModDependencyDrawer from '../components/ModDependencyDrawer.vue'
import ResourceItemRow from '../components/ResourceItemRow.vue'
import {
  ipcClient,
  type CompProject,
  type ModDependencyReport,
  type ResourceEntry,
} from '../../../ipc/client'
import zhCnResources from '../../../locales/zh-CN/resources.json'
import zhCnCommon from '../../../locales/zh-CN/common.json'

vi.mock('@tauri-apps/api/core', () => ({
  convertFileSrc: (path: string) => `asset://localhost/${encodeURIComponent(path)}`,
}))

const i18n = createI18n({
  legacy: false,
  locale: 'zh-CN',
  messages: { 'zh-CN': { resources: zhCnResources, common: zhCnCommon } },
})

function makeEntry(overrides: Partial<ResourceEntry> = {}): ResourceEntry {
  return {
    path: '/mods/sodium.jar',
    relPath: 'sodium.jar',
    fileName: 'sodium.jar',
    isDir: false,
    size: 1024,
    createdMs: null,
    modifiedMs: null,
    enabled: true,
    state: 'ok',
    title: 'Sodium',
    description: null,
    version: '0.5.8',
    icon: null,
    tags: [],
    modId: 'sodium',
    projectId: 'AANobbMI',
    latestFile: null,
    updateAvailable: false,
    dependents: [],
    ...overrides,
  }
}

function makeProject(overrides: Partial<CompProject> = {}): CompProject {
  return {
    platform: 'modrinth',
    id: 'AANobbMI',
    slug: 'sodium',
    title: 'Sodium',
    summary: '',
    author: 'jellysquid',
    downloads: 0,
    follows: 0,
    categories: [],
    mcVersions: ['1.20.1'],
    loaders: ['fabric'],
    compType: 'mod',
    updatedAt: '2024-01-01',
    ...overrides,
  }
}

function missingReport(): ModDependencyReport {
  return {
    fileName: 'sodium.jar',
    modId: 'sodium',
    deps: [
      {
        id: 'fabric-api',
        kind: 'required',
        range: '>=0.90',
        status: 'missing',
        providerFile: null,
        providerVersion: null,
      },
    ],
    dependents: ['iris.jar'],
  }
}
async function mountDrawer(report: ModDependencyReport, mainId = 'fabric-api') {
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/', component: defineComponent({ render: () => h('div') }) },
      { path: '/comp', component: defineComponent({ render: () => h('div') }) },
      {
        path: '/comp/project/:platform/:id',
        component: defineComponent({ render: () => h('div') }),
      },
    ],
  })
  await router.push('/')
  await router.isReady()
  const pushSpy = vi.spyOn(router, 'push')

  mount(ModDependencyDrawer, {
    props: { open: true, entry: makeEntry(), report },
    global: { plugins: [i18n, router] },
  })
  await flushPromises()

  const row = document.body.querySelector<HTMLElement>(`[data-testid="dependency-main-${mainId}"]`)
  expect(row).not.toBeNull()
  return { pushSpy, row: row!, router }
}

afterEach(() => {
  document.body.innerHTML = ''
  vi.restoreAllMocks()
})

describe('ModDependencyDrawer', () => {
  it('renders a missing dependency row with its status badge', async () => {
    const { row } = await mountDrawer(missingReport())

    expect(row.textContent).toContain('fabric-api')
    const wrapper = document.body
    expect(wrapper.textContent).toContain('缺失')
    // The provider name of a declared-but-absent dependency is exactly what is missing.
    expect(wrapper.textContent).toContain('必需')
  })

  it('routes to the project page when a missing dependency is found online', async () => {
    vi.spyOn(ipcClient, 'compDepLookup').mockResolvedValue(makeProject({ id: 'P7dR8mSH' }))
    const { row, pushSpy } = await mountDrawer(missingReport())

    row.click()
    await flushPromises()

    expect(ipcClient.compDepLookup).toHaveBeenCalledWith('fabric-api')
    expect(pushSpy).toHaveBeenCalledWith('/comp/project/modrinth/P7dR8mSH')
  })

  it('falls back to a community search when the dependency is unknown', async () => {
    vi.spyOn(ipcClient, 'compDepLookup').mockResolvedValue(null)
    const { row, pushSpy } = await mountDrawer(missingReport())

    row.click()
    await flushPromises()

    expect(pushSpy).toHaveBeenCalledWith({ path: '/comp', query: { q: 'fabric-api' } })
  })

  it('renders an optional missing dependency as neutral optionalMissing status', async () => {
    const optionalReport: ModDependencyReport = {
      fileName: 'sodium.jar',
      modId: 'sodium',
      deps: [
        {
          id: 'cloth-config',
          kind: 'optional',
          range: null,
          status: 'missing',
          providerFile: null,
          providerVersion: null,
        },
      ],
      dependents: [],
    }
    const { row } = await mountDrawer(optionalReport, 'cloth-config')
    expect(row.textContent).toContain('cloth-config')
    expect(document.body.textContent).toContain('未安装 (可选)')
  })
})

describe('ResourceItemRow Chinese names', () => {
  it('shows the Chinese name as the title and the file name as the detail line', () => {
    const wrapper = mount(ResourceItemRow, {
      props: {
        entry: makeEntry({ zhName: '钠' }),
        kind: 'mod',
        selected: false,
        showZhName: true,
        localNameStyle: 0,
      },
      global: { plugins: [i18n] },
    })

    expect(wrapper.get('[data-testid="resource-row-title"]').text()).toBe('钠')
    expect(wrapper.get('[data-testid="resource-row-subtitle"]').text()).toBe('sodium.jar')
  })

  it('swaps the title and detail line for the file-name-first style', () => {
    const wrapper = mount(ResourceItemRow, {
      props: {
        entry: makeEntry({ zhName: '钠' }),
        kind: 'mod',
        selected: false,
        showZhName: true,
        localNameStyle: 1,
      },
      global: { plugins: [i18n] },
    })

    expect(wrapper.get('[data-testid="resource-row-title"]').text()).toBe('sodium.jar')
    expect(wrapper.get('[data-testid="resource-row-subtitle"]').text()).toBe('钠')
  })

  it('keeps the original title when Chinese names are disabled', () => {
    const wrapper = mount(ResourceItemRow, {
      props: {
        entry: makeEntry({ zhName: '钠' }),
        kind: 'mod',
        selected: false,
        showZhName: false,
        localNameStyle: 0,
      },
      global: { plugins: [i18n] },
    })

    expect(wrapper.get('[data-testid="resource-row-title"]').text()).toBe('Sodium')
    expect(wrapper.get('[data-testid="resource-row-subtitle"]').text()).toBe('sodium.jar')
  })
})

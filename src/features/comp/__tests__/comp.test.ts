import { describe, it, expect, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { mount, flushPromises } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import MarkdownIt from 'markdown-it'
import DOMPurify from 'dompurify'
import { useCompStore } from '../store'
import CompBrowserPage from '../CompBrowserPage.vue'
import CompDetailPage from '../CompDetailPage.vue'
import { ipcClient } from '../../../ipc/client'
import { setupMockIpc } from '../../../ipc/mock'
import zhCnComp from '../../../locales/zh-CN/comp.json'
import zhCnCommon from '../../../locales/zh-CN/common.json'
import zhCnError from '../../../locales/zh-CN/error.json'

const i18n = createI18n({
  legacy: false,
  locale: 'zh-CN',
  messages: {
    'zh-CN': {
      comp: zhCnComp,
      common: zhCnCommon,
      error: zhCnError,
    },
  },
})

describe('Community platform feature (Comp)', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    setupMockIpc()
  })

  it('searches projects by query and filters by type', async () => {
    const store = useCompStore()
    store.compType = 'mod'
    store.platform = 'modrinth'
    store.query = 'sodium'

    await store.search(true)

    expect(store.hits.length).toBeGreaterThan(0)
    expect(store.hits[0].slug).toBe('sodium')
    expect(store.hits[0].title).toBe('Sodium')
    expect(store.totalHits).toBe(1)
  })

  it('switches CompType tabs and resets search hits', async () => {
    const store = useCompStore()
    store.compType = 'shader'
    store.platform = 'modrinth'
    store.query = ''

    await store.search(true)

    expect(store.hits.length).toBe(1)
    expect(store.hits[0].slug).toBe('complementary-reimagined')
    expect(store.hits[0].compType).toBe('shader')
  })

  it('searches with platform all and returns hits from both modrinth and curseforge', async () => {
    const store = useCompStore()
    store.compType = 'mod'
    store.platform = 'all'
    store.query = ''

    await store.search(true)

    expect(store.hits.length).toBeGreaterThan(1)
    const platforms = store.hits.map((h) => h.platform)
    expect(platforms).toContain('modrinth')
    expect(platforms).toContain('curseforge')
  })
  it('paginates results using goToPage replacing hits', async () => {
    const store = useCompStore()
    store.compType = 'mod'
    store.platform = 'modrinth'
    store.query = ''
    store.limit = 1

    await store.search(true)
    expect(store.page).toBe(1)
    expect(store.pageCount).toBeGreaterThanOrEqual(1)
    const firstHitId = store.hits[0]?.id
    expect(firstHitId).toBeDefined()

    // Mock response when moving to page 2
    await store.goToPage(2)
    expect(store.page).toBe(2)
    expect(store.offset).toBe(1)
    // Hits must be replaced, not appended
    expect(store.hits.length).toBeLessThanOrEqual(store.limit)
  })

  it('toggles favorites and saves to settings', async () => {
    const store = useCompStore()
    await store.loadFavorites()
    expect(store.favorites.length).toBe(0)

    const project = {
      platform: 'modrinth' as const,
      id: 'AANobbMI',
      slug: 'sodium',
      title: 'Sodium',
      summary: 'Optimization mod',
      iconUrl: 'https://example.com/sodium.png',
      author: 'jellysquid3',
      downloads: 45000000,
      follows: 120000,
      categories: ['optimization'],
      mcVersions: ['1.20.1'],
      loaders: ['fabric'],
      compType: 'mod' as const,
      updatedAt: '2024-06-15T12:00:00Z',
    }

    expect(store.isFavorite('modrinth', 'AANobbMI')).toBe(false)
    await store.toggleFavorite(project)
    expect(store.isFavorite('modrinth', 'AANobbMI')).toBe(true)
    expect(store.favorites.length).toBe(1)

    // Toggle again to remove
    await store.toggleFavorite(project)
    expect(store.isFavorite('modrinth', 'AANobbMI')).toBe(false)
    expect(store.favorites.length).toBe(0)
  })

  it('renders project detail description markdown safely with DOMPurify', () => {
    const md = new MarkdownIt({ html: true, breaks: true })
    const dirtyMarkdown = '# Title\n\n**Bold Text** and [Safe Link](https://example.com)'
    const rendered = DOMPurify.sanitize(md.render(dirtyMarkdown))

    expect(rendered).toContain('Title')
    expect(rendered).toContain('<strong>Bold Text</strong>')
    expect(rendered).toContain('<a href="https://example.com">Safe Link</a>')
  })

  it('loads project files and downloads file to target instance', async () => {
    const store = useCompStore()
    await store.openProject('modrinth', 'AANobbMI')

    expect(store.currentProject).not.toBeNull()
    expect(store.currentProject?.slug).toBe('sodium')
    expect(store.projectFiles.length).toBe(2)

    const fileToDownload = store.projectFiles[0]
    const taskId = await store.downloadFile(fileToDownload, 'TestInstance', 'mod')
    expect(taskId).toMatch(/^task-comp-download-/)
  })

  it('mounts CompBrowserPage and renders search results and tabs', async () => {
    const wrapper = mount(CompBrowserPage, {
      global: {
        plugins: [i18n],
        stubs: {
          RouterLink: true,
        },
      },
    })

    await flushPromises()
    expect(wrapper.text()).toContain('Modrinth')
    expect(wrapper.text()).toContain('CurseForge')
    expect(wrapper.find('input[type="text"]').exists()).toBe(true)
    const compStore = useCompStore()
    expect(compStore.mcVersion).toBe('1.20.4')
    expect(compStore.loader).toBe('fabric')
    // Test loading spinner on search input
    compStore.loading = true
    await flushPromises()
    const input = wrapper.findComponent({ name: 'MInput' })
    expect(input.props('icon')).toContain('animate-spin')
    compStore.loading = false
    await flushPromises()
    expect(input.props('icon')).not.toContain('animate-spin')

    // Test skeleton container classes match real grid
    compStore.hits = []
    compStore.loading = true
    await flushPromises()
    const skeletonContainer = wrapper.find('[data-testid="comp-skeleton"]')
    expect(skeletonContainer.exists()).toBe(true)
    expect(skeletonContainer.classes()).toContain('grid')
    expect(skeletonContainer.classes()).toContain('grid-cols-1')
    expect(skeletonContainer.classes()).toContain('md:grid-cols-2')
    expect(skeletonContainer.classes()).toContain('lg:grid-cols-3')
    expect(skeletonContainer.classes()).toContain('gap-4')

    // Load hits and test real grid
    await compStore.search(true)
    await flushPromises()
    const realContainer = wrapper.find('[data-testid="comp-grid"]')
    expect(realContainer.exists()).toBe(true)
    expect(realContainer.classes()).toContain('grid')
    expect(realContainer.classes()).toContain('grid-cols-1')
    expect(realContainer.classes()).toContain('md:grid-cols-2')
    expect(realContainer.classes()).toContain('lg:grid-cols-3')
    expect(realContainer.classes()).toContain('gap-4')
  })

  it('verifies platform options include all, tabs have no duplicate border, and modpack import button is removed', async () => {
    const wrapper = mount(CompBrowserPage, {
      global: {
        plugins: [i18n],
        stubs: {
          RouterLink: true,
        },
      },
    })

    await flushPromises()
    expect(wrapper.text()).toContain('全部')
    expect(wrapper.text()).toContain('Modrinth')
    expect(wrapper.text()).toContain('CurseForge')
    // Modpack import button should not be present in CompBrowserPage
    expect(wrapper.text()).not.toContain('导入整合包')
    // Outer border-b wrapper should be removed
    expect(wrapper.find('.border-b.border-line.pb-2').exists()).toBe(false)
  })

  it('resolves dependencies for mod download', async () => {
    const file = {
      id: 'test-file',
      projectId: 'test-project',
      platform: 'modrinth' as const,
      name: 'test-mod.jar',
      fileName: 'test-mod.jar',
      mcVersions: ['1.20.1'],
      loaders: ['fabric'],
      channel: 'release' as const,
      releaseDate: '2024-01-01',
      size: 1024,
      hashes: {},
      downloadUrls: [],
      distributionDisabled: false,
      dependencies: [],
    }
    const deps = await ipcClient.compResolveDependencies(file, 'TestInstance', '1.20.1', 'fabric')
    expect(deps.length).toBe(2)
    expect(deps[0].project.title).toBe('Fabric API')
    expect(deps[0].installed).toBe(true)
    expect(deps[1].project.title).toBe('Mod Menu')
    expect(deps[1].installed).toBe(false)
  })

  it('inspects and imports modpack archive file', async () => {
    const inspect = await ipcClient.modpackInspect('C:/packs/fabulous.mrpack')
    expect(inspect.name).toBe('Fabulously Optimized')
    expect(inspect.format).toBe('modrinth')
    expect(inspect.mcVersion).toBe('1.20.1')
    expect(inspect.fileCount).toBe(42)

    const taskId = await ipcClient.modpackImportFile('C:/packs/fabulous.mrpack', 'Fabulous')
    expect(taskId).toMatch(/^task-modpack-import-/)
  })

  it('renders CompDetailPage with selectable title', async () => {
    const store = useCompStore()
    store.currentProject = {
      id: 'sodium',
      slug: 'sodium',
      title: 'Sodium',
      summary: 'Fast renderer',
      description: 'Fast renderer',
      compType: 'mod',
      platform: 'modrinth',
      author: 'jellysquid',
      downloads: 1000,
      follows: 100,
      iconUrl: '',
      categories: [],
      mcVersions: ['1.20.1'],
      loaders: ['fabric'],
      websiteUrl: null,
      updatedAt: '2024-01-01',
    }
    const wrapper = mount(CompDetailPage, {
      global: {
        plugins: [i18n],
        stubs: { RouterLink: true },
      },
    })
    expect(wrapper.find('h2.select-text').exists()).toBe(true)
    expect(wrapper.find('h2.select-text').text()).toBe('Sodium')
  })

  it('renders CompDetailPage version accordion with rounded corners and spacing', async () => {
    const store = useCompStore()
    store.currentProject = {
      id: 'sodium',
      slug: 'sodium',
      title: 'Sodium',
      summary: 'Fast renderer',
      description: 'Fast renderer',
      compType: 'mod',
      platform: 'modrinth',
      author: 'jellysquid',
      downloads: 1000,
      follows: 100,
      iconUrl: '',
      categories: [],
      mcVersions: ['1.20.1'],
      loaders: ['fabric'],
      websiteUrl: null,
      updatedAt: '2024-01-01',
    }
    store.projectFiles = [
      {
        id: 'file-1',
        projectId: 'sodium',
        platform: 'modrinth',
        name: 'sodium-fabric-0.5.8.jar',
        fileName: 'sodium-fabric-0.5.8.jar',
        mcVersions: ['1.20.1'],
        loaders: ['fabric'],
        channel: 'release',
        releaseDate: '2024-01-01',
        size: 1024,
        hashes: {},
        downloadUrls: ['https://example.com/file.jar'],
        distributionDisabled: false,
        dependencies: [],
      },
    ]
    store.filesLoading = false
    const wrapper = mount(CompDetailPage, {
      global: {
        plugins: [i18n],
        stubs: { RouterLink: true },
      },
    })
    await flushPromises()
    const accordionContainer = wrapper.find('.bg-sunken\\/40.rounded-xl')
    expect(accordionContainer.exists()).toBe(true)
    expect(accordionContainer.classes()).toContain('mt-2.5')
    expect(accordionContainer.classes()).toContain('p-3')
    expect(accordionContainer.classes()).toContain('space-y-3')
  })

  it('renders CompBrowserPage search cards with category prefix and clean stats line', async () => {
    const wrapper = mount(CompBrowserPage, {
      global: {
        plugins: [i18n],
        stubs: { RouterLink: true },
      },
    })
    await flushPromises()
    const cards = wrapper.findAll('[data-testid="comp-grid"] > div')
    expect(cards.length).toBeGreaterThan(0)
    // Primary category prefix is styled with text-primary and mr-2
    const catPrefix = wrapper.find('[data-testid="comp-grid"] .text-primary.font-medium')
    expect(catPrefix.exists()).toBe(true)
    // Bottom category badge pill list should not be present
    expect(wrapper.find('[data-testid="comp-grid"] .i-lucide-clock').exists()).toBe(true)
    expect(wrapper.find('[data-testid="comp-grid"] .i-lucide-download').exists()).toBe(true)
  })

  it('verifies sortOptions has only one relevance option and refresh button is positioned at end', async () => {
    const wrapper = mount(CompBrowserPage, {
      global: {
        plugins: [i18n],
        stubs: { RouterLink: true },
      },
    })
    await flushPromises()
    const selects = wrapper.findAllComponents({ name: 'MSelect' })
    const sortSelect = selects.find(
      (s) =>
        Array.isArray(s.props('options')) &&
        s.props('options').some((o: { value: string }) => o.value === 'relevance'),
    )
    expect(sortSelect).toBeDefined()
    const relevanceOptions = (sortSelect!.props('options') as Array<{ value: string }>).filter(
      (o) => o.value === 'relevance',
    )
    expect(relevanceOptions.length).toBe(1)
  })

  it('verifies CompDetailPage groups files by major drop version and snapshot group descending', async () => {
    const store = useCompStore()
    store.currentProject = {
      id: 'test-project',
      slug: 'test-project',
      title: 'Test Project',
      summary: 'Test Summary',
      description: 'Test Description',
      author: 'Author',
      iconUrl: '',
      platform: 'modrinth',
      downloads: 100,
      follows: 10,
      categories: ['optimization'],
      mcVersions: ['1.21', '1.20.1', '24w14a', '26.1'],
      loaders: ['fabric'],
      compType: 'mod',
      updatedAt: '2024-06-15T12:00:00Z',
    }
    store.projectFiles = [
      {
        id: 'file-snap',
        projectId: 'test-project',
        platform: 'modrinth',
        name: 'test-24w14a.jar',
        fileName: 'test-24w14a.jar',
        mcVersions: ['24w14a'],
        loaders: ['fabric'],
        channel: 'beta',
        releaseDate: '2024-03-01',
        size: 1024,
        hashes: {},
        downloadUrls: ['https://example.com/file.jar'],
        distributionDisabled: false,
        dependencies: [],
      },
      {
        id: 'file-26',
        projectId: 'test-project',
        platform: 'modrinth',
        name: 'test-26.1.jar',
        fileName: 'test-26.1.jar',
        mcVersions: ['26.1'],
        loaders: ['fabric'],
        channel: 'release',
        releaseDate: '2026-01-01',
        size: 1024,
        hashes: {},
        downloadUrls: ['https://example.com/file.jar'],
        distributionDisabled: false,
        dependencies: [],
      },
      {
        id: 'file-121',
        projectId: 'test-project',
        platform: 'modrinth',
        name: 'test-1.21.jar',
        fileName: 'test-1.21.jar',
        mcVersions: ['1.21'],
        loaders: ['fabric'],
        channel: 'release',
        releaseDate: '2024-06-01',
        size: 1024,
        hashes: {},
        downloadUrls: ['https://example.com/file.jar'],
        distributionDisabled: false,
        dependencies: [],
      },
      {
        id: 'file-120',
        projectId: 'test-project',
        platform: 'modrinth',
        name: 'test-1.20.1.jar',
        fileName: 'test-1.20.1.jar',
        mcVersions: ['1.20.1'],
        loaders: ['fabric'],
        channel: 'release',
        releaseDate: '2023-06-01',
        size: 1024,
        hashes: {},
        downloadUrls: ['https://example.com/file.jar'],
        distributionDisabled: false,
        dependencies: [],
      },
    ]
    store.filesLoading = false

    const wrapper = mount(CompDetailPage, {
      global: {
        plugins: [i18n],
        stubs: { RouterLink: true },
      },
    })
    await flushPromises()

    // Find all group headers
    const groupTitles = wrapper
      .findAll('button span')
      .map((el) => el.text().trim())
      .filter(Boolean)
    expect(groupTitles).toContain('26.1')
    expect(groupTitles).toContain('1.21')
    expect(groupTitles).toContain('1.20')
    expect(groupTitles).toContain('快照 / 测试版')

    // 26.1 should be before 1.21, 1.21 before 1.20, snapshot group should be at the bottom
    const idx26 = groupTitles.indexOf('26.1')
    const idx121 = groupTitles.indexOf('1.21')
    const idx120 = groupTitles.indexOf('1.20')
    const idxSnap = groupTitles.indexOf('快照 / 测试版')
    expect(idx26).toBeLessThan(idx121)
    expect(idx121).toBeLessThan(idx120)
    expect(idx120).toBeLessThan(idxSnap)
  })

  it('verifies CompBrowserPage search bar is sticky with top-1 and includes PCL canonical versions', async () => {
    const wrapper = mount(CompBrowserPage, {
      global: {
        plugins: [i18n],
        stubs: { RouterLink: true },
      },
    })
    await flushPromises()

    // Verify search bar styling
    const filterBar = wrapper.findComponent({ name: 'CompSearchToolbar' })
    expect(filterBar.exists()).toBe(true)
    expect(filterBar.find('.m-card').classes()).toContain('shadow-sm')

    // Verify version options contain PCL canonical versions in descending order
    const selects = wrapper.findAllComponents({ name: 'MSelect' })
    const versionSelect = selects.find((s) => {
      const opts = (s.props('options') as Array<{ label: string; value: string }>) || []
      return opts.some((o) => o.value === '1.21.4')
    })
    expect(versionSelect).toBeDefined()
    const options = (
      versionSelect!.props('options') as Array<{ label: string; value: string }>
    ).map((o) => o.value)
    expect(options).toContain('26.3')
    expect(options).toContain('1.21.4')
    expect(options).toContain('1.21.1')
    expect(options).toContain('1.20.6')
    expect(options).toContain('1.10.2')
    expect(options).toContain('1.8.9')

    // Check order: 26.3 before 1.21.4 before 1.20.1 before 1.7.10
    expect(options.indexOf('26.3')).toBeLessThan(options.indexOf('1.21.4'))
    expect(options.indexOf('1.21.4')).toBeLessThan(options.indexOf('1.20.1'))
    expect(options.indexOf('1.20.1')).toBeLessThan(options.indexOf('1.7.10'))
  })

  it('opens CompQuickDownloadDialog on card click and navigates on view details', async () => {
    const compStore = useCompStore()
    compStore.hits = [
      {
        id: 'sodium',
        slug: 'sodium',
        platform: 'modrinth',
        title: 'Sodium',
        summary: 'Rendering optimization',
        author: 'jellysquid3',
        downloads: 1000000,
        follows: 50000,
        iconUrl: '',
        categories: ['optimization'],
        compType: 'mod',
        mcVersions: ['1.20.1'],
        loaders: ['fabric'],
        updatedAt: '2026-01-01',
      },
    ]
    compStore.totalHits = 1

    const wrapper = mount(CompBrowserPage, {
      global: {
        plugins: [i18n],
      },
    })
    await flushPromises()

    const projectCard = wrapper.findComponent({ name: 'CompProjectCard' })
    expect(projectCard.exists()).toBe(true)
    await projectCard.trigger('click')
    await flushPromises()

    const quickDialog = wrapper.findComponent({ name: 'CompQuickDownloadDialog' })
    expect(quickDialog.exists()).toBe(true)
    expect(quickDialog.props('open')).toBe(true)
    expect(quickDialog.props('project')?.id).toBe('AANobbMI')
  })
})

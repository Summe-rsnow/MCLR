import { describe, it, expect, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { mount, flushPromises } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import MarkdownIt from 'markdown-it'
import DOMPurify from 'dompurify'
import { useCompStore } from '../store'
import CompBrowserPage from '../CompBrowserPage.vue'
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
  })
})

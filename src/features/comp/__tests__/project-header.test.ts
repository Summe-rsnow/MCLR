import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import CompProjectHeader from '../components/CompProjectHeader.vue'
import type { CompProject } from '../../../ipc/client'
import zhCnComp from '../../../locales/zh-CN/comp.json'

const i18n = createI18n({
  legacy: false,
  locale: 'zh-CN',
  messages: {
    'zh-CN': {
      comp: zhCnComp,
    },
  },
})

const mockProject: CompProject = {
  id: 'sodium',
  slug: 'sodium',
  title: 'Sodium',
  summary: 'A modern rendering engine for Minecraft',
  description: 'A modern rendering engine for Minecraft',
  author: '4reLOAKe',
  iconUrl: 'https://cdn.modrinth.com/data/AANobbMI/icon.png',
  downloads: 259435605,
  follows: 42477,
  platform: 'modrinth',
  compType: 'mod',
  categories: ['optimization'],
  mcVersions: ['1.20.1'],
  loaders: ['fabric', 'quilt'],
  updatedAt: '2026-10-16T00:00:00Z',
  websiteUrl: 'https://modrinth.com/mod/sodium',
  wikiId: null,
  zhName: '钠',
}

describe('CompProjectHeader', () => {
  it('renders project icon using MImage with iconUrl', () => {
    const wrapper = mount(CompProjectHeader, {
      props: {
        project: mockProject,
        isFavorite: false,
        showZhName: true,
      },
      global: {
        plugins: [i18n],
      },
    })

    const img = wrapper.find('img')
    expect(img.exists()).toBe(true)
    expect(img.attributes('src')).toBe(mockProject.iconUrl)
  })

  it('renders fallback icon when iconUrl is null', () => {
    const noIconProject = { ...mockProject, iconUrl: null }
    const wrapper = mount(CompProjectHeader, {
      props: {
        project: noIconProject,
        isFavorite: false,
        showZhName: true,
      },
      global: {
        plugins: [i18n],
      },
    })

    const img = wrapper.find('img')
    expect(img.exists()).toBe(false)
    expect(wrapper.find('.i-lucide-puzzle').exists()).toBe(true)
  })
})

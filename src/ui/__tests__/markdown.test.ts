import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import MMarkdown from '../MMarkdown.vue'

vi.mock('@tauri-apps/plugin-opener', () => ({
  openUrl: vi.fn(),
}))

describe('MMarkdown', () => {
  it('renders center/img/p/comment markdown snippet correctly without raw tags or comments', () => {
    const source = `**bold text**\n\n<center><img src="https://example.com/a.png" alt="x"><p><!-- c --></p></center>`
    const wrapper = mount(MMarkdown, {
      props: {
        source,
        format: 'markdown',
      },
    })

    const img = wrapper.find('img')
    expect(img.exists()).toBe(true)
    expect(img.attributes('src')).toBe('https://example.com/a.png')
    expect(img.attributes('alt')).toBe('x')
    expect(img.attributes('loading')).toBe('lazy')
    expect(img.attributes('referrerpolicy')).toBe('no-referrer')

    expect(wrapper.html()).not.toContain('&lt;center')
    expect(wrapper.html()).not.toContain('<!--')
    expect(wrapper.find('strong').text()).toBe('bold text')
  })
  it('strips script and iframe tags for security', () => {
    const source = `<script>window.__mclr_unsafe = 1</script><iframe src="https://example.com/embed"></iframe>Hello`
    const wrapper = mount(MMarkdown, {
      props: {
        source,
        format: 'html',
      },
    })
    expect(wrapper.find('script').exists()).toBe(false)
    expect(wrapper.find('iframe').exists()).toBe(false)
    expect(wrapper.text()).toContain('Hello')
    expect('__mclr_unsafe' in window).toBe(false)
  })

  it('disallows javascript: links', () => {
    const source = `[x](javascript:alert(1))`
    const wrapper = mount(MMarkdown, {
      props: {
        source,
        format: 'markdown',
      },
    })

    const a = wrapper.find('a')
    if (a.exists()) {
      expect(a.attributes('href')).toBeUndefined()
    }
  })

  it('adds target="_blank" and rel to rendered links', () => {
    const source = `[link](https://example.com)`
    const wrapper = mount(MMarkdown, {
      props: {
        source,
        format: 'markdown',
      },
    })

    const a = wrapper.find('a')
    expect(a.exists()).toBe(true)
    expect(a.attributes('target')).toBe('_blank')
    expect(a.attributes('rel')).toBe('noopener noreferrer nofollow')
  })

  it('renders format="html" verbatim as elements', () => {
    const source = `<p>a</p><ul><li>b</li></ul>`
    const wrapper = mount(MMarkdown, {
      props: {
        source,
        format: 'html',
      },
    })

    expect(wrapper.find('p').text()).toBe('a')
    expect(wrapper.find('ul li').text()).toBe('b')
  })
})

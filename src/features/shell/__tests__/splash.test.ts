import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { mount, flushPromises } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import StartupSplash from '../StartupSplash.vue'
import { useThemeStore } from '../../../stores/theme'
import zhCnApp from '../../../locales/zh-CN/app.json'

const i18n = createI18n({
  legacy: false,
  locale: 'zh-CN',
  messages: {
    'zh-CN': {
      app: zhCnApp,
    },
  },
})

describe('StartupSplash component', () => {
  let storage: Record<string, string> = {}

  beforeEach(() => {
    setActivePinia(createPinia())
    vi.useFakeTimers()
    storage = {}
    const mockStorage = {
      getItem: (k: string) => (k in storage ? storage[k] : null),
      setItem: (k: string, v: string) => {
        storage[k] = String(v)
      },
      removeItem: (k: string) => {
        delete storage[k]
      },
      clear: () => {
        storage = {}
      },
    }
    Object.defineProperty(window, 'localStorage', {
      value: mockStorage,
      writable: true,
      configurable: true,
    })
  })
  afterEach(() => {
    vi.useRealTimers()
  })

  it('emits done after 1600ms on full motion and not before 1500ms', async () => {
    const themeStore = useThemeStore()
    themeStore.motion = 'full'

    const wrapper = mount(StartupSplash, {
      global: {
        plugins: [i18n],
      },
    })

    expect(wrapper.emitted('done')).toBeUndefined()

    // Advance 1400ms: fade should have started (isFadingOut = true), but done not yet emitted
    vi.advanceTimersByTime(1400)
    await flushPromises()
    expect(wrapper.classes()).toContain('m-splash-out')
    expect(wrapper.emitted('done')).toBeUndefined()

    // Advance remaining to 1600ms: done emitted
    vi.advanceTimersByTime(200)
    await flushPromises()
    expect(wrapper.emitted('done')).toBeDefined()
    expect(wrapper.emitted('done')!.length).toBe(1)
  })

  it('emits done after 700ms on reduced motion and fade out starts at 500ms', async () => {
    const themeStore = useThemeStore()
    themeStore.motion = 'reduced'

    const wrapper = mount(StartupSplash, {
      global: {
        plugins: [i18n],
      },
    })

    expect(wrapper.emitted('done')).toBeUndefined()

    vi.advanceTimersByTime(550)
    await flushPromises()
    expect(wrapper.classes()).toContain('m-splash-out')
    expect(wrapper.emitted('done')).toBeUndefined()

    vi.advanceTimersByTime(150)
    await flushPromises()
    expect(wrapper.emitted('done')).toBeDefined()
    expect(wrapper.emitted('done')!.length).toBe(1)
  })

  it('dismisses immediately when clicked', async () => {
    const themeStore = useThemeStore()
    themeStore.motion = 'full'

    const wrapper = mount(StartupSplash, {
      global: {
        plugins: [i18n],
      },
    })

    expect(wrapper.emitted('done')).toBeUndefined()

    await wrapper.trigger('click')
    expect(wrapper.emitted('done')).toBeDefined()
    expect(wrapper.emitted('done')!.length).toBe(1)
  })

  it('emits done immediately when motion is off', async () => {
    const themeStore = useThemeStore()
    themeStore.motion = 'off'

    const wrapper = mount(StartupSplash, {
      global: {
        plugins: [i18n],
      },
    })

    expect(wrapper.emitted('done')).toBeDefined()
  })

  it('persists startupSplash in themeStore and localStorage', () => {
    const themeStore = useThemeStore()
    expect(themeStore.startupSplash).toBe(true)

    themeStore.setStartupSplash(false)
    expect(themeStore.startupSplash).toBe(false)
    expect(localStorage.getItem('mclr.splash')).toBe('off')

    // Fresh store reads off
    const newStore = useThemeStore()
    expect(newStore.startupSplash).toBe(false)

    themeStore.setStartupSplash(true)
    expect(themeStore.startupSplash).toBe(true)
    expect(localStorage.getItem('mclr.splash')).toBe('on')
  })
})

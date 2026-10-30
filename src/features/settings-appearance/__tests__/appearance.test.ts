import { describe, it, expect, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { mount, flushPromises } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import SettingsAppearancePage from '../SettingsAppearancePage.vue'
import { useThemeStore } from '../../../stores/theme'
import { setupMockIpc } from '../../../ipc/mock'
import zhCnSettingsAppearance from '../../../locales/zh-CN/settingsAppearance.json'
import zhCnShell from '../../../locales/zh-CN/shell.json'
import zhCnApp from '../../../locales/zh-CN/app.json'
import zhCnCommon from '../../../locales/zh-CN/common.json'

const i18n = createI18n({
  legacy: false,
  locale: 'zh-CN',
  messages: {
    'zh-CN': {
      settingsAppearance: zhCnSettingsAppearance,
      shell: zhCnShell,
      app: zhCnApp,
      common: zhCnCommon,
    },
  },
})

describe('Appearance and Personalisation feature', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    setupMockIpc()
    // Motion/splash preferences live in localStorage, so tests that change them
    // would otherwise leak into every later default tier.
    localStorage.clear()
    // Reset document styles
    document.documentElement.style.fontSize = ''
  })

  it('renders SettingsAppearancePage with theme, scale, and background sections cleanly', async () => {
    const wrapper = mount(SettingsAppearancePage, {
      global: {
        plugins: [i18n],
      },
    })
    await flushPromises()

    expect(wrapper.text()).toContain('外观与个性化设置')
    expect(wrapper.text()).toContain('主题与材质')
    expect(wrapper.text()).toContain('强调色 (种子颜色)')
    expect(wrapper.text()).toContain('界面缩放')
    expect(wrapper.text()).toContain('背景设置')
    expect(wrapper.text()).not.toContain('背景音乐')
    expect(wrapper.text()).toContain('查看完整组件库')
  })

  it('updates seed color and mode in useThemeStore', async () => {
    const store = useThemeStore()
    expect(store.seed).toBe('#3b82f6')

    store.setSeed('#ff0000')
    expect(store.seed).toBe('#ff0000')

    store.setMode('dark')
    expect(store.mode).toBe('dark')

    await store.setMaterial('glass')
    expect(store.material).toBe('glass')
  })
  it('maps the performance tier onto material, motion and startup splash', async () => {
    const store = useThemeStore()
    expect(store.material).toBe('flat')
    expect(store.motion).toBe('full')
    expect(store.startupSplash).toBe(true)
    expect(store.performanceTier).toBe('balanced')

    await store.setPerformanceTier('performance')
    expect(store.material).toBe('flat')
    expect(store.motion).toBe('off')
    expect(store.startupSplash).toBe(false)
    expect(store.performanceTier).toBe('performance')

    await store.setPerformanceTier('quality')
    expect(store.material).toBe('glass')
    expect(store.motion).toBe('full')
    expect(store.startupSplash).toBe(true)
    expect(store.performanceTier).toBe('quality')

    await store.setPerformanceTier('balanced')
    expect(store.material).toBe('flat')
    expect(store.motion).toBe('reduced')
    expect(store.startupSplash).toBe(true)
  })

  it('derives the tier when material or motion is changed elsewhere', async () => {
    const store = useThemeStore()

    // Quick settings can still flip the material on its own.
    await store.setMaterial('glass')
    expect(store.performanceTier).toBe('quality')

    await store.setMaterial('flat')
    store.setMotion('off')
    expect(store.performanceTier).toBe('performance')
  })

  it('keeps the original motion and splash controls alongside the slider', async () => {
    const store = useThemeStore()
    expect(store.motion).toBe('full')
    store.setMotion('reduced')
    expect(store.motion).toBe('reduced')
    store.setMotion('off')
    expect(store.motion).toBe('off')

    const wrapper = mount(SettingsAppearancePage, {
      global: {
        plugins: [i18n],
      },
    })
    await flushPromises()
    expect(wrapper.text()).toContain('动画效果')
    expect(wrapper.text()).toContain('完整')
    expect(wrapper.text()).toContain('精简')
    expect(wrapper.text()).toContain('关闭')
    expect(wrapper.text()).toContain('高级材质效果')
    expect(wrapper.text()).toContain('扁平纯色')
    expect(wrapper.text()).toContain('流光玻璃')
    expect(wrapper.text()).toContain('启动动画')

    const motionRow = wrapper
      .findAll('.m-setting-row')
      .find((row) => row.text().includes('动画效果'))
    expect(motionRow).toBeDefined()
    // Cheapest first: closing the animation sits on the left, matching the tier slider.
    expect(
      motionRow!
        .findAll('button')
        .map((b) => b.text())
        .filter(Boolean),
    ).toEqual(['关闭', '精简', '完整'])

    store.setStartupSplash(false)
    expect(store.startupSplash).toBe(false)
  })

  it('moves the tier slider when a manual control changes', async () => {
    const store = useThemeStore()
    const wrapper = mount(SettingsAppearancePage, {
      global: {
        plugins: [i18n],
      },
    })
    await flushPromises()

    const slider = wrapper.find('[data-testid="perf-tier-slider"] input[type="range"]')
    expect((slider.element as HTMLInputElement).value).toBe('1')

    await store.setMaterial('glass')
    await flushPromises()
    expect((slider.element as HTMLInputElement).value).toBe('2')
  })

  it('renders the tier slider with three stops and applies a picked tier', async () => {
    const store = useThemeStore()
    const wrapper = mount(SettingsAppearancePage, {
      global: {
        plugins: [i18n],
      },
    })
    await flushPromises()

    expect(wrapper.text()).toContain('性能档位')

    const slider = wrapper.find('[data-testid="perf-tier-slider"]')
    expect(slider.exists()).toBe(true)
    expect(slider.findAll('button').map((b) => b.text())).toEqual(['性能', '均衡', '质量'])
    // Balanced is active by default, and the hint spells out the combination.
    expect(slider.findAll('button')[1].classes()).toContain('text-primary')
    expect(wrapper.text()).toContain('扁平纯色 · 精简动画 · 播放启动动画')

    const range = slider.find('input[type="range"]')
    expect(range.attributes('max')).toBe('2')
    await range.setValue('0')

    expect(store.performanceTier).toBe('performance')
    expect(store.motion).toBe('off')
    expect(store.startupSplash).toBe(false)
    expect(wrapper.text()).toContain('扁平纯色 · 关闭动画 · 跳过启动动画')

    // Clicking a stop label jumps there.
    await slider.findAll('button')[2].trigger('click')
    expect(store.material).toBe('glass')
    expect(store.performanceTier).toBe('quality')
  })

  it('applies UI scale to document.documentElement', async () => {
    const store = useThemeStore()

    store.setScale(1.2)
    expect(document.documentElement.style.fontSize).toBe(`${1.2 * 16}px`)
  })

  it('renders single-image background preview and handles clear button', async () => {
    const store = useThemeStore()
    store.setBackgroundMode('image')
    store.setBackgroundPath('C:/images/background.png')
    store.setBackgroundBlur(10)
    store.setBackgroundOpacity(0.5)

    const pageWrapper = mount(SettingsAppearancePage, {
      global: {
        plugins: [i18n],
      },
    })
    await flushPromises()

    const previewImg = pageWrapper.find('[data-testid="bg-preview-img"]')
    expect(previewImg.exists()).toBe(true)

    // Find clear button (with icon i-lucide-x)
    const clearBtn = pageWrapper.find('button[class*="i-lucide-x"], button .i-lucide-x')
    expect(clearBtn.exists()).toBe(true)
    await clearBtn.trigger('click')
    await flushPromises()

    expect(store.backgroundPath).toBe('')
    expect(store.backgroundMode).toBe('none')
  })

  it('renders empty state without contrast sample card when backgroundMode is none or no media path', async () => {
    const store = useThemeStore()
    store.setBackgroundMode('none')
    store.setBackgroundPath('')

    const pageWrapper = mount(SettingsAppearancePage, {
      global: {
        plugins: [i18n],
      },
    })
    await flushPromises()

    expect(pageWrapper.find('[data-testid="bg-preview-video"]').exists()).toBe(false)
    expect(pageWrapper.find('[data-testid="bg-preview-img"]').exists()).toBe(false)
    expect(pageWrapper.text()).toContain('未选择背景媒体或模式为无')
    expect(pageWrapper.text()).not.toContain('Sample UI Foreground Content')
  })
})

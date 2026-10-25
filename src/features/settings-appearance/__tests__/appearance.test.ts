import { describe, it, expect, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { mount, flushPromises } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import SettingsAppearancePage from '../SettingsAppearancePage.vue'
import AppShell from '../../shell/AppShell.vue'
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
  it('updates motion mode in useThemeStore and renders segmented control', async () => {
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

  it('renders video element when backgroundMode is video in SettingsAppearancePage preview and AppShell', async () => {
    const store = useThemeStore()
    store.setBackgroundMode('video')
    store.setBackgroundPath('C:/videos/background.mp4')
    store.setBackgroundBlur(10)
    store.setBackgroundOpacity(0.5)

    const pageWrapper = mount(SettingsAppearancePage, {
      global: {
        plugins: [i18n],
      },
    })
    await flushPromises()

    const previewVideo = pageWrapper.find('[data-testid="bg-preview-video"]')
    expect(previewVideo.exists()).toBe(true)

    const shellWrapper = mount(AppShell, {
      global: {
        plugins: [i18n],
        stubs: {
          RouterView: true,
          TaskIndicator: true,
          CrashDialog: true,
        },
      },
    })
    await flushPromises()

    const bgVideo = shellWrapper.find('[data-testid="bg-video"]')
    expect(bgVideo.exists()).toBe(true)
    expect(bgVideo.attributes('autoplay')).toBeDefined()
    expect(bgVideo.attributes('muted')).toBeDefined()
    expect(bgVideo.attributes('loop')).toBeDefined()
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

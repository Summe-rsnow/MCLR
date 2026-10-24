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

const i18n = createI18n({
  legacy: false,
  locale: 'zh-CN',
  messages: {
    'zh-CN': {
      settingsAppearance: zhCnSettingsAppearance,
      shell: zhCnShell,
      app: zhCnApp,
    },
  },
})

describe('Appearance and Personalisation feature', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    setupMockIpc()
    // Reset document styles
    document.documentElement.style.fontFamily = ''
    document.documentElement.style.fontSize = ''
  })

  it('renders SettingsAppearancePage with theme, typography, background and music sections', async () => {
    const wrapper = mount(SettingsAppearancePage, {
      global: {
        plugins: [i18n],
      },
    })
    await flushPromises()

    expect(wrapper.text()).toContain('外观与个性化设置')
    expect(wrapper.text()).toContain('主题与材质')
    expect(wrapper.text()).toContain('强调色 (种子颜色)')
    expect(wrapper.text()).toContain('字体与缩放')
    expect(wrapper.text()).toContain('背景设置')
    expect(wrapper.text()).toContain('背景音乐')
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

  it('applies fontFamily and UI scale to document.documentElement', async () => {
    const store = useThemeStore()

    store.setFontFamily('Consolas, monospace')
    expect(document.documentElement.style.fontFamily).toBe('Consolas, monospace')

    store.setScale(1.2)
    expect(document.documentElement.style.fontSize).toBe(`${1.2 * 16}px`)
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

  it('handles music controller in AppShell and supports auto-pause on launch and resume on exit', async () => {
    const store = useThemeStore()
    store.setMusicEnabled(true)
    store.playlist = ['C:/music/track1.mp3', 'C:/music/track2.mp3']
    store.currentTrackIndex = 0

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

    const controls = shellWrapper.find('[data-testid="music-controls"]')
    expect(controls.exists()).toBe(true)

    const playBtn = shellWrapper.find('[data-testid="music-play-btn"]')
    expect(playBtn.exists()).toBe(true)
    await playBtn.trigger('click')
    expect(store.isPlaying).toBe(true)

    const nextBtn = shellWrapper.find('[data-testid="music-next-btn"]')
    await nextBtn.trigger('click')
    expect(store.currentTrackIndex).toBe(1)

    // Simulate game launch auto-pause logic
    store.isPlaying = true
    if (store.isPlaying) {
      store.pausedByGame = true
      store.isPlaying = false
    }
    expect(store.isPlaying).toBe(false)
    expect(store.pausedByGame).toBe(true)

    // Simulate game exit auto-resume logic
    if (store.pausedByGame) {
      store.pausedByGame = false
      if (store.musicEnabled) {
        store.isPlaying = true
      }
    }
    expect(store.isPlaying).toBe(true)
    expect(store.pausedByGame).toBe(false)
  })
})

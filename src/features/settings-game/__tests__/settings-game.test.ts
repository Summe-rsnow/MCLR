import { describe, it, expect, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { mount, flushPromises } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import SettingsGamePage from '../SettingsGamePage.vue'
import { setupMockIpc } from '../../../ipc/mock'
import { ipcClient } from '../../../ipc/client'
import zhCnSettingsGame from '../../../locales/zh-CN/settingsGame.json'
import zhCnCommon from '../../../locales/zh-CN/common.json'

const i18n = createI18n({
  legacy: false,
  locale: 'zh-CN',
  messages: {
    'zh-CN': {
      settingsGame: zhCnSettingsGame,
      common: zhCnCommon,
    },
  },
})

describe('Settings Game feature', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    setupMockIpc()
  })

  it('renders SettingsGamePage with launch settings without java tab', async () => {
    const wrapper = mount(SettingsGamePage, {
      global: {
        plugins: [i18n],
      },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('游戏运行与环境设置')
    expect(wrapper.text()).toContain('内存分配')
    expect(wrapper.text()).toContain('游戏窗口与显示')
    expect(wrapper.find('[data-testid="tab-launch"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="tab-java"]').exists()).toBe(false)
  })

  it('patches launch settings correctly', async () => {
    const updated = await ipcClient.settingsPatch({
      launch: {
        ramMode: 'custom',
        ramCustomMb: 8192,
        fullscreen: true,
      },
    })

    expect(updated.launch?.ramMode).toBe('custom')
    expect(updated.launch?.ramCustomMb).toBe(8192)
    expect(updated.launch?.fullscreen).toBe(true)
  })

  it('renders the advanced launch options panel with PCL parity rows', async () => {
    const wrapper = mount(SettingsGamePage, {
      global: { plugins: [i18n] },
    })
    await flushPromises()

    const text = wrapper.text()
    expect(text).toContain('高级启动选项')
    expect(text).toContain('渲染器')
    expect(text).toContain('JVM 参数头部')
    expect(text).toContain('游戏参数尾部')
    expect(text).toContain('启动前执行命令')
    expect(text).toContain('要求 Java 使用高性能显卡')
    expect(text).toContain('使用 java.exe 而不是 javaw.exe')
    expect(text).toContain('锁定内存分配')
    expect(text).toContain('禁用内置崩溃分析')
  })

  it('persists the new advanced launch toggles', async () => {
    const updated = await ipcClient.settingsPatch({
      launch: {
        renderer: 'vulkan',
        preLaunchCommand: 'echo hi',
        useJavaExe: true,
        requireGpuHighPerformance: true,
        lockMemory: true,
      },
    })

    expect(updated.launch?.renderer).toBe('vulkan')
    expect(updated.launch?.preLaunchCommand).toBe('echo hi')
    expect(updated.launch?.useJavaExe).toBe(true)
    expect(updated.launch?.requireGpuHighPerformance).toBe(true)
    expect(updated.launch?.lockMemory).toBe(true)
  })

  it('gives every advanced launch row its own explanation tooltip', async () => {
    const wrapper = mount(SettingsGamePage, {
      global: { plugins: [i18n] },
    })
    await flushPromises()

    const section = wrapper
      .findAll('section, div')
      .find((el) => el.text().includes('高级启动选项') && el.text().includes('渲染器'))
    expect(section).toBeTruthy()

    const advanced = wrapper
      .findAllComponents({ name: 'MSection' })
      .find((c) => c.text().includes('高级启动选项'))
    expect(advanced).toBeTruthy()

    const labels = advanced!.findAll('button.cursor-help').map((b) => b.attributes('aria-label'))
    for (const key of [
      'rendererHint',
      'jvmArgsHeadHint',
      'gameArgsTailHint',
      'preLaunchCommandHint',
      'requireGpuHint',
      'useJavaExeHint',
      'lockMemoryHint',
      'disableCrashAnalysisHint',
    ]) {
      const text = i18n.global.t(`settingsGame.launch.${key}`)
      expect((zhCnSettingsGame.launch as Record<string, string>)[key], key).toBeTruthy()
      expect(labels, key).toContain(text)
    }
  })

  it('defaults the JVM argument head and the GPU preference on', async () => {
    const wrapper = mount(SettingsGamePage, {
      global: { plugins: [i18n] },
    })
    await flushPromises()

    const textarea = wrapper.find('textarea')
    expect(textarea.exists()).toBe(true)
    expect((textarea.element as HTMLTextAreaElement).value).toContain('-XX:+UseG1GC')

    const gpuRow = wrapper
      .findAllComponents({ name: 'MSettingRow' })
      .find((r) => r.text().includes('要求 Java 使用高性能显卡'))
    expect(gpuRow).toBeTruthy()
    expect(gpuRow!.findComponent({ name: 'MSwitch' }).props('modelValue')).toBe(true)
  })
})

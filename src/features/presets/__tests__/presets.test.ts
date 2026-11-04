import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { mount, flushPromises } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { ipcClient } from '../../../ipc/client'
import { setupMockIpc } from '../../../ipc/mock'
import zhCnPresets from '../../../locales/zh-CN/presets.json'
import zhCnCommon from '../../../locales/zh-CN/common.json'
import zhCnError from '../../../locales/zh-CN/error.json'
import PresetsPage from '../PresetsPage.vue'
import PresetPinDrawer from '../components/PresetPinDrawer.vue'
import PresetApplyDialog from '../components/PresetApplyDialog.vue'
import ApplyReportDialog from '../components/ApplyReportDialog.vue'
import { usePresetsStore } from '../store'
import { mockApplyReport, mockPresets } from '../mock'

const i18n = createI18n({
  legacy: false,
  locale: 'zh-CN',
  messages: {
    'zh-CN': {
      presets: zhCnPresets,
      common: zhCnCommon,
      error: zhCnError,
    },
  },
})

function bodyButtons(): HTMLButtonElement[] {
  return Array.from(document.body.querySelectorAll('button'))
}

describe('Resource presets feature', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    setupMockIpc()
  })

  afterEach(() => {
    vi.restoreAllMocks()
    document.body.innerHTML = ''
  })

  it('lists the stored presets on the page', async () => {
    const wrapper = mount(PresetsPage, { global: { plugins: [i18n] } })
    await flushPromises()

    const text = wrapper.text()
    expect(text).toContain(mockPresets[0].name)
    expect(text).toContain(mockPresets[1].name)
    // First preset is auto-selected, so its items and pin badges are visible.
    expect(text).toContain('跟随最新')
    expect(wrapper.findAll('[data-testid="preset-item-row"]').length).toBeGreaterThan(0)

    wrapper.unmount()
  })

  it('pins the clicked file version through presets_pin', async () => {
    const pinSpy = vi.spyOn(ipcClient, 'presetsPin').mockResolvedValue(undefined)
    const preset = mockPresets[0]
    const item = preset.items[0]

    const wrapper = mount(PresetPinDrawer, {
      props: { open: true, preset, item },
      global: { plugins: [i18n] },
    })
    await flushPromises()

    const rows = document.body.querySelectorAll<HTMLElement>('[data-testid="pin-file-row"]')
    expect(rows.length).toBeGreaterThan(0)
    rows[0].click()
    await flushPromises()

    expect(pinSpy).toHaveBeenCalledTimes(1)
    const [presetId, platform, projectId, fileId, fileName] = pinSpy.mock.calls[0]
    expect(presetId).toBe(preset.id)
    expect(platform).toBe(item.platform)
    expect(projectId).toBe(item.projectId)
    expect(typeof fileId).toBe('string')
    expect(typeof fileName).toBe('string')

    wrapper.unmount()
  })

  it('sorts versions so 1.x.1 comes before 1.x in the version filter chips', async () => {
    vi.spyOn(ipcClient, 'compFiles').mockResolvedValue([
      {
        id: 'f1',
        projectId: 'sodium',
        platform: 'modrinth',
        name: 'f1',
        fileName: 'f1.jar',
        mcVersions: ['1.21', '1.20.1'],
        loaders: ['fabric'],
        channel: 'release',
        releaseDate: '2024-01-01',
        size: 100,
        hashes: {},
        downloadUrls: [],
        distributionDisabled: false,
        dependencies: [],
      },
      {
        id: 'f2',
        projectId: 'sodium',
        platform: 'modrinth',
        name: 'f2',
        fileName: 'f2.jar',
        mcVersions: ['1.21.1', '1.20.6'],
        loaders: ['fabric'],
        channel: 'release',
        releaseDate: '2024-02-01',
        size: 100,
        hashes: {},
        downloadUrls: [],
        distributionDisabled: false,
        dependencies: [],
      },
    ])

    const preset = { ...mockPresets[0], gameVersions: [], loaders: [] }
    const item = preset.items[0]

    const wrapper = mount(PresetPinDrawer, {
      props: { open: true, preset, item },
      global: { plugins: [i18n] },
    })
    await flushPromises()

    const text = document.body.textContent ?? ''
    const idx211 = text.indexOf('1.21.1')
    const idx21 = text.indexOf('1.21', idx211 + 6)
    expect(idx211).toBeGreaterThan(-1)
    expect(idx21).toBeGreaterThan(idx211)

    wrapper.unmount()
  })

  it('renders a report row for every status and the overlap badge', async () => {
    const wrapper = mount(ApplyReportDialog, {
      props: { open: true, report: mockApplyReport },
      global: { plugins: [i18n] },
    })
    await flushPromises()

    const text = document.body.textContent ?? ''
    expect(document.body.querySelectorAll('[data-testid="report-row"]').length).toBe(
      mockApplyReport.rows.length,
    )

    const statusLabels = [
      '已安装',
      '已替换',
      '已有相同资源',
      '已有不同版本资源',
      '没有对应的游戏版本',
      '没有对应的模组加载器',
      '没有可用文件',
      '数据包需要选择世界',
      '作者禁止第三方分发',
      '安装失败',
    ]
    for (const label of statusLabels) {
      expect(text).toContain(label)
    }

    // One overlapping row names both presets that share the resource.
    expect(text).toContain('与 常用优化模组、辅助工具 重叠')
    // A failing gate is surfaced as a warning banner.
    expect(text).toContain('预设「旧版本预设」不适用于此游戏版本，已跳过')

    wrapper.unmount()
  })

  it('disables apply while a datapack is selected and no world is chosen', async () => {
    const store = usePresetsStore()
    store.presets = [
      {
        id: 'preset-datapack',
        name: '地形数据包',
        description: '',
        gameVersions: ['1.20.1'],
        loaders: [],
        items: [
          {
            platform: 'curseforge',
            projectId: 'cf-datapack',
            kind: 'datapack',
            title: 'Terralith',
            slug: 'terralith',
            iconUrl: null,
            pinnedFileId: null,
            pinnedFileName: null,
          },
        ],
        createdAt: 1700000000000,
        updatedAt: 1700000000000,
      },
    ]

    const wrapper = mount(PresetApplyDialog, {
      props: { open: true, instanceId: 'inst-1' },
      global: { plugins: [i18n] },
    })
    await flushPromises()

    const row = document.body.querySelector<HTMLElement>('[data-testid="apply-preset-row"]')
    expect(row).not.toBeNull()
    row!.click()
    await flushPromises()

    // Selecting a datapack preset reveals the required world selector.
    expect(document.body.textContent).toContain('数据包目标世界')

    const confirmButton = bodyButtons().find((button) => button.textContent?.includes('开始应用'))
    expect(confirmButton).toBeDefined()
    expect(confirmButton!.disabled).toBe(true)

    wrapper.unmount()
  })
})

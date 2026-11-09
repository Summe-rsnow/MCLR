import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { mount, flushPromises } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import type { CompFile, ResourcePreset } from '../../../ipc/client'
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

function file(
  id: string,
  mcVersions: string[],
  loaders: string[] = ['fabric'],
  projectId = 'sodium',
): CompFile {
  return {
    id,
    projectId,
    platform: 'modrinth',
    name: id,
    fileName: `${id}.jar`,
    mcVersions,
    loaders,
    channel: 'release',
    releaseDate: '2024-01-01',
    size: 100,
    hashes: {},
    downloadUrls: [],
    distributionDisabled: false,
    dependencies: [],
  }
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
    // The version badge on the list row and the filter summary in the header come back.
    expect(text).toContain('游戏版本: 1.20.1 · 加载器: Fabric')

    wrapper.unmount()
  })

  it('saves the chosen game versions and loaders with the preset', async () => {
    const saveSpy = vi
      .spyOn(ipcClient, 'presetsSave')
      .mockImplementation(async (preset) => ({ ...mockPresets[0], ...preset }) as ResourcePreset)

    const wrapper = mount(PresetsPage, { global: { plugins: [i18n] } })
    await flushPromises()

    const settingsTab = wrapper
      .findAll('button')
      .find((button) => button.text().includes('预设设置'))
    expect(settingsTab).toBeDefined()
    await settingsTab!.trigger('click')
    await flushPromises()

    const versionChips = wrapper.findAll('[data-testid="version-chip"]')
    const version121 = versionChips.find((chip) => chip.text() === '1.21')
    expect(version121).toBeDefined()
    await version121!.trigger('click')

    const loaderChips = wrapper.findAll('[data-testid="loader-chip"]')
    const neoForge = loaderChips.find((chip) => chip.text() === 'NeoForge')
    expect(neoForge).toBeDefined()
    await neoForge!.trigger('click')

    const saveBtn = wrapper.find('[data-testid="save-preset-btn"]')
    await saveBtn.trigger('click')
    await flushPromises()

    expect(saveSpy).toHaveBeenCalledTimes(1)
    const saved = saveSpy.mock.calls[0][0]
    expect([...(saved.gameVersions ?? [])].sort()).toEqual(['1.20.1', '1.21'])
    expect([...(saved.loaders ?? [])].sort()).toEqual(['fabric', 'neoforge'])

    wrapper.unmount()
  })

  it('pins a file for the selected version and loader context', async () => {
    vi.spyOn(ipcClient, 'compFiles').mockResolvedValue([
      file('sodium-file-1', ['1.20.1']),
      file('sodium-file-2', ['1.20.4']),
    ])
    const pinRuleSpy = vi.spyOn(ipcClient, 'presetsPinRule').mockResolvedValue(undefined)

    const source = mockPresets[0]
    const item = source.items[0]
    const preset: ResourcePreset = { ...source, gameVersions: ['1.20.1', '1.20.4'] }

    const wrapper = mount(PresetPinDrawer, {
      props: { open: true, preset, item },
      global: { plugins: [i18n] },
    })
    await flushPromises()

    // The pin context defaults to the preset's first version/loader, so only that build shows.
    const initialRows = document.body.querySelectorAll<HTMLElement>('[data-testid="pin-file-row"]')
    expect(initialRows).toHaveLength(1)
    expect(initialRows[0].textContent).toContain('sodium-file-1.jar')

    const versionChips = document.body.querySelectorAll<HTMLElement>(
      '[data-testid="pin-context-versions"] button',
    )
    const v1204 = Array.from(versionChips).find((chip) => chip.textContent?.trim() === '1.20.4')
    expect(v1204).toBeDefined()
    v1204!.click()
    await flushPromises()

    const rows = document.body.querySelectorAll<HTMLElement>('[data-testid="pin-file-row"]')
    expect(rows).toHaveLength(1)
    rows[0].click()
    await flushPromises()

    expect(pinRuleSpy).toHaveBeenCalledTimes(1)
    expect(pinRuleSpy).toHaveBeenCalledWith(preset.id, item.platform, item.projectId, {
      gameVersions: ['1.20.4'],
      loaders: ['fabric'],
      fileId: 'sodium-file-2',
      fileName: 'sodium-file-2.jar',
    })

    wrapper.unmount()
  })

  it('removes the rule when clicking the file already pinned for that context', async () => {
    vi.spyOn(ipcClient, 'compFiles').mockResolvedValue([file('sodium-file-1', ['1.20.1'])])
    const pinRuleSpy = vi.spyOn(ipcClient, 'presetsPinRule').mockResolvedValue(undefined)
    const removeSpy = vi.spyOn(ipcClient, 'presetsRemovePinRule').mockResolvedValue(undefined)

    const source = mockPresets[0]
    const item = {
      ...source.items[0],
      pinnedFileId: 'sodium-file-1',
      pinnedFileName: 'sodium-file-1.jar',
      pinRules: [
        {
          gameVersions: ['1.20.1'],
          loaders: ['fabric'],
          fileId: 'sodium-file-1',
          fileName: 'sodium-file-1.jar',
        },
      ],
    }
    const preset: ResourcePreset = { ...source, items: [item] }

    const wrapper = mount(PresetPinDrawer, {
      props: { open: true, preset, item },
      global: { plugins: [i18n] },
    })
    await flushPromises()

    const row = document.body.querySelector<HTMLElement>('[data-testid="pin-file-row"]')
    expect(row).not.toBeNull()
    row!.click()
    await flushPromises()

    expect(removeSpy).toHaveBeenCalledTimes(1)
    expect(removeSpy).toHaveBeenCalledWith(
      preset.id,
      item.platform,
      item.projectId,
      'sodium-file-1',
    )
    expect(pinRuleSpy).not.toHaveBeenCalled()

    wrapper.unmount()
  })

  it('lists the configured rules, deletes one and clears every rule', async () => {
    vi.spyOn(ipcClient, 'compFiles').mockResolvedValue([file('sodium-file-1', ['1.20.1'])])
    const removeSpy = vi.spyOn(ipcClient, 'presetsRemovePinRule').mockResolvedValue(undefined)
    const pinSpy = vi.spyOn(ipcClient, 'presetsPin').mockResolvedValue(undefined)

    const source = mockPresets[0]
    // The fixture's Iris item carries one contextual rule.
    const item = source.items[1]
    const preset: ResourcePreset = { ...source, items: [source.items[0], item] }

    const wrapper = mount(PresetPinDrawer, {
      props: { open: true, preset, item },
      global: { plugins: [i18n] },
    })
    await flushPromises()

    const ruleRow = document.body.querySelector('[data-testid="pin-rule-row"]')
    expect(ruleRow).not.toBeNull()
    expect(ruleRow!.textContent).toContain('1.20.1')
    expect(ruleRow!.textContent).toContain('fabric')
    expect(ruleRow!.textContent).toContain('iris-mc1.20.1-1.7.0.jar')

    const deleteBtn = document.body.querySelector<HTMLElement>(
      '[data-testid="delete-rule-v-iris-1.7.0"]',
    )
    expect(deleteBtn).not.toBeNull()
    deleteBtn!.click()
    await flushPromises()

    expect(removeSpy).toHaveBeenCalledWith(preset.id, item.platform, item.projectId, 'v-iris-1.7.0')

    const followLatest = document.body.querySelector<HTMLButtonElement>(
      '[data-testid="follow-latest-btn"]',
    )
    expect(followLatest).not.toBeNull()
    followLatest!.click()
    await flushPromises()

    // "Follow latest" clears every rule in one call.
    expect(pinSpy).toHaveBeenCalledTimes(1)
    expect(pinSpy.mock.calls[0][3]).toBeNull()
    expect(pinSpy.mock.calls[0][4]).toBeNull()

    wrapper.unmount()
  })

  it('asks for a pin context before pinning anything', async () => {
    vi.spyOn(ipcClient, 'compFiles').mockResolvedValue([file('sodium-file-1', ['1.20.1'])])
    const pinRuleSpy = vi.spyOn(ipcClient, 'presetsPinRule').mockResolvedValue(undefined)

    const source = mockPresets[0]
    // No version/loader filters: the drawer opens with an empty pin context.
    const preset: ResourcePreset = { ...source, gameVersions: [], loaders: [] }
    const item = preset.items[0]

    const wrapper = mount(PresetPinDrawer, {
      props: { open: true, preset, item },
      global: { plugins: [i18n] },
    })
    await flushPromises()

    expect(document.body.querySelector('[data-testid="pin-context-hint"]')).not.toBeNull()

    const row = document.body.querySelector<HTMLElement>('[data-testid="pin-file-row"]')
    expect(row).not.toBeNull()
    row!.click()
    await flushPromises()

    // A wildcard pin would fight the "latest compatible file" default, so nothing is written.
    expect(pinRuleSpy).not.toHaveBeenCalled()

    wrapper.unmount()
  })

  it('sorts versions so 1.x.1 comes before 1.x in the pin context chips', async () => {
    vi.spyOn(ipcClient, 'compFiles').mockResolvedValue([
      file('f1', ['1.21', '1.20.1']),
      file('f2', ['1.21.1', '1.20.6']),
    ])

    const source = mockPresets[0]
    const preset: ResourcePreset = { ...source, gameVersions: [], loaders: [] }
    const item = preset.items[0]

    const wrapper = mount(PresetPinDrawer, {
      props: { open: true, preset, item },
      global: { plugins: [i18n] },
    })
    await flushPromises()

    const chips = Array.from(
      document.body.querySelectorAll<HTMLElement>('[data-testid="pin-context-versions"] button'),
    ).map((chip) => chip.textContent?.trim())
    const idx211 = chips.indexOf('1.21.1')
    const idx21 = chips.indexOf('1.21')
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
    // Each failing gate is surfaced with its own reason instead of one generic sentence.
    expect(text).toContain('预设「旧版本预设」不适用于此游戏版本，已跳过')
    expect(text).toContain('预设「锻造预设」不适用于当前模组加载器，已跳过')

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

    const confirmButton = Array.from(document.body.querySelectorAll('button')).find((button) =>
      button.textContent?.includes('开始应用'),
    )
    expect(confirmButton).toBeDefined()
    expect(confirmButton!.disabled).toBe(true)

    wrapper.unmount()
  })
})

import type {
  ResourcePreset,
  ApplyReport,
  StoreInfo,
  AddItemOutcome,
  PruneResult,
  CompPlatform,
  PresetItem,
  PresetPinRule,
} from '../../ipc/client'
import { mockTasks } from '../tasks/mock'
export const mockPresets: ResourcePreset[] = [
  {
    id: 'preset-1',
    name: '常用优化模组',
    description: '基础性能与渲染优化组合',
    gameVersions: ['1.20.1'],
    loaders: ['fabric'],
    items: [
      {
        platform: 'modrinth',
        projectId: 'AANobbMI',
        kind: 'mod',
        title: 'Sodium',
        slug: 'sodium',
        iconUrl: 'https://cdn.modrinth.com/data/AANobbMI/icon.png',
        pinnedFileId: null,
        pinnedFileName: null,
      },
      {
        platform: 'modrinth',
        projectId: 'YL57xq9U',
        kind: 'mod',
        title: 'Iris Shaders',
        slug: 'iris',
        iconUrl: 'https://cdn.modrinth.com/data/YL57xq9U/icon.png',
        pinnedFileId: 'v-iris-1.7.0',
        pinnedFileName: 'iris-mc1.20.1-1.7.0.jar',
        pinRules: [
          {
            gameVersions: ['1.20.1'],
            loaders: ['fabric'],
            fileId: 'v-iris-1.7.0',
            fileName: 'iris-mc1.20.1-1.7.0.jar',
          },
        ],
      },
    ],
    createdAt: 1700000000000,
    updatedAt: 1700001000000,
  },
  {
    id: 'preset-2',
    name: '视觉与材质包',
    description: '高清材质与精选光影',
    gameVersions: ['1.20.1'],
    loaders: [],
    items: [
      {
        platform: 'modrinth',
        projectId: '1KVoqItZ',
        kind: 'resourcepack',
        title: 'Faithful 32x',
        slug: 'faithful-32x',
        iconUrl: null,
        pinnedFileId: null,
        pinnedFileName: null,
      },
    ],
    createdAt: 1700002000000,
    updatedAt: 1700003000000,
  },
]

export const mockApplyReport: ApplyReport = {
  instanceId: 'inst-1',
  gates: [
    {
      presetId: 'preset-1',
      name: '常用优化模组',
      ok: true,
      reason: null,
    },
    {
      presetId: 'preset-old',
      name: '旧版本预设',
      ok: false,
      reason: 'gameVersion',
    },
    {
      presetId: 'preset-forge',
      name: '锻造预设',
      ok: false,
      reason: 'loader',
    },
  ],
  rows: [
    {
      platform: 'modrinth',
      projectId: 'AANobbMI',
      kind: 'mod',
      title: 'Sodium',
      zhTitle: '钠',
      status: 'installed',
      fileId: 'f-1',
      fileName: 'sodium-0.5.8.jar',
      installedFileName: null,
      linkMode: 'hardLink',
      fromPresets: ['常用优化模组'],
      overlap: false,
      versionConflict: false,
      viaDependencyOf: null,
      messageCode: null,
    },
    {
      platform: 'modrinth',
      projectId: 'YL57xq9U',
      kind: 'mod',
      title: 'Iris',
      zhTitle: '虹彩光影',
      status: 'replaced',
      fileId: 'f-2',
      fileName: 'iris-1.7.0.jar',
      installedFileName: 'iris-1.6.0.jar',
      linkMode: 'hardLink',
      fromPresets: ['常用优化模组'],
      overlap: true,
      versionConflict: true,
      viaDependencyOf: null,
      messageCode: null,
    },
    {
      platform: 'curseforge',
      projectId: '238222',
      kind: 'mod',
      title: 'JEI',
      zhTitle: '物品管理器',
      status: 'alreadySame',
      fileId: 'f-3',
      fileName: 'jei-1.20.1.jar',
      installedFileName: 'jei-1.20.1.jar',
      linkMode: null,
      fromPresets: ['常用优化模组', '辅助工具'],
      overlap: true,
      versionConflict: false,
      viaDependencyOf: null,
      messageCode: null,
    },
    {
      platform: 'modrinth',
      projectId: 'm-other',
      kind: 'mod',
      title: 'Cloth Config',
      zhTitle: '布料配置',
      status: 'alreadyOtherVersion',
      fileId: 'f-4',
      fileName: 'cloth-config-11.0.jar',
      installedFileName: 'cloth-config-10.0.jar',
      linkMode: null,
      fromPresets: ['常用优化模组'],
      overlap: false,
      versionConflict: false,
      viaDependencyOf: null,
      messageCode: null,
    },
    {
      platform: 'modrinth',
      projectId: 'm-nov',
      kind: 'mod',
      title: 'Old Mod',
      zhTitle: null,
      status: 'noGameVersion',
      fileId: null,
      fileName: null,
      installedFileName: null,
      linkMode: null,
      fromPresets: ['常用优化模组'],
      overlap: false,
      versionConflict: false,
      viaDependencyOf: null,
      messageCode: null,
    },
    {
      platform: 'curseforge',
      projectId: 'cf-noloader',
      kind: 'mod',
      title: 'Forge Only Mod',
      zhTitle: null,
      status: 'noLoader',
      fileId: null,
      fileName: null,
      installedFileName: null,
      linkMode: null,
      fromPresets: ['常用优化模组'],
      overlap: false,
      versionConflict: false,
      viaDependencyOf: null,
      messageCode: null,
    },
    {
      platform: 'modrinth',
      projectId: 'm-nofile',
      kind: 'mod',
      title: 'Empty Project',
      zhTitle: null,
      status: 'noFile',
      fileId: null,
      fileName: null,
      installedFileName: null,
      linkMode: null,
      fromPresets: ['常用优化模组'],
      overlap: false,
      versionConflict: false,
      viaDependencyOf: null,
      messageCode: null,
    },
    {
      platform: 'curseforge',
      projectId: 'cf-nodist',
      kind: 'mod',
      title: 'Forbidden Mod',
      zhTitle: null,
      status: 'distributionDisabled',
      fileId: null,
      fileName: null,
      installedFileName: null,
      linkMode: null,
      fromPresets: ['常用优化模组'],
      overlap: false,
      versionConflict: false,
      viaDependencyOf: null,
      messageCode: null,
    },
    {
      platform: 'modrinth',
      projectId: 'm-datapack',
      kind: 'datapack',
      title: 'Terralith Datapack',
      zhTitle: null,
      status: 'needsWorld',
      fileId: null,
      fileName: null,
      installedFileName: null,
      linkMode: null,
      fromPresets: ['常用优化模组'],
      overlap: false,
      versionConflict: false,
      viaDependencyOf: null,
      messageCode: null,
    },
    {
      platform: 'modrinth',
      projectId: 'm-fail',
      kind: 'mod',
      title: 'Corrupted Mod',
      zhTitle: null,
      status: 'failed',
      fileId: null,
      fileName: null,
      installedFileName: null,
      linkMode: null,
      fromPresets: ['常用优化模组'],
      overlap: false,
      versionConflict: false,
      viaDependencyOf: null,
      messageCode: 'net.connectFailed',
    },
  ],
}

export const mockStoreInfo: StoreInfo = {
  dir: 'C:\\mclr\\cache\\store',
  defaultDir: 'C:\\mclr\\cache\\store',
  objectCount: 3,
  totalBytes: 52428800,
  savedBytes: 31457280,
  entries: [
    {
      sha1: 'a1b2c3d4e5f60123456789abcdef0123456789ab',
      platform: 'modrinth',
      projectId: 'AANobbMI',
      fileId: 'f-1',
      fileName: 'sodium-fabric-0.5.8+mc1.20.1.jar',
      kind: 'mod',
      size: 1048576,
      addedAt: 1700000000000,
      liveRefs: 2,
      pinned: true,
    },
    {
      sha1: 'b2c3d4e5f60123456789abcdef0123456789abc1',
      platform: 'modrinth',
      projectId: 'YL57xq9U',
      fileId: 'f-2',
      fileName: 'iris-mc1.20.1-1.7.0.jar',
      kind: 'mod',
      size: 2097152,
      addedAt: 1700001000000,
      liveRefs: 1,
      pinned: false,
    },
    {
      sha1: 'c3d4e5f60123456789abcdef0123456789abc1d2',
      platform: 'curseforge',
      projectId: '238222',
      fileId: 'f-3',
      fileName: 'jei-1.20.1-15.3.0.4.jar',
      kind: 'mod',
      size: 3145728,
      addedAt: 1700002000000,
      liveRefs: 0,
      pinned: false,
    },
  ],
}

let presets = [...mockPresets]

const handlers = {
  // Real IPC serialises fresh objects on every call; returning live objects here would hide
  // reactivity bugs because mutated items keep their identity.
  presets_list: (): ResourcePreset[] => structuredClone(presets),
  presets_save: (args: Record<string, unknown>): ResourcePreset => {
    const p = (args.preset || {}) as Partial<ResourcePreset>
    const existingIdx = presets.findIndex((item) => item.id === p.id)
    const now = Date.now()
    if (existingIdx >= 0) {
      const updated: ResourcePreset = {
        ...presets[existingIdx],
        ...p,
        updatedAt: now,
      } as ResourcePreset
      presets[existingIdx] = updated
      return updated
    }
    const created: ResourcePreset = {
      id: p.id || `preset-${now}`,
      name: p.name || 'New Preset',
      description: p.description || '',
      gameVersions: p.gameVersions || [],
      loaders: p.loaders || [],
      items: p.items || [],
      createdAt: now,
      updatedAt: now,
    }
    presets.push(created)
    return created
  },
  presets_delete: (args: Record<string, unknown>): void => {
    const id = String(args.id || '')
    presets = presets.filter((p) => p.id !== id)
  },
  presets_add_items: (args: Record<string, unknown>): AddItemOutcome[] => {
    const items = (args.items || []) as PresetItem[]
    const preset = presets.find((p) => p.id === args.presetId)
    return items.map((it) => {
      const outcome: AddItemOutcome = {
        platform: it.platform,
        projectId: it.projectId,
        title: it.title,
        status: 'added',
        latestFileName: 'latest.jar',
      }
      if (!preset) return { ...outcome, status: 'unsupported' }
      const existing = preset.items.find(
        (x) => x.platform === it.platform && x.projectId === it.projectId,
      )
      if (!existing) {
        preset.items.push({ ...it, pinRules: it.pinRules ?? [] })
        return outcome
      }
      let changed = false
      for (const rule of it.pinRules ?? []) {
        const rules = (existing.pinRules ??= [])
        const same = rules.findIndex(
          (r) =>
            r.gameVersions.join(',') === rule.gameVersions.join(',') &&
            r.loaders.join(',') === rule.loaders.join(','),
        )
        if (same >= 0) {
          if (rules[same].fileId !== rule.fileId) changed = true
          rules[same] = rule
        } else {
          rules.push(rule)
          changed = true
        }
      }
      if (existing.pinRules?.length) {
        existing.pinnedFileId = existing.pinRules[0].fileId
        existing.pinnedFileName = existing.pinRules[0].fileName
      }
      return { ...outcome, status: changed ? 'alreadyInPresetOtherVersion' : 'alreadyInPreset' }
    })
  },
  presets_remove_item: (args: Record<string, unknown>): void => {
    const { presetId, platform, projectId } = args
    const p = presets.find((x) => x.id === presetId)
    if (p) {
      p.items = p.items.filter((it) => !(it.platform === platform && it.projectId === projectId))
    }
  },
  presets_pin: (args: Record<string, unknown>): void => {
    const { presetId, platform, projectId, fileId, fileName } = args
    const p = presets.find((x) => x.id === presetId)
    if (p) {
      const it = p.items.find((item) => item.platform === platform && item.projectId === projectId)
      if (it) {
        it.pinnedFileId = (fileId as string) || null
        it.pinnedFileName = (fileName as string) || null
        // Passing no file means "follow latest", which drops every contextual rule too.
        if (!fileId) it.pinRules = []
      }
    }
  },
  presets_pin_rule: (args: Record<string, unknown>): void => {
    const { presetId, platform, projectId, rule } = args as {
      presetId: string
      platform: CompPlatform
      projectId: string
      rule: PresetPinRule
    }
    const p = presets.find((x) => x.id === presetId)
    const it = p?.items.find((item) => item.platform === platform && item.projectId === projectId)
    if (!it) return
    const rules = (it.pinRules ??= [])
    const existing = rules.find(
      (r) =>
        r.gameVersions.join(',') === rule.gameVersions.join(',') &&
        r.loaders.join(',') === rule.loaders.join(','),
    )
    if (existing) {
      existing.fileId = rule.fileId
      existing.fileName = rule.fileName
    } else {
      rules.push(rule)
    }
    const first = rules[0]
    it.pinnedFileId = first.fileId
    it.pinnedFileName = first.fileName
  },
  presets_remove_pin_rule: (args: Record<string, unknown>): void => {
    const { presetId, platform, projectId, fileId } = args as {
      presetId: string
      platform: CompPlatform
      projectId: string
      fileId: string
    }
    const p = presets.find((x) => x.id === presetId)
    const it = p?.items.find((item) => item.platform === platform && item.projectId === projectId)
    if (!it?.pinRules) return
    it.pinRules = it.pinRules.filter((rule) => rule.fileId !== fileId)
    const first = it.pinRules[0]
    it.pinnedFileId = first?.fileId ?? null
    it.pinnedFileName = first?.fileName ?? null
  },
  presets_apply: (): string => {
    const id = 'task-preset-apply-1'
    if (!mockTasks.some((t) => t.id === id)) {
      mockTasks.push({
        id,
        kind: 'presets.apply',
        titleKey: 'tasks.titles.presetApply',
        titleParams: {},
        state: 'success',
        progress: 1.0,
        speedBps: 0,
        steps: [],
        attempt: 0,
        maxAttempts: 1,
        resumable: true,
        queueSeq: 100,
        createdAtMs: Date.now(),
      })
    }
    return id
  },
  presets_apply_report: (): ApplyReport => ({ ...mockApplyReport }),
  presets_cache: (): string => {
    const id = 'task-preset-cache-1'
    if (!mockTasks.some((t) => t.id === id)) {
      mockTasks.push({
        id,
        kind: 'presets.cache',
        titleKey: 'tasks.titles.presetCache',
        titleParams: {},
        state: 'success',
        progress: 1.0,
        speedBps: 0,
        steps: [],
        attempt: 0,
        maxAttempts: 1,
        resumable: true,
        queueSeq: 101,
        createdAtMs: Date.now(),
      })
    }
    return id
  },
  presets_store_info: (): StoreInfo => ({ ...mockStoreInfo }),
  presets_store_set_dir: (args: Record<string, unknown>): StoreInfo => ({
    ...mockStoreInfo,
    dir: String(args.dir || mockStoreInfo.dir),
  }),
  presets_store_prune: (): PruneResult => ({
    removedCount: 1,
    freedBytes: 3145728,
  }),
  presets_store_remove: (): void => {},
}

export default handlers

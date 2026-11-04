import type {
  ResourcePreset,
  PresetPinRule,
  ApplyReport,
  StoreInfo,
  AddItemOutcome,
  PruneResult,
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
  presets_list: (): ResourcePreset[] => [...presets],
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
    const items = (args.items || []) as AddItemOutcome[]
    return items.map((it) => ({
      platform: it.platform,
      projectId: it.projectId,
      title: it.title,
      status: 'added',
      latestFileName: 'latest.jar',
    }))
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
      }
    }
  },
  presets_pin_rule: (args: Record<string, unknown>): void => {
    const { presetId, platform, projectId, rule } = args
    const p = presets.find((x) => x.id === presetId)
    if (p) {
      const it = p.items.find((item) => item.platform === platform && item.projectId === projectId)
      if (it) {
        if (!it.pinRules) it.pinRules = []
        const r = rule as PresetPinRule
        it.pinRules = it.pinRules.filter((x) => x.fileId !== r.fileId)
        it.pinRules.push(r)
        it.pinnedFileId = r.fileId
        it.pinnedFileName = r.fileName
      }
    }
  },
  presets_remove_pin_rule: (args: Record<string, unknown>): void => {
    const { presetId, platform, projectId, fileId } = args
    const p = presets.find((x) => x.id === presetId)
    if (p) {
      const it = p.items.find((item) => item.platform === platform && item.projectId === projectId)
      if (it && it.pinRules) {
        it.pinRules = it.pinRules.filter((r) => r.fileId !== fileId)
        if (it.pinRules.length > 0) {
          it.pinnedFileId = it.pinRules[0].fileId
          it.pinnedFileName = it.pinRules[0].fileName
        } else {
          it.pinnedFileId = null
          it.pinnedFileName = null
        }
      }
    }
  },
  presets_apply: (): string => {
    const id = 'task-preset-apply-1'
    if (!mockTasks.some((t) => t.id === id)) {
      mockTasks.push({
        id,
        parentId: null,
        titleKey: 'tasks.titles.presetApply',
        titleParams: {},
        state: 'success',
        progress: 1.0,
        speedBps: 0,
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
        parentId: null,
        titleKey: 'tasks.titles.presetCache',
        titleParams: {},
        state: 'success',
        progress: 1.0,
        speedBps: 0,
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

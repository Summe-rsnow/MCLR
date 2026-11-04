import { invokeCommand } from '../client'
import type { CompPlatform, CompType } from './comp'

export type OtherVersionPolicy = 'skip' | 'replace'

export interface PresetPinRule {
  gameVersions: string[]
  loaders: string[]
  fileId: string
  fileName: string
}

export interface PresetItem {
  platform: CompPlatform
  projectId: string
  kind: CompType
  title: string
  slug?: string | null
  iconUrl?: string | null
  pinnedFileId?: string | null
  pinnedFileName?: string | null
  pinRules?: PresetPinRule[]
}
export interface ResourcePreset {
  id: string
  name: string
  description: string
  gameVersions: string[]
  loaders: string[]
  items: PresetItem[]
  createdAt: number
  updatedAt: number
}

export interface ApplyRequest {
  instanceId: string
  presetIds: string[]
  datapackWorld?: string | null
  includeDependencies: boolean
  onOtherVersion: OtherVersionPolicy
}

export type LinkMode = 'hardLink' | 'copy' | 'extracted'

export type ApplyStatus =
  | 'installed'
  | 'replaced'
  | 'alreadySame'
  | 'alreadyOtherVersion'
  | 'noGameVersion'
  | 'noLoader'
  | 'noFile'
  | 'distributionDisabled'
  | 'needsWorld'
  | 'failed'

export interface ApplyRow {
  platform: CompPlatform
  projectId: string
  kind: CompType
  title: string
  zhTitle?: string | null
  status: ApplyStatus
  fileId?: string | null
  fileName?: string | null
  installedFileName?: string | null
  linkMode?: LinkMode | null
  fromPresets: string[]
  overlap: boolean
  versionConflict: boolean
  viaDependencyOf?: string | null
  messageCode?: string | null
}

export type GateReason = 'gameVersion'

export interface PresetGate {
  presetId: string
  name: string
  ok: boolean
  reason?: GateReason | null
}

export interface ApplyReport {
  instanceId: string
  gates: PresetGate[]
  rows: ApplyRow[]
}

export type AddStatus =
  | 'added'
  | 'alreadyInPreset'
  | 'alreadyInPresetOtherVersion'
  | 'noGameVersion'
  | 'noLoader'
  | 'noFile'
  | 'unsupported'

export interface AddItemOutcome {
  platform: CompPlatform
  projectId: string
  title: string
  status: AddStatus
  latestFileName?: string | null
}

export interface StoreEntry {
  sha1: string
  platform: CompPlatform
  projectId: string
  fileId: string
  fileName: string
  kind: CompType
  size: number
  addedAt: number
  liveRefs: number
  pinned: boolean
}

export interface StoreInfo {
  dir: string
  defaultDir: string
  objectCount: number
  totalBytes: number
  savedBytes: number
  entries: StoreEntry[]
}

export interface PruneResult {
  removedCount: number
  freedBytes: number
}

export const presetsIpc = {
  presetsList: (): Promise<ResourcePreset[]> => invokeCommand<ResourcePreset[]>('presets_list'),
  presetsSave: (preset: Partial<ResourcePreset> & { name: string }): Promise<ResourcePreset> =>
    invokeCommand<ResourcePreset>('presets_save', { preset }),
  presetsDelete: (id: string): Promise<void> => invokeCommand<void>('presets_delete', { id }),
  presetsAddItems: (presetId: string, items: PresetItem[]): Promise<AddItemOutcome[]> =>
    invokeCommand<AddItemOutcome[]>('presets_add_items', { presetId, items }),
  presetsRemoveItem: (presetId: string, platform: CompPlatform, projectId: string): Promise<void> =>
    invokeCommand<void>('presets_remove_item', { presetId, platform, projectId }),
  presetsPin: (
    presetId: string,
    platform: CompPlatform,
    projectId: string,
    fileId?: string | null,
    fileName?: string | null,
  ): Promise<void> =>
    invokeCommand<void>('presets_pin', { presetId, platform, projectId, fileId, fileName }),
  presetsPinRule: (
    presetId: string,
    platform: CompPlatform,
    projectId: string,
    rule: PresetPinRule,
  ): Promise<void> =>
    invokeCommand<void>('presets_pin_rule', { presetId, platform, projectId, rule }),
  presetsRemovePinRule: (
    presetId: string,
    platform: CompPlatform,
    projectId: string,
    fileId: string,
  ): Promise<void> =>
    invokeCommand<void>('presets_remove_pin_rule', { presetId, platform, projectId, fileId }),
  presetsApply: (request: ApplyRequest): Promise<string> =>
    invokeCommand<string>('presets_apply', { request }),
  presetsApplyReport: (taskId: string): Promise<ApplyReport> =>
    invokeCommand<ApplyReport>('presets_apply_report', { taskId }),
  presetsCache: (presetId: string): Promise<string> =>
    invokeCommand<string>('presets_cache', { presetId }),
  presetsStoreInfo: (): Promise<StoreInfo> => invokeCommand<StoreInfo>('presets_store_info'),
  presetsStoreSetDir: (dir: string, migrate: boolean): Promise<StoreInfo> =>
    invokeCommand<StoreInfo>('presets_store_set_dir', { dir, migrate }),
  presetsStorePrune: (): Promise<PruneResult> => invokeCommand<PruneResult>('presets_store_prune'),
  presetsStoreRemove: (sha1: string): Promise<void> =>
    invokeCommand<void>('presets_store_remove', { sha1 }),
}

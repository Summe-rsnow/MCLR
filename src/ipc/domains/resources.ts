import { invokeCommand } from '../client'
import type { CompFile, CompType } from '../bindings'
import type { CompPlatform, CompProject } from './comp'

export type ResourceKind = 'mod' | 'resourcePack' | 'shader' | 'schematic'
export type ResourceState = 'ok' | 'unavailable'
export type OverwritePolicy = 'ask' | 'overwrite' | 'skip'

export interface ResourceEntry {
  path: string
  relPath: string
  fileName: string
  isDir: boolean
  size: number
  createdMs?: number | null
  modifiedMs?: number | null
  enabled: boolean
  state: ResourceState
  title?: string | null
  zhName?: string | null
  description?: string | null
  version?: string | null
  icon?: string | null
  tags: string[]
  modId?: string | null
  projectId?: string | null
  latestFile?: CompFile | null
  updateAvailable: boolean
  dependents: string[]
}

export interface ResourcePatch {
  fileName: string
  title?: string | null
  zhName?: string | null
  description?: string | null
  version?: string | null
  tags?: string[] | null
  modId?: string | null
  projectId?: string | null
  latestFile?: CompFile | null
  updateAvailable?: boolean | null
  dependents?: string[] | null
  state?: ResourceState | null
  icon?: string | null
}

export interface ResourcesPatchEvent {
  requestId: string
  instanceId: string
  patches: ResourcePatch[]
}

export interface LocalModsScanResult {
  entries: ResourceEntry[]
}

export interface LocalModUpdateItem {
  currentFileName: string
  newFile: CompFile
}

export interface SchematicInfo {
  name: string
  author?: string | null
  size: [number, number, number]
  totalVolume: number
  totalBlocks: number
  regionCount: number
  createdMs?: number | null
  modifiedMs?: number | null
}

export interface ImportResourcesResult {
  imported: string[]
  conflicts: string[]
}

export interface CompPickTargetResult {
  instanceId?: string | null
  dir?: string | null
}

export type ModDepKind = 'required' | 'optional' | 'incompatible'

export interface ModDep {
  id: string
  range?: string | null
  kind: ModDepKind
}

export type DepStatus =
  | 'installed'
  | 'disabled'
  | 'bundled'
  | 'versionMismatch'
  | 'missing'
  | 'conflict'

export interface DepItem {
  id: string
  kind: ModDepKind
  range?: string | null
  status: DepStatus
  providerFile?: string | null
  providerVersion?: string | null
}

export interface ModDependencyReport {
  fileName: string
  modId: string
  deps: DepItem[]
  dependents: string[]
}

export const resourcesIpc = {
  resourcesList: (
    instanceId: string,
    kind: ResourceKind,
    rel?: string | null,
    requestId?: string | null,
  ): Promise<ResourceEntry[]> =>
    invokeCommand<ResourceEntry[]>('resources_list', {
      instanceId,
      kind,
      rel: rel ?? null,
      requestId: requestId ?? null,
    }),

  localModsScanStart: (instanceId: string, requestId: string): Promise<LocalModsScanResult> =>
    invokeCommand<LocalModsScanResult>('local_mods_scan_start', {
      instanceId,
      requestId,
    }),

  resourceToggle: (instanceId: string, path: string): Promise<string> =>
    invokeCommand<string>('resource_toggle', { instanceId, path }),

  resourcesDelete: (paths: string[], permanent: boolean): Promise<void> =>
    invokeCommand<void>('resources_delete', { paths, permanent }),

  resourcesImport: (
    instanceId: string,
    kind: ResourceKind,
    sources: string[],
    policy: OverwritePolicy,
  ): Promise<ImportResourcesResult> =>
    invokeCommand<ImportResourcesResult>('resources_import', {
      instanceId,
      kind,
      sources,
      policy,
    }),

  resourcesExport: (entries: ResourceEntry[], format: 'txt' | 'csv'): Promise<string> =>
    invokeCommand<string>('resources_export', { entries, format }),

  schematicInspect: (path: string): Promise<SchematicInfo> =>
    invokeCommand<SchematicInfo>('schematic_inspect', { path }),

  localModsDependents: (instanceId: string, fileName: string): Promise<string[]> =>
    invokeCommand<string[]>('local_mods_dependents', { instanceId, fileName }),

  localModsUpdateMany: (instanceId: string, updates: LocalModUpdateItem[]): Promise<string> =>
    invokeCommand<string>('local_mods_update_many', { instanceId, updates }),

  compPickTarget: (
    projectType: CompType,
    preferredInstanceId?: string | null,
    mcVersions?: string[],
    loaders?: string[],
  ): Promise<CompPickTargetResult> =>
    invokeCommand<CompPickTargetResult>('comp_pick_target', {
      projectType,
      preferredInstanceId: preferredInstanceId ?? null,
      mcVersions: mcVersions ?? [],
      loaders: loaders ?? [],
    }),

  localModsDependencies: (instanceId: string): Promise<ModDependencyReport[]> =>
    invokeCommand<ModDependencyReport[]>('local_mods_dependencies', { instanceId }),

  compDepLookup: (modId: string): Promise<CompProject | null> =>
    invokeCommand<CompProject | null>('comp_dep_lookup', { modId }),

  compDepProjects: (platform: CompPlatform, projectIds: string[]): Promise<CompProject[]> =>
    invokeCommand<CompProject[]>('comp_dep_projects', { platform, projectIds }),
}

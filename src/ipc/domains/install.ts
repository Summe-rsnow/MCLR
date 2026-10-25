// Slice-owned IPC types and client methods (install).
import { invokeCommand } from '../client'
import type { CompFile } from '../bindings'

export type LoaderKind =
  | 'fabric'
  | 'legacyFabric'
  | 'liteLoader'
  | 'labyMod'
  | 'optiFine'
  | 'forge'
  | 'neoForge'
  | 'cleanroom'

export interface LoaderVersionEntry {
  loader: LoaderKind
  mcVersion: string
  version: string
  stable: boolean
  releaseTime: string | null
}

export interface VersionManifestEntry {
  id: string
  type: string
  url: string
  releaseTime: string
}

export interface VersionManifestList {
  latestRelease: string
  latestSnapshot: string
  versions: VersionManifestEntry[]
}

export interface LabyModSel {
  channel: string
  commitRef: string
  baseVersion: string
}

export interface ExtraMod {
  fileName: string
  urls: string[]
  sha1: string | null
  size: number | null
}

export type InstallMode =
  | { type: 'create' }
  | { type: 'modify'; removeModFiles: string[] }

export interface InstallRequest {
  mcVersion: string
  instanceName: string
  forge?: string | null
  neoforge?: string | null
  cleanroom?: string | null
  fabric?: string | null
  legacyFabric?: string | null
  labymod?: LabyModSel | null
  liteloader?: string | null
  optifine?: string | null
  extraMods: ExtraMod[]
  mode: InstallMode
}

export interface ExtraCandidates {
  fabricApi: CompFile[]
  legacyFabricApi: CompFile[]
  optiFabric: CompFile[]
}

export interface CurrentExtras {
  fabricApi?: string | null
  legacyFabricApi?: string | null
  optifabric?: string | null
}

export interface CurrentInstallInfo {
  loaders: string[]
  extras: CurrentExtras
}

export const installIpc = {
  installCreate: (req: InstallRequest): Promise<string> =>
    invokeCommand<string>('install_create', { req }),
  installExtras: (
    mcVersion: string,
    fabric: boolean,
    legacyFabric: boolean,
    optifabric: boolean,
  ): Promise<ExtraCandidates> =>
    invokeCommand<ExtraCandidates>('install_extras', {
      mcVersion,
      fabric,
      legacyFabric,
      optifabric,
    }),
  installCurrentInfo: (instanceId: string): Promise<CurrentInstallInfo> =>
    invokeCommand<CurrentInstallInfo>('install_current_info', { instanceId }),
  installValidateName: (name: string): Promise<void> =>
    invokeCommand<void>('install_validate_name', { name }),
}

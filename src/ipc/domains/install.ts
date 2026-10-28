// Slice-owned IPC types and client methods (install).
import { invokeCommand } from '../client'
import type {
  ExtraCandidates,
  ExtraMod,
  CurrentExtras,
  CurrentInstallInfo,
  LabyModSel,
  InstallRequest as BindingsInstallRequest,
} from '../bindings'

export type { ExtraCandidates, ExtraMod, CurrentExtras, CurrentInstallInfo, LabyModSel }

export type InstallMode =
  | { type: 'create' }
  // Rust field name is snake_case: the enum's `rename_all` only renames variants.
  | { type: 'modify'; remove_mod_files?: string[] }

export type InstallRequest = Omit<BindingsInstallRequest, 'mode'> & {
  mode: InstallMode
}

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
  versionType: string
  type?: string
  url: string
  releaseTime: string
}

export interface VersionManifestList {
  latestRelease: string
  latestSnapshot: string
  versions: VersionManifestEntry[]
}

export interface VersionCompatInfo {
  isAprilFools: boolean
  foolCode?: string | null
  supportedLoaders: string[]
  incompatiblePairs: [string, string][]
  recommendedJava: number
}

export const installIpc = {
  installManifest: (forceRefresh?: boolean): Promise<VersionManifestList> =>
    invokeCommand<VersionManifestList>('install_manifest', { forceRefresh: !!forceRefresh }),
  installVanilla: (version: string, name: string): Promise<string> =>
    invokeCommand<string>('install_vanilla', { version, name }),
  installRepair: (instanceId: string): Promise<string> =>
    invokeCommand<string>('install_repair', { instanceId }),
  loadersVersions: (loader: LoaderKind, mcVersion: string): Promise<LoaderVersionEntry[]> =>
    invokeCommand<LoaderVersionEntry[]>('loaders_versions', { loader, mcVersion }),
  loadersInstall: (
    mcVersion: string,
    instanceName: string,
    loader: LoaderKind,
    loaderVersion: string,
    optifineVersion?: string | null,
  ): Promise<string> =>
    invokeCommand<string>('loaders_install', {
      mcVersion,
      instanceName,
      loader,
      loaderVersion,
      optifineVersion: optifineVersion ?? null,
    }),
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
  installVersionCompat: (
    mcVersion: string,
    releaseType: string,
    releaseTime: string,
  ): Promise<VersionCompatInfo> =>
    invokeCommand<VersionCompatInfo>('install_version_compat', {
      mcVersion,
      releaseType,
      releaseTime,
    }),
}

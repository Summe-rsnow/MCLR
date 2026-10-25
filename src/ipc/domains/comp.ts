import { invokeCommand } from '../client'

export type CompPlatform = 'modrinth' | 'curseforge'

export type CompType = 'mod' | 'modpack' | 'resourcepack' | 'shader' | 'datapack' | 'world'

export type CompReleaseChannel = 'release' | 'beta' | 'alpha'

export type CompDependencyKind = 'required' | 'optional' | 'incompatible' | 'embedded'

export interface CompDependency {
  projectId: string
  kind: CompDependencyKind
}

export interface CompFileHashes {
  sha1?: string | null
  sha512?: string | null
  murmur2?: number | null
}

export interface CompFile {
  id: string
  projectId: string
  platform: CompPlatform
  name: string
  fileName: string
  mcVersions: string[]
  loaders: string[]
  channel: CompReleaseChannel
  releaseDate: string
  size: number
  hashes: CompFileHashes
  downloadUrls: string[]
  distributionDisabled: boolean
  dependencies: CompDependency[]
}

export interface CompProject {
  platform: CompPlatform
  id: string
  slug: string
  title: string
  summary: string
  description?: string | null
  iconUrl?: string | null
  author: string
  downloads: number
  follows: number
  categories: string[]
  mcVersions: string[]
  loaders: string[]
  compType: CompType
  updatedAt: string
  websiteUrl?: string | null
}

export type CompSort = 'relevance' | 'downloads' | 'follows' | 'newest' | 'updated'

export interface CompSearchQuery {
  platform: CompPlatform | 'all'
  compType: CompType
  query: string
  mcVersion?: string | null
  loader?: string | null
  category?: string | null
  sort: CompSort
  offset: number
  limit: number
}

export interface CompSearchResult {
  hits: CompProject[]
  totalHits: number
  offset: number
  limit: number
}

export interface CompFavoriteEntry {
  platform: string
  projectId: string
  slug: string
  title: string
  iconUrl?: string | null
  compType: string
}

export interface CompSettings {
  favorites: CompFavoriteEntry[]
}

export type ModpackFormat =
  | 'curseforge'
  | 'modrinth'
  | 'prism'
  | 'mcbbs'
  | 'hmcl'
  | 'nested'
  | 'rawMinecraft'

export interface ModpackInspectResult {
  format: ModpackFormat
  name: string
  version: string
  author?: string | null
  mcVersion: string
  loader?: string | null
  loaderVersion?: string | null
  fileCount: number
}

export interface LocalModInfo {
  fileName: string
  path: string
  enabled: boolean
  modId: string
  name: string
  version: string
  description?: string | null
  authors: string[]
  loader: string
  sha1: string
  murmur2: number
  size: number
  projectId?: string | null
  updateFile?: CompFile | null
}

export interface JarInJarNode {
  id: string
  name: string
  version: string
  path: string
  size: number
  children: JarInJarNode[]
}

export interface ResolvedDependencyItem {
  project: CompProject
  file: CompFile
  installed: boolean
}

export interface ExportCandidateEntry {
  relPath: string
  isDir: boolean
  size: number
  defaultSelected: boolean
}

export type ModpackExportFormat = 'modrinth' | 'curseforge'

export interface ModpackExportOptions {
  instanceId: string
  format: ModpackExportFormat
  packName: string
  packVersion: string
  author: string
  summary: string
  selectedPaths: string[]
  outputPath: string
}

export const compIpc = {
  compSearch: (query: CompSearchQuery): Promise<CompSearchResult> =>
    invokeCommand<CompSearchResult>('comp_search', { query }),
  compProject: (platform: CompPlatform, projectId: string): Promise<CompProject> =>
    invokeCommand<CompProject>('comp_project', { platform, projectId }),
  compFiles: (
    platform: CompPlatform,
    projectId: string,
    mcVersion?: string | null,
    loader?: string | null,
  ): Promise<CompFile[]> =>
    invokeCommand<CompFile[]>('comp_files', {
      platform,
      projectId,
      mcVersion: mcVersion ?? null,
      loader: loader ?? null,
    }),
  compDownload: (file: CompFile, targetInstance: string, kind: CompType): Promise<string> =>
    invokeCommand<string>('comp_download', { file, targetInstance, kind }),
  modpackInspect: (archivePath: string): Promise<ModpackInspectResult> =>
    invokeCommand<ModpackInspectResult>('modpack_inspect', { archivePath }),
  modpackImportFile: (archivePath: string, instanceName: string): Promise<string> =>
    invokeCommand<string>('modpack_import_file', { archivePath, instanceName }),
  modpackInstallRemote: (file: CompFile, instanceName: string): Promise<string> =>
    invokeCommand<string>('modpack_install_remote', { file, instanceName }),
  localModsList: (instanceId: string): Promise<LocalModInfo[]> =>
    invokeCommand<LocalModInfo[]>('local_mods_list', { instanceId }),
  localModToggle: (instanceId: string, fileName: string, enabled: boolean): Promise<LocalModInfo> =>
    invokeCommand<LocalModInfo>('local_mod_toggle', { instanceId, fileName, enabled }),
  localModDelete: (instanceId: string, fileName: string): Promise<void> =>
    invokeCommand<void>('local_mod_delete', { instanceId, fileName }),
  localModsCheckUpdates: (
    instanceId: string,
    mcVersion: string,
    loader: string,
  ): Promise<LocalModInfo[]> =>
    invokeCommand<LocalModInfo[]>('local_mods_check_updates', {
      instanceId,
      mcVersion,
      loader,
    }),
  localModUpdate: (
    instanceId: string,
    currentFileName: string,
    newFile: CompFile,
  ): Promise<string> =>
    invokeCommand<string>('local_mod_update', {
      instanceId,
      currentFileName,
      newFile,
    }),
  localModJarInJar: (instanceId: string, fileName: string): Promise<JarInJarNode[]> =>
    invokeCommand<JarInJarNode[]>('local_mod_jar_in_jar', { instanceId, fileName }),
  compResolveDependencies: (
    file: CompFile,
    instanceId: string,
    mcVersion: string,
    loader: string,
  ): Promise<ResolvedDependencyItem[]> =>
    invokeCommand<ResolvedDependencyItem[]>('comp_resolve_dependencies', {
      file,
      instanceId,
      mcVersion,
      loader,
    }),
  exportScanInstance: (instanceId: string): Promise<ExportCandidateEntry[]> =>
    invokeCommand<ExportCandidateEntry[]>('export_scan_instance', { instanceId }),
  exportModpack: (options: ModpackExportOptions): Promise<string> =>
    invokeCommand<string>('export_modpack', { options }),
}

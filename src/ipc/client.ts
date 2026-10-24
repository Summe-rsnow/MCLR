import { commands, type AppError, type AppInfo, type Result } from './bindings'
import { invoke as TAURI_INVOKE } from '@tauri-apps/api/core'

export class AppErrorEx extends Error {
  readonly code: string
  readonly params: Record<string, string>
  readonly detail?: string

  constructor(err: AppError) {
    super(err.detail ? `${err.code}: ${err.detail}` : err.code)
    this.name = 'AppErrorEx'
    this.code = err.code
    const cleanParams: Record<string, string> = {}
    if (err.params) {
      for (const [k, v] of Object.entries(err.params)) {
        if (typeof v === 'string') cleanParams[k] = v
      }
    }
    this.params = cleanParams
    this.detail = err.detail ?? undefined
  }
}

export async function unwrapResult<T>(promise: Promise<Result<T, AppError>>): Promise<T> {
  const res = await promise
  if (res.status === 'ok') {
    return res.data
  }
  throw new AppErrorEx(res.error)
}

export interface SecretsStatus {
  msClientId: boolean
  curseforgeApiKey: boolean
  updateEndpoint: boolean
  updatePubkey: boolean
}

export type TaskState = 'waiting' | 'running' | 'paused' | 'success' | 'canceled' | 'failed'

export interface TaskNode {
  id: string
  parentId?: string | null
  titleKey: string
  titleParams: Record<string, string>
  state: TaskState
  progress: number
  speedBps: number
  error?: AppError | null
}

export type InstanceKind =
  | 'vanilla'
  | 'snapshot'
  | 'oldBeta'
  | 'oldAlpha'
  | 'fabric'
  | 'legacyFabric'
  | 'forge'
  | 'neoforge'
  | 'cleanroom'
  | 'optiFine'
  | 'liteLoader'
  | 'labyMod'
  | 'quilt'
  | 'moddedMultiple'

export type InstanceState = 'ok' | 'broken' | 'incomplete'

export interface InstanceConfig {
  icon: string
  category: string
  pinned: boolean
  isolation: string
  java?: string | null
  memory?: number | null
  jvmArgs?: string | null
  gameArgs?: string | null
  windowTitle?: string | null
  preLaunchCommand?: string | null
  loginRequirement: string
  serverQuickPlay?: string | null
}

export interface InstanceInfo {
  id: string
  name: string
  folder: string
  path: string
  mcVersion: string
  kind: InstanceKind
  loaders: string[]
  state: InstanceState
  config: InstanceConfig
}

export interface JavaInstallation {
  path: string
  home: string
  version: string
  major: number
  implementor: string
  arch: string
  isJdk: boolean
  source: string
}

export interface JavaConstraint {
  minMajor?: number | null
  maxMajor?: number | null
  exactMajors: number[]
}

export interface GameFolderEntry {
  name: string
  path: string
}

export interface GameSettings {
  folders: GameFolderEntry[]
  currentFolder: string
  defaultIsolation: string
  sortBy: string
}

export interface LaunchSettings {
  ramMode: 'auto' | 'custom'
  ramCustomMb: number
  jvmArgs: string
  gameArgs: string
  windowTitle: string
  fullscreen: boolean
  width: number
  height: number
  preferredIpStack: 'auto' | 'ipv4' | 'ipv6'
  processPriority: 'normal' | 'aboveNormal' | 'high'
  closeLauncherOnStart: 'keep' | 'hide' | 'exit'
  disableCrashAnalysis: boolean
}

export interface JavaSettings {
  custom: string[]
  select: string
  preferJdk: boolean
}

export interface Settings {
  schemaVersion: number
  game?: GameSettings
  launch?: LaunchSettings
  java?: JavaSettings
  ui: {
    seed: string
    mode: 'light' | 'dark' | 'system'
    material: 'flat' | 'glass'
    scale: number
    fontFamily: string
    backgroundMode: string
    backgroundPath: string
    backgroundBlur: number
    backgroundOpacity: number
    musicEnabled: boolean
    musicFolder: string
    musicVolume: number
  }
  language: {
    locale: string
    formatCulture: string
  }
  system: {
    logLevel: string
    logRetentionDays: number
    disableGpu: boolean
  }
  net?: NetSettings
  comp?: CompSettings
  update?: UpdateSettings
  states?: StatesSettings
}

export interface UpdateSettings {
  channel: string
  autoCheck: boolean
  announcementUrl: string
}
export interface StatesSettings {
  dismissedAnnouncements: string[]
}

export type ProxyMode = 'none' | 'system' | 'custom'
export type SourcePriority = 'mirrorFirst' | 'smart' | 'officialFirst'

export interface NetSettings {
  proxyMode: ProxyMode
  proxyUrl: string
  doh: boolean
  sourcePriority: SourcePriority
  connectionsPerFile: number
  maxConcurrentFiles: number
}

export interface UpdateCheckResult {
  available: boolean
  currentVersion: string
  version?: string | null
  date?: string | null
  body?: string | null
  downloadUrl?: string | null
  signature?: string | null
}

export interface LauncherLogFile {
  fileName: string
  path: string
  size: number
  modifiedAt: string
}

export interface NewsItem {
  id: string
  title: string
  summary: string
  date: string
  category: string
  imageUrl?: string | null
  articleUrl: string
}

export interface AnnouncementButton {
  text: string
  url?: string | null
}

export interface AnnouncementItem {
  id: string
  title: string
  detail: string
  btn1?: AnnouncementButton | null
  btn2?: AnnouncementButton | null
}

export interface SkinFetchResult {
  username: string
  uuid: string
  skinUrl: string
  skinDataUrl: string
  headDataUrl: string
  slim: boolean
}

export interface CacheSizeInfo {
  cacheBytes: number
  logsBytes: number
  skinCacheBytes: number
  totalBytes: number
}

export interface DailyLuckResult {
  date: string
  username: string
  score: number
  levelKey: string
}

export type LogLevel = 'info' | 'warn' | 'error' | 'debug' | 'fatal'

export interface LogLine {
  lineId: number
  processId: number
  instanceId: string
  level: LogLevel
  text: string
  timestamp: string
}

export interface LaunchProcessInfo {
  processId: number
  instanceId: string
  accountName: string
  startedAt: string
  running: boolean
  exitCode?: number | null
}

export interface LaunchStartResult {
  taskId: string
  processId?: number | null
}

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
  platform: CompPlatform
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

export interface SaveSpawn {
  x: number
  y: number
  z: number
}

export interface SaveInfo {
  folderName: string
  path: string
  name: string
  seed: string
  spawn: SaveSpawn
  gameMode: number
  difficulty: number
  playTime: number
  lastPlayed: number
  dataVersion?: number | null
  iconDataUrl?: string | null
}

export interface DatapackInfo {
  id: string
  fileName: string
  description: string
  packFormat: number
  enabled: boolean
  builtIn: boolean
}

export interface ScreenshotInfo {
  fileName: string
  path: string
  size: number
  modifiedAt: string
  dataUrl?: string | null
}

export interface InstanceFileItem {
  fileName: string
  path: string
  enabled: boolean
  size: number
  modifiedAt: string
}

export interface ServerEntry {
  index: number
  name: string
  ip: string
  icon?: string | null
  acceptTextures?: number | null
}

export interface ServerPingResult {
  ip: string
  resolvedAddr: string
  online: boolean
  motd: string
  versionName: string
  protocol: number
  playersOnline: number
  playersMax: number
  favicon?: string | null
  latencyMs: number
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

export type CrashConfidence = 'high' | 'medium' | 'low'

export interface CrashCauseFinding {
  code: string
  params: Record<string, string>
  confidence: CrashConfidence
  evidence: string[]
}

export interface CrashResult {
  instanceId: string
  analyzedAt: string
  causes: CrashCauseFinding[]
  summaryLogLines: string[]
  reportFiles: string[]
}
export interface NewsItem {
  id: string
  title: string
  summary: string
  date: string
  category: string
  imageUrl?: string | null
  articleUrl: string
}

export interface AnnouncementButton {
  text: string
  url?: string | null
}

export interface AnnouncementItem {
  id: string
  title: string
  detail: string
  btn1?: AnnouncementButton | null
  btn2?: AnnouncementButton | null
}




export type AccountKind = 'offline' | 'microsoft' | 'authlib' | 'yggdrasilConnect'

export interface CapeView {
  id: string
  alias: string
  url: string
  active: boolean
}

export interface AccountView {
  id: string
  kind: AccountKind
  username: string
  uuid: string
  selected: boolean
  server?: string | null
  serverName?: string | null
  skinHeadDataUrl?: string | null
  capes: CapeView[]
  defaultSkinModel: string
}

export interface DeviceCodeEvent {
  userCode: string
  verificationUri: string
  expiresIn: number
}

export interface AuthlibProfileView {
  id: string
  name: string
}

export interface AuthlibLoginResponse {
  account?: AccountView | null
  pendingSessionId?: string | null
  profiles: AuthlibProfileView[]
}

export interface VersionManifestEntry {
  id: string
  versionType: string
  url: string
  releaseTime: string
}

export interface VersionManifestList {
  latestRelease: string
  latestSnapshot: string
  versions: VersionManifestEntry[]
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
  releaseTime?: string | null
}

async function invokeCommand<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await TAURI_INVOKE<T>(cmd, args)
  } catch (err: unknown) {
    if (err && typeof err === 'object' && 'code' in err) {
      throw new AppErrorEx(err as AppError)
    }
    throw err
  }
}

export const ipcClient = {
  appInfo: (): Promise<AppInfo> => unwrapResult(commands.appInfo()),
  secretsStatus: (): Promise<SecretsStatus> => invokeCommand<SecretsStatus>('secrets_status'),
  windowSetMaterial: (glass: boolean): Promise<void> =>
    invokeCommand<void>('window_set_material', { glass }),
  settingsGet: (): Promise<Settings> => invokeCommand<Settings>('settings_get'),
  settingsPatch: (patch: Record<string, unknown>): Promise<Settings> =>
    invokeCommand<Settings>('settings_patch', { patch }),
  tasksList: (): Promise<TaskNode[]> => invokeCommand<TaskNode[]>('tasks_list'),
  taskCancel: (id: string): Promise<void> => invokeCommand<void>('task_cancel', { id }),
  taskPause: (id: string): Promise<void> => invokeCommand<void>('task_pause', { id }),
  taskResume: (id: string): Promise<void> => invokeCommand<void>('task_resume', { id }),
  instancesList: (): Promise<InstanceInfo[]> => invokeCommand<InstanceInfo[]>('instances_list'),
  instanceRename: (id: string, newName: string): Promise<InstanceInfo> =>
    invokeCommand<InstanceInfo>('instance_rename', { id, newName }),
  instanceDuplicate: (id: string, newName: string): Promise<InstanceInfo> =>
    invokeCommand<InstanceInfo>('instance_duplicate', { id, newName }),
  instanceDelete: (id: string): Promise<void> => invokeCommand<void>('instance_delete', { id }),
  instanceUpdateConfig: (id: string, config: InstanceConfig): Promise<InstanceInfo> =>
    invokeCommand<InstanceInfo>('instance_update_config', { id, config }),
  instanceOpenFolder: (id: string): Promise<void> =>
    invokeCommand<void>('instance_open_folder', { id }),
  javaScan: (): Promise<JavaInstallation[]> => invokeCommand<JavaInstallation[]>('java_scan'),
  javaDownload: (component: string): Promise<string> =>
    invokeCommand<string>('java_download', { component }),
  // Accounts
  accountsList: (): Promise<AccountView[]> => invokeCommand<AccountView[]>('accounts_list'),
  accountsSelect: (id: string): Promise<AccountView[]> =>
    invokeCommand<AccountView[]>('accounts_select', { id }),
  accountsRemove: (id: string): Promise<AccountView[]> =>
    invokeCommand<AccountView[]>('accounts_remove', { id }),
  accountsAddOffline: (
    username: string,
    uuidMode: string,
    customUuid?: string,
  ): Promise<AccountView> =>
    invokeCommand<AccountView>('accounts_add_offline', {
      username,
      uuidMode,
      customUuid,
    }),
  accountsAddMicrosoft: (): Promise<string> =>
    invokeCommand<string>('accounts_add_microsoft'),
  accountsAddAuthlib: (
    server: string,
    username: string,
    password: string,
  ): Promise<AuthlibLoginResponse> =>
    invokeCommand<AuthlibLoginResponse>('accounts_add_authlib', {
      server,
      username,
      password,
    }),
  accountsSelectProfile: (
    pendingSessionId: string,
    profileId: string,
  ): Promise<AccountView> =>
    invokeCommand<AccountView>('accounts_select_profile', {
      pendingSessionId,
      profileId,
    }),
  accountsRefresh: (id: string): Promise<AccountView> =>
    invokeCommand<AccountView>('accounts_refresh', { id }),
  accountsSkinUpload: (
    id: string,
    variant: string,
    pngPath: string,
  ): Promise<AccountView> =>
    invokeCommand<AccountView>('accounts_skin_upload', {
      id,
      variant,
      pngPath,
    }),
  accountsCapeSet: (id: string, capeId: string | null): Promise<AccountView> =>
    invokeCommand<AccountView>('accounts_cape_set', { id, capeId }),

  // Install
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
  // Launch & logs
  launchInstance: (instanceId: string, accountId: string): Promise<LaunchStartResult> =>
    invokeCommand<LaunchStartResult>('launch_instance', { instanceId, accountId }),
  launchLogs: (instanceId?: string): Promise<LogLine[]> =>
    invokeCommand<LogLine[]>('launch_logs', { instanceId }),
  launchProcesses: (): Promise<LaunchProcessInfo[]> =>
    invokeCommand<LaunchProcessInfo[]>('launch_processes'),
  launchKill: (processId: number): Promise<void> =>
    invokeCommand<void>('launch_kill', { processId }),

  // Comp
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
  // Modpack
  modpackInspect: (archivePath: string): Promise<ModpackInspectResult> =>
    invokeCommand<ModpackInspectResult>('modpack_inspect', { archivePath }),
  modpackImportFile: (archivePath: string, instanceName: string): Promise<string> =>
    invokeCommand<string>('modpack_import_file', { archivePath, instanceName }),
  modpackInstallRemote: (file: CompFile, instanceName: string): Promise<string> =>
    invokeCommand<string>('modpack_install_remote', { file, instanceName }),

  // Local Mods & JarInJar & Dependency Resolver
  localModsList: (instanceId: string): Promise<LocalModInfo[]> =>
    invokeCommand<LocalModInfo[]>('local_mods_list', { instanceId }),
  localModToggle: (
    instanceId: string,
    fileName: string,
    enabled: boolean,
  ): Promise<LocalModInfo> =>
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
  // Saves & Datapacks
  savesList: (instanceId: string): Promise<SaveInfo[]> =>
    invokeCommand<SaveInfo[]>('saves_list', { instanceId }),
  saveDelete: (instanceId: string, folderName: string): Promise<void> =>
    invokeCommand<void>('save_delete', { instanceId, folderName }),
  saveBackup: (instanceId: string, folderName: string): Promise<string> =>
    invokeCommand<string>('save_backup', { instanceId, folderName }),
  saveExport: (instanceId: string, folderName: string, destZipPath: string): Promise<void> =>
    invokeCommand<void>('save_export', { instanceId, folderName, destZipPath }),
  saveImport: (instanceId: string, zipPath: string): Promise<SaveInfo> =>
    invokeCommand<SaveInfo>('save_import', { instanceId, zipPath }),
  saveDatapacksList: (instanceId: string, folderName: string): Promise<DatapackInfo[]> =>
    invokeCommand<DatapackInfo[]>('save_datapacks_list', { instanceId, folderName }),
  saveDatapackToggle: (
    instanceId: string,
    folderName: string,
    packId: string,
    enabled: boolean,
  ): Promise<DatapackInfo[]> =>
    invokeCommand<DatapackInfo[]>('save_datapack_toggle', {
      instanceId,
      folderName,
      packId,
      enabled,
    }),
  // Screenshots & Instance Files
  screenshotsList: (instanceId: string): Promise<ScreenshotInfo[]> =>
    invokeCommand<ScreenshotInfo[]>('screenshots_list', { instanceId }),
  screenshotDelete: (instanceId: string, fileName: string): Promise<void> =>
    invokeCommand<void>('screenshot_delete', { instanceId, fileName }),
  instanceFilesList: (instanceId: string, folderKind: string): Promise<InstanceFileItem[]> =>
    invokeCommand<InstanceFileItem[]>('instance_files_list', { instanceId, folderKind }),
  instanceFileToggle: (
    instanceId: string,
    folderKind: string,
    fileName: string,
    enabled: boolean,
  ): Promise<InstanceFileItem> =>
    invokeCommand<InstanceFileItem>('instance_file_toggle', {
      instanceId,
      folderKind,
      fileName,
      enabled,
    }),
  instanceFileDelete: (
    instanceId: string,
    folderKind: string,
    fileName: string,
  ): Promise<void> =>
    invokeCommand<void>('instance_file_delete', { instanceId, folderKind, fileName }),
  // Servers
  serversList: (instanceId: string): Promise<ServerEntry[]> =>
    invokeCommand<ServerEntry[]>('servers_list', { instanceId }),
  serverAdd: (
    instanceId: string,
    name: string,
    ip: string,
    acceptTextures?: number | null,
  ): Promise<ServerEntry[]> =>
    invokeCommand<ServerEntry[]>('server_add', {
      instanceId,
      name,
      ip,
      acceptTextures: acceptTextures ?? null,
    }),
  serverUpdate: (
    instanceId: string,
    index: number,
    name: string,
    ip: string,
    acceptTextures?: number | null,
  ): Promise<ServerEntry[]> =>
    invokeCommand<ServerEntry[]>('server_update', {
      instanceId,
      index,
      name,
      ip,
      acceptTextures: acceptTextures ?? null,
    }),
  serverRemove: (instanceId: string, index: number): Promise<ServerEntry[]> =>
    invokeCommand<ServerEntry[]>('server_remove', { instanceId, index }),
  serverPing: (ip: string): Promise<ServerPingResult> =>
    invokeCommand<ServerPingResult>('server_ping', { ip }),
  exportScanInstance: (instanceId: string): Promise<ExportCandidateEntry[]> =>
    invokeCommand<ExportCandidateEntry[]>('export_scan_instance', { instanceId }),
  exportModpack: (options: ModpackExportOptions): Promise<string> =>
    invokeCommand<string>('export_modpack', { options }),
  crashAnalyze: (
    instanceId: string,
    customLogPath?: string | null,
  ): Promise<CrashResult> =>
    invokeCommand<CrashResult>('crash_analyze', {
      instanceId,
      customLogPath: customLogPath ?? null,
    }),
  crashExportReport: (
    instanceId: string,
    destZipPath: string,
  ): Promise<string> =>
    invokeCommand<string>('crash_export_report', { instanceId, destZipPath }),
  // Home
  newsFetch: (page?: number, forceRefresh?: boolean): Promise<NewsItem[]> =>
    invokeCommand<NewsItem[]>('news_fetch', {
      page: page ?? 0,
      forceRefresh: forceRefresh ?? false,
    }),
  announcementsFetch: (): Promise<AnnouncementItem[]> =>
    invokeCommand<AnnouncementItem[]>('announcements_fetch'),
  // Tools
  toolsFetchSkin: (username: string): Promise<SkinFetchResult> =>
    invokeCommand<SkinFetchResult>('tools_fetch_skin', { username }),
  toolsCacheInfo: (): Promise<CacheSizeInfo> =>
    invokeCommand<CacheSizeInfo>('tools_cache_info'),
  toolsCacheClear: (
    clearCache: boolean,
    clearLogs: boolean,
    clearSkins: boolean,
  ): Promise<CacheSizeInfo> =>
    invokeCommand<CacheSizeInfo>('tools_cache_clear', {
      clearCache,
      clearLogs,
      clearSkins,
    }),
  toolsCustomDownload: (url: string, destPath: string): Promise<string> =>
    invokeCommand<string>('tools_custom_download', { url, destPath }),
  toolsCreateShortcut: (
    instanceId: string,
    shortcutName?: string | null,
  ): Promise<string> =>
    invokeCommand<string>('tools_create_shortcut', {
      instanceId,
      shortcutName: shortcutName ?? null,
    }),
  toolsDailyLuck: (username: string): Promise<DailyLuckResult> =>
    invokeCommand<DailyLuckResult>('tools_daily_luck', { username }),
  toolsListMediaFiles: (folderPath: string, kind: string): Promise<string[]> =>
    invokeCommand<string[]>('tools_list_media_files', { folderPath, kind }),
  // Update & Launcher Logs
  updateCheck: (channelOverride?: string | null): Promise<UpdateCheckResult> =>
    invokeCommand<UpdateCheckResult>('update_check', {
      channelOverride: channelOverride ?? null,
    }),
  updateInstall: (channelOverride?: string | null): Promise<string> =>
    invokeCommand<string>('update_install', {
      channelOverride: channelOverride ?? null,
    }),
  launcherLogsList: (): Promise<LauncherLogFile[]> =>
    invokeCommand<LauncherLogFile[]>('launcher_logs_list'),
  launcherLogRead: (fileName: string): Promise<string> =>
    invokeCommand<string>('launcher_log_read', { fileName }),
}

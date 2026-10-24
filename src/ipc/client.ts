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

export interface Settings {
  schemaVersion: number
  game?: GameSettings
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
}

import { invoke as TAURI_INVOKE } from '@tauri-apps/api/core'
import { commands, type AppError, type AppInfo, type Result } from './bindings'
import { accountsIpc } from './domains/accounts'
import { compIpc } from './domains/comp'
import { crashIpc } from './domains/crash'
import { homeIpc } from './domains/home'
import { installIpc } from './domains/install'
import { instancesIpc } from './domains/instances'
import { launchIpc } from './domains/launch'
import { mediaIpc } from './domains/media'
import { resourcesIpc } from './domains/resources'
import { savesIpc } from './domains/saves'
import { serversIpc } from './domains/servers'
import { settingsIpc } from './domains/settings'
import { tasksIpc } from './domains/tasks'
import { toolsIpc } from './domains/tools'
import { updateIpc } from './domains/update'

export * from './domains/accounts'
export * from './domains/comp'
export * from './domains/crash'
export * from './domains/home'
export * from './domains/install'
export * from './domains/instances'
export * from './domains/launch'
export * from './domains/media'
export * from './domains/resources'
export * from './domains/saves'
export * from './domains/servers'
export * from './domains/settings'
export * from './domains/tasks'
export * from './domains/tools'
export * from './domains/update'

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

export async function invokeCommand<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await TAURI_INVOKE<T>(cmd, args)
  } catch (err: unknown) {
    if (err && typeof err === 'object' && 'code' in err) {
      throw new AppErrorEx(err as AppError)
    }
    throw err
  }
}

export interface SecretsStatus {
  msClientId: boolean
  curseforgeApiKey: boolean
  updateEndpoint: boolean
  updatePubkey: boolean
}

export const ipcClient = {
  appInfo: (): Promise<AppInfo> => unwrapResult(commands.appInfo()),
  secretsStatus: (): Promise<SecretsStatus> => invokeCommand<SecretsStatus>('secrets_status'),
  ...accountsIpc,
  ...compIpc,
  ...crashIpc,
  ...homeIpc,
  ...installIpc,
  ...instancesIpc,
  ...launchIpc,
  ...mediaIpc,
  ...resourcesIpc,
  ...savesIpc,
  ...serversIpc,
  ...settingsIpc,
  ...tasksIpc,
  ...toolsIpc,
  ...updateIpc,
}

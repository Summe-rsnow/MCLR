import { invokeCommand } from '../client'

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

export const updateIpc = {
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

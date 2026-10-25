import { invokeCommand } from '../client'

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

export const launchIpc = {
  launchInstance: (instanceId: string, accountId: string): Promise<LaunchStartResult> =>
    invokeCommand<LaunchStartResult>('launch_instance', { instanceId, accountId }),
  launchLogs: (instanceId?: string): Promise<LogLine[]> =>
    invokeCommand<LogLine[]>('launch_logs', { instanceId }),
  launchProcesses: (): Promise<LaunchProcessInfo[]> =>
    invokeCommand<LaunchProcessInfo[]>('launch_processes'),
  launchKill: (processId: number): Promise<void> =>
    invokeCommand<void>('launch_kill', { processId }),
}

import type { LaunchProcessInfo, LaunchStartResult, LogLine } from '../../ipc/client'

export const mockLogLines: LogLine[] = [
  {
    lineId: 1,
    processId: 1001,
    instanceId: '1.20.1-vanilla',
    level: 'info',
    text: '[12:00:00] [main/INFO]: Loading Minecraft 1.20.1 with MCLR Launcher',
    timestamp: '2026-10-24T12:00:00.000Z',
  },
  {
    lineId: 2,
    processId: 1001,
    instanceId: '1.20.1-vanilla',
    level: 'info',
    text: '[12:00:01] [main/INFO]: Setting user: StevePro',
    timestamp: '2026-10-24T12:00:01.000Z',
  },
  {
    lineId: 3,
    processId: 1001,
    instanceId: '1.20.1-vanilla',
    level: 'warn',
    text: '[12:00:02] [Render thread/WARN]: Missing sound event: minecraft:item.goat_horn.play',
    timestamp: '2026-10-24T12:00:02.000Z',
  },
  {
    lineId: 4,
    processId: 1001,
    instanceId: '1.20.1-vanilla',
    level: 'debug',
    text: '[12:00:03] [Render thread/DEBUG]: Initializing GLFW 3.3.8 on Windows x64',
    timestamp: '2026-10-24T12:00:03.000Z',
  },
  {
    lineId: 5,
    processId: 1001,
    instanceId: '1.20.1-vanilla',
    level: 'info',
    text: '[12:00:04] [Render thread/INFO]: OpenGL 4.6.0 NVIDIA 551.76 initialized',
    timestamp: '2026-10-24T12:00:04.000Z',
  },
]

export const mockRunningProcesses: LaunchProcessInfo[] = [
  {
    processId: 1001,
    instanceId: '1.20.1-vanilla',
    accountName: 'StevePro',
    startedAt: '2026-10-24T12:00:00.000Z',
    running: true,
    exitCode: null,
  },
]

let logsStorage: LogLine[] = [...mockLogLines]
let processesStorage: LaunchProcessInfo[] = [...mockRunningProcesses]

export function resetMockLaunchData(): void {
  logsStorage = [...mockLogLines]
  processesStorage = [...mockRunningProcesses]
}

const handlers = {
  launch_instance: (args: Record<string, unknown>): LaunchStartResult => {
    const instanceId = String(args.instanceId || '1.20.1-vanilla')
    const accountId = String(args.accountId || 'acc-ms-1')

    if (!accountId) {
      throw { code: 'launch.accountRequired', params: {} }
    }

    const pid = 1000 + Math.floor(Math.random() * 9000)
    const proc: LaunchProcessInfo = {
      processId: pid,
      instanceId,
      accountName: 'StevePro',
      startedAt: new Date().toISOString(),
      running: true,
      exitCode: null,
    }
    processesStorage.push(proc)

    const newLine: LogLine = {
      lineId: logsStorage.length + 1,
      processId: pid,
      instanceId,
      level: 'info',
      text: `[${new Date().toLocaleTimeString()}] [main/INFO]: Started Minecraft instance ${instanceId} (PID: ${pid})`,
      timestamp: new Date().toISOString(),
    }
    logsStorage.push(newLine)

    return {
      taskId: `task-launch-${instanceId}-${pid}`,
      processId: pid,
    }
  },

  launch_logs: (args: Record<string, unknown>): LogLine[] => {
    const instanceId = args.instanceId ? String(args.instanceId) : undefined
    if (instanceId) {
      return logsStorage.filter((l) => l.instanceId === instanceId)
    }
    return [...logsStorage]
  },

  launch_processes: (): LaunchProcessInfo[] => {
    return [...processesStorage]
  },

  launch_kill: (args: Record<string, unknown>): null => {
    const processId = Number(args.processId)
    const proc = processesStorage.find((p) => p.processId === processId)
    if (!proc) {
      throw { code: 'launch.processNotFound', params: { pid: String(processId) } }
    }
    proc.running = false
    proc.exitCode = -1

    logsStorage.push({
      lineId: logsStorage.length + 1,
      processId,
      instanceId: proc.instanceId,
      level: 'warn',
      text: `[${new Date().toLocaleTimeString()}] [MCLR]: Process ${processId} terminated by user.`,
      timestamp: new Date().toISOString(),
    })
    return null
  },
}

export default handlers

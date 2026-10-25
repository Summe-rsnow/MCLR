import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import {
  ipcClient,
  type LogLine,
  type LogLevel,
  type LaunchProcessInfo,
} from '../../ipc/client'

export const useLogsStore = defineStore('logs', () => {
  const logs = ref<LogLine[]>([])
  const runningProcesses = ref<LaunchProcessInfo[]>([])
  const selectedInstanceId = ref<string | null>(null)
  const selectedProcessId = ref<number | null>(null)
  const levelFilter = ref<LogLevel | 'all'>('all')
  const searchQuery = ref('')
  const autoScroll = ref(true)
  const loading = ref(false)

  let unlistenLog: UnlistenFn | null = null
  let unlistenBatch: UnlistenFn | null = null
  let unlistenStarted: UnlistenFn | null = null
  let unlistenExit: UnlistenFn | null = null

  const pendingQueue: LogLine[] = []
  let rafHandle: number | null = null

  function flushLogs() {
    if (pendingQueue.length > 0) {
      logs.value.push(...pendingQueue)
      pendingQueue.length = 0
      if (logs.value.length > 10000) {
        logs.value.splice(0, logs.value.length - 10000)
      }
    }
    rafHandle = null
  }

  function enqueueLogs(lines: LogLine[]) {
    pendingQueue.push(...lines)
    if (rafHandle === null) {
      if (typeof requestAnimationFrame !== 'undefined') {
        rafHandle = requestAnimationFrame(flushLogs)
      } else {
        flushLogs()
      }
    }
  }

  const filteredLogs = computed(() => {
    let result = logs.value

    if (selectedProcessId.value != null) {
      result = result.filter((l) => l.processId === selectedProcessId.value)
    } else if (selectedInstanceId.value) {
      result = result.filter((l) => l.instanceId === selectedInstanceId.value)
    }

    if (levelFilter.value !== 'all') {
      result = result.filter((l) => l.level.toLowerCase() === levelFilter.value.toLowerCase())
    }

    const query = searchQuery.value.trim().toLowerCase()
    if (query) {
      result = result.filter((l) => l.text.toLowerCase().includes(query))
    }

    return result
  })

  const activeProcess = computed(() => {
    if (selectedProcessId.value != null) {
      return (
        runningProcesses.value.find(
          (p) => p.processId === selectedProcessId.value && p.running,
        ) || null
      )
    }
    return runningProcesses.value.find((p) => p.running) || null
  })

  async function startListening() {
    if (unlistenLog) return

    try {
      unlistenLog = await listen<LogLine>('launch://log', (event) => {
        enqueueLogs([event.payload])
      })

      unlistenBatch = await listen<{ instanceId: string; lines: LogLine[] }>(
        'launch://logBatch',
        (event) => {
          enqueueLogs(event.payload.lines)
        },
      )

      unlistenStarted = await listen<LaunchProcessInfo>('launch://started', (event) => {
        const proc = event.payload
        const idx = runningProcesses.value.findIndex((p) => p.processId === proc.processId)
        if (idx >= 0) {
          runningProcesses.value[idx] = proc
        } else {
          runningProcesses.value.unshift(proc)
        }
      })

      unlistenExit = await listen<{ processId: number; instanceId: string; exitCode: number }>(
        'launch://exit',
        (event) => {
          const { processId, exitCode } = event.payload
          const proc = runningProcesses.value.find((p) => p.processId === processId)
          if (proc) {
            proc.running = false
            proc.exitCode = exitCode
          }
        },
      )
    } catch {
      // Mock / Vitest environment fallback
    }
  }

  function stopListening() {
    if (unlistenLog) {
      unlistenLog()
      unlistenLog = null
    }
    if (unlistenBatch) {
      unlistenBatch()
      unlistenBatch = null
    }
    if (rafHandle !== null && typeof cancelAnimationFrame !== 'undefined') {
      cancelAnimationFrame(rafHandle)
      rafHandle = null
    }
    pendingQueue.length = 0
    if (unlistenStarted) {
      unlistenStarted()
      unlistenStarted = null
    }
    if (unlistenExit) {
      unlistenExit()
      unlistenExit = null
    }
  }

  async function fetchLogs(instanceId?: string): Promise<void> {
    loading.value = true
    try {
      logs.value = await ipcClient.launchLogs(instanceId)
      runningProcesses.value = await ipcClient.launchProcesses()
    } finally {
      loading.value = false
    }
  }

  async function killRunningProcess(processId?: number): Promise<void> {
    const pid = processId ?? activeProcess.value?.processId
    if (pid == null) return
    await ipcClient.launchKill(pid)
    runningProcesses.value = runningProcesses.value.map((p) =>
      p.processId === pid ? { ...p, running: false, exitCode: -1 } : p,
    )
  }

  function clearLogs(): void {
    if (rafHandle !== null && typeof cancelAnimationFrame !== 'undefined') {
      cancelAnimationFrame(rafHandle)
      rafHandle = null
    }
    pendingQueue.length = 0
    logs.value = []
  }

  async function copyLogsToClipboard(): Promise<void> {
    const text = filteredLogs.value.map((l) => l.text).join('\n')
    await navigator.clipboard.writeText(text)
  }

  function exportLogs(): string {
    return filteredLogs.value.map((l) => l.text).join('\n')
  }

  return {
    logs,
    runningProcesses,
    selectedInstanceId,
    selectedProcessId,
    levelFilter,
    searchQuery,
    autoScroll,
    loading,
    filteredLogs,
    activeProcess,
    startListening,
    stopListening,
    fetchLogs,
    killRunningProcess,
    clearLogs,
    copyLogsToClipboard,
    exportLogs,
  }
})

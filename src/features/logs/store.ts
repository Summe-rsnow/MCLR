import { defineStore } from 'pinia'
import { ref, shallowRef, computed, watch } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { ipcClient, type LogLine, type LogLevel, type LaunchProcessInfo } from '../../ipc/client'

const LOG_CAP = 10000

/** A log line as kept by the store: carries a monotonic `seq` for incremental filtering and trimming. */
export type SeqLogLine = LogLine & { seq?: number }

export type LogFilterKey = {
  processId: number | null
  instanceId: string | null
  level: LogLevel | 'all'
  query: string
}

/** Filter rules: processId wins over instanceId, level ignores case, query is a lowercase containment match. */
export function matchesFilter(line: LogLine, filter: LogFilterKey): boolean {
  if (filter.processId != null) {
    if (line.processId !== filter.processId) return false
  } else if (filter.instanceId) {
    if (line.instanceId !== filter.instanceId) return false
  }

  if (filter.level !== 'all') {
    if (line.level.toLowerCase() !== filter.level.toLowerCase()) return false
  }

  if (filter.query && !line.text.toLowerCase().includes(filter.query)) return false

  return true
}

export const useLogsStore = defineStore('logs', () => {
  // shallowRef: log lines are no longer individually proxied; a flush swaps the array reference
  const logs = shallowRef<SeqLogLine[]>([])
  const filteredLogs = shallowRef<SeqLogLine[]>([])
  /** Incremented by every flush that actually appended lines, so the terminal can follow the flush rhythm. */
  const flushTick = ref(0)
  const runningProcesses = ref<LaunchProcessInfo[]>([])
  const selectedInstanceId = ref<string | null>(null)
  const selectedProcessId = ref<number | null>(null)
  const levelFilter = ref<LogLevel | 'all'>('all')
  const searchQuery = ref('')
  const autoScroll = ref(true)
  const loading = ref(false)

  let unlistenBatch: UnlistenFn | null = null
  let unlistenStarted: UnlistenFn | null = null
  let unlistenExit: UnlistenFn | null = null

  const pendingQueue: SeqLogLine[] = []
  let rafHandle: number | null = null
  let nextSeq = 0
  /** Filter snapshot `filteredLogs` was built from; while it matches, a flush only handles the new lines. */
  let syncedFilterKey = ''

  function currentFilter(): LogFilterKey {
    return {
      processId: selectedProcessId.value,
      instanceId: selectedInstanceId.value,
      level: levelFilter.value,
      query: searchQuery.value.trim().toLowerCase(),
    }
  }

  const filterKey = computed(() => {
    const f = currentFilter()
    return `${f.processId}|${f.instanceId}|${f.level}|${f.query}`
  })

  function recomputeFiltered() {
    syncedFilterKey = filterKey.value
    const filter = currentFilter()
    filteredLogs.value = logs.value.filter((l) => matchesFilter(l, filter))
  }

  /** Drops the prefix already trimmed out of `logs`: `filteredLogs` is seq-monotonic, so binary search is enough. */
  function trimFiltered() {
    const base = logs.value
    const current = filteredLogs.value
    if (current.length === 0) return
    if (base.length === 0) {
      filteredLogs.value = []
      return
    }
    const minSeq = base[0].seq ?? 0
    if ((current[0].seq ?? 0) >= minSeq) return

    let lo = 0
    let hi = current.length
    while (lo < hi) {
      const mid = (lo + hi) >> 1
      if ((current[mid].seq ?? 0) < minSeq) lo = mid + 1
      else hi = mid
    }
    filteredLogs.value = current.slice(lo)
  }

  function appendFiltered(newLines: SeqLogLine[]) {
    if (filterKey.value !== syncedFilterKey) {
      recomputeFiltered()
      return
    }
    if (newLines.length === 0) return

    const filter = currentFilter()
    const matched: SeqLogLine[] = []
    for (const line of newLines) {
      if (matchesFilter(line, filter)) matched.push(line)
    }

    if (matched.length > 0) filteredLogs.value = filteredLogs.value.concat(matched)
    trimFiltered()
  }

  function flushLogs() {
    rafHandle = null
    if (pendingQueue.length === 0) return

    const incoming = pendingQueue.slice()
    pendingQueue.length = 0

    const next = logs.value.concat(incoming)
    logs.value = next.length > LOG_CAP ? next.slice(next.length - LOG_CAP) : next

    appendFiltered(incoming)
    flushTick.value++
  }

  /** Flushes pending lines synchronously (tests and stopListening) and drops any queued rAF. */
  function flushNow() {
    if (rafHandle !== null && typeof cancelAnimationFrame !== 'undefined') {
      cancelAnimationFrame(rafHandle)
    }
    flushLogs()
  }

  function enqueueLogs(lines: LogLine[]) {
    for (const line of lines) {
      pendingQueue.push({ ...line, seq: nextSeq++ })
    }
    if (rafHandle === null) {
      if (typeof requestAnimationFrame !== 'undefined') {
        rafHandle = requestAnimationFrame(flushLogs)
      } else {
        flushLogs()
      }
    }
  }

  // Only a filter change recomputes the whole list; `sync` keeps the old computed's synchronous semantics
  watch(filterKey, recomputeFiltered, { immediate: true, flush: 'sync' })

  const activeProcess = computed(() => {
    if (selectedProcessId.value != null) {
      return (
        runningProcesses.value.find((p) => p.processId === selectedProcessId.value && p.running) ||
        null
      )
    }
    return runningProcesses.value.find((p) => p.running) || null
  })

  async function startListening() {
    if (unlistenBatch) return

    try {
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
    if (unlistenBatch) {
      unlistenBatch()
      unlistenBatch = null
    }
    flushNow()
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
      const fetched = await ipcClient.launchLogs(instanceId)
      // The fetched snapshot replaces `logs`; queued-but-unflushed lines follow it so `seq` stays monotonic
      const carried = pendingQueue.splice(0, pendingQueue.length)
      logs.value = fetched
        .map((line) => ({ ...line, seq: nextSeq++ }))
        .concat(carried.map((line) => ({ ...line, seq: nextSeq++ })))
      recomputeFiltered()
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
    recomputeFiltered()
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
    entries: logs,
    filteredLogs,
    flushTick,
    runningProcesses,
    selectedInstanceId,
    selectedProcessId,
    levelFilter,
    searchQuery,
    autoScroll,
    loading,
    activeProcess,
    enqueueLogs,
    flushNow,
    startListening,
    stopListening,
    fetchLogs,
    killRunningProcess,
    clearLogs,
    copyLogsToClipboard,
    exportLogs,
  }
})

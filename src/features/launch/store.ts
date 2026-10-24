import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import {
  ipcClient,
  AppErrorEx,
  type LaunchProcessInfo,
  type LaunchStartResult,
} from '../../ipc/client'
import { useInstancesStore } from '../instances/store'
import { useAccountsStore } from '../accounts/store'

export type LaunchStage =
  | 'idle'
  | 'precheck'
  | 'java'
  | 'account'
  | 'repair'
  | 'natives'
  | 'args'
  | 'prelaunch'
  | 'spawn'
  | 'running'

export const useLaunchStore = defineStore('launch', () => {
  const instancesStore = useInstancesStore()
  const accountsStore = useAccountsStore()

  const selectedInstanceId = ref<string | null>(null)
  const isLaunching = ref(false)
  const currentStage = ref<LaunchStage>('idle')
  const launchProgress = ref(0)
  const activeTaskId = ref<string | null>(null)
  const runningProcesses = ref<LaunchProcessInfo[]>([])

  // Java missing recovery state
  const javaMissingMajor = ref<string | null>(null)
  const isDownloadingJava = ref(false)

  let unlistenStarted: UnlistenFn | null = null
  let unlistenExit: UnlistenFn | null = null
  let unlistenCrashed: UnlistenFn | null = null

  const selectedInstance = computed(() => {
    if (!selectedInstanceId.value) {
      return (
        instancesStore.instances.find((i) => i.config?.pinned) ||
        instancesStore.instances[0] ||
        null
      )
    }
    return (
      instancesStore.instances.find((i) => i.id === selectedInstanceId.value) ||
      instancesStore.instances[0] ||
      null
    )
  })

  const selectedAccount = computed(() => accountsStore.selectedAccount)

  const activeProcess = computed(() => {
    return runningProcesses.value.find((p) => p.running) || null
  })

  async function startListening() {
    if (unlistenStarted) return

    try {
      unlistenStarted = await listen<LaunchProcessInfo>('launch://started', (event) => {
        const proc = event.payload
        const idx = runningProcesses.value.findIndex((p) => p.processId === proc.processId)
        if (idx >= 0) {
          runningProcesses.value[idx] = proc
        } else {
          runningProcesses.value.unshift(proc)
        }
        currentStage.value = 'running'
        isLaunching.value = false
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
          if (activeProcess.value == null) {
            currentStage.value = 'idle'
            launchProgress.value = 0
          }
        },
      )

      unlistenCrashed = await listen<{ processId: number; instanceId: string; exitCode: number }>(
        'launch://crashed',
        (event) => {
          const { processId, exitCode } = event.payload
          const proc = runningProcesses.value.find((p) => p.processId === processId)
          if (proc) {
            proc.running = false
            proc.exitCode = exitCode
          }
          currentStage.value = 'idle'
          launchProgress.value = 0
        },
      )
    } catch {
      // Mock / Vitest environment fallback
    }
  }

  function stopListening() {
    if (unlistenStarted) {
      unlistenStarted()
      unlistenStarted = null
    }
    if (unlistenExit) {
      unlistenExit()
      unlistenExit = null
    }
    if (unlistenCrashed) {
      unlistenCrashed()
      unlistenCrashed = null
    }
  }

  async function fetchProcesses(): Promise<void> {
    try {
      runningProcesses.value = await ipcClient.launchProcesses()
    } catch {
      // Ignore in mock/initial setup
    }
  }

  function selectInstance(id: string): void {
    selectedInstanceId.value = id
  }

  async function startLaunch(): Promise<LaunchStartResult | null> {
    const instance = selectedInstance.value
    if (!instance) {
      throw new Error('launch.noInstance')
    }
    const account = selectedAccount.value
    if (!account) {
      throw new Error('launch.noAccount')
    }

    isLaunching.value = true
    currentStage.value = 'precheck'
    launchProgress.value = 0.1
    javaMissingMajor.value = null

    try {
      currentStage.value = 'java'
      launchProgress.value = 0.3

      const result = await ipcClient.launchInstance(instance.id, account.id)
      activeTaskId.value = result.taskId

      currentStage.value = 'running'
      launchProgress.value = 1.0

      if (result.processId) {
        const proc: LaunchProcessInfo = {
          processId: result.processId,
          instanceId: instance.id,
          accountName: account.username,
          startedAt: new Date().toISOString(),
          running: true,
          exitCode: null,
        }
        runningProcesses.value = [
          proc,
          ...runningProcesses.value.filter((p) => p.processId !== result.processId),
        ]
      }

      await fetchProcesses()
      return result
    } catch (err: unknown) {
      currentStage.value = 'idle'
      launchProgress.value = 0
      isLaunching.value = false

      if (err instanceof AppErrorEx && err.code === 'launch.java.notFound') {
        javaMissingMajor.value = err.params?.major || '17'
      }
      throw err
    } finally {
      isLaunching.value = false
    }
  }

  async function downloadJavaRuntime(component = 'java-runtime-gamma'): Promise<string> {
    isDownloadingJava.value = true
    try {
      const taskId = await ipcClient.javaDownload(component)
      javaMissingMajor.value = null
      return taskId
    } finally {
      isDownloadingJava.value = false
    }
  }

  async function killProcess(processId: number): Promise<void> {
    await ipcClient.launchKill(processId)
    const proc = runningProcesses.value.find((p) => p.processId === processId)
    if (proc) {
      proc.running = false
      proc.exitCode = -1
    }
  }

  return {
    selectedInstanceId,
    selectedInstance,
    selectedAccount,
    isLaunching,
    currentStage,
    launchProgress,
    activeTaskId,
    runningProcesses,
    activeProcess,
    javaMissingMajor,
    isDownloadingJava,
    startListening,
    stopListening,
    fetchProcesses,
    selectInstance,
    startLaunch,
    downloadJavaRuntime,
    killProcess,
  }
})

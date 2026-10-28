import { defineStore } from 'pinia'
import { ref } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { ipcClient, type CrashResult } from '../../ipc/client'

export const useCrashStore = defineStore('crash', () => {
  const isOpen = ref(false)
  const isAnalyzing = ref(false)
  const isExporting = ref(false)
  const currentResult = ref<CrashResult | null>(null)
  const currentInstanceId = ref<string>('')
  const lastError = ref<string | null>(null)

  let unlistenCrashEvent: UnlistenFn | null = null

  async function startListening(): Promise<void> {
    if (unlistenCrashEvent) return

    try {
      unlistenCrashEvent = await listen<{
        processId: number
        instanceId: string
        exitCode: number
      }>('launch://crashed', async (event) => {
        const { instanceId } = event.payload
        try {
          const settings = await ipcClient.settingsGet()
          if (settings.launch && settings.launch.crashAnalysis === false) {
            return
          }
        } catch {
          // If reading settings fails, proceed with analysis
        }

        await analyzeInstance(instanceId, null, true)
      })
    } catch {
      // Mock / non-tauri environment fallback
    }
  }

  function stopListening(): void {
    if (unlistenCrashEvent) {
      unlistenCrashEvent()
      unlistenCrashEvent = null
    }
  }

  async function analyzeInstance(
    instanceId: string,
    customLogPath?: string | null,
    autoOpen = true,
  ): Promise<CrashResult | null> {
    isAnalyzing.value = true
    lastError.value = null
    currentInstanceId.value = instanceId

    try {
      const result = await ipcClient.crashAnalyze(instanceId, customLogPath)
      currentResult.value = result
      if (autoOpen) {
        isOpen.value = true
      }
      return result
    } catch (e: unknown) {
      lastError.value = String(e)
      return null
    } finally {
      isAnalyzing.value = false
    }
  }

  async function exportReport(destZipPath: string): Promise<string> {
    if (!currentInstanceId.value) {
      throw new Error('No instance selected for crash report export')
    }
    isExporting.value = true
    try {
      return await ipcClient.crashExportReport(currentInstanceId.value, destZipPath)
    } finally {
      isExporting.value = false
    }
  }

  function openDialog(result?: CrashResult): void {
    if (result) {
      currentResult.value = result
      currentInstanceId.value = result.instanceId
    }
    isOpen.value = true
  }

  function closeDialog(): void {
    isOpen.value = false
  }

  return {
    isOpen,
    isAnalyzing,
    isExporting,
    currentResult,
    currentInstanceId,
    lastError,
    startListening,
    stopListening,
    analyzeInstance,
    exportReport,
    openDialog,
    closeDialog,
  }
})

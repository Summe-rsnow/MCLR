import { defineStore } from 'pinia'
import { ref } from 'vue'
import {
  ipcClient,
  type AddItemOutcome,
  type ApplyReport,
  type ApplyRequest,
  type CompPlatform,
  type PresetItem,
  type PresetPinRule,
  type ResourcePreset,
  type StoreInfo,
} from '../../ipc/client'
import { useDialog } from '../../ui'
import { i18n } from '../../i18n'
import { awaitTask } from '../tasks/awaitTask'

export const usePresetsStore = defineStore('presets', () => {
  const presets = ref<ResourcePreset[]>([])
  const storeInfo = ref<StoreInfo | null>(null)
  /** Last apply report per instance, kept so the instance tab can show it after the dialog closes. */
  const lastReports = ref<Record<string, ApplyReport>>({})
  const loading = ref(false)
  const applying = ref(false)

  async function loadPresets(): Promise<ResourcePreset[]> {
    loading.value = true
    try {
      presets.value = await ipcClient.presetsList()
      return presets.value
    } finally {
      loading.value = false
    }
  }

  async function loadStoreInfo(): Promise<StoreInfo> {
    storeInfo.value = await ipcClient.presetsStoreInfo()
    return storeInfo.value
  }

  async function savePreset(
    preset: Partial<ResourcePreset> & { name: string },
  ): Promise<ResourcePreset> {
    const saved = await ipcClient.presetsSave(preset)
    const idx = presets.value.findIndex((p) => p.id === saved.id)
    if (idx >= 0) {
      presets.value[idx] = saved
    } else {
      presets.value.push(saved)
    }
    return saved
  }

  /**
   * Confirms, then deletes the preset definition. Files already installed in instances are not
   * touched (the backend only drops the preset record).
   */
  async function deletePreset(id: string): Promise<boolean> {
    const target = presets.value.find((p) => p.id === id)
    const dialog = useDialog()
    const ok = await dialog.confirmDanger({
      title: i18n.global.t('presets.deleteConfirm.title'),
      content: i18n.global.t('presets.deleteConfirm.content', {
        name: target?.name ?? id,
      }),
      confirmText: i18n.global.t('common.delete'),
      cancelText: i18n.global.t('common.cancel'),
    })
    if (!ok) return false
    await ipcClient.presetsDelete(id)
    presets.value = presets.value.filter((p) => p.id !== id)
    return true
  }

  async function pin(
    presetId: string,
    platform: CompPlatform,
    projectId: string,
    fileId?: string | null,
    fileName?: string | null,
  ): Promise<void> {
    await ipcClient.presetsPin(presetId, platform, projectId, fileId, fileName)
    await loadPresets()
  }

  async function pinRule(
    presetId: string,
    platform: CompPlatform,
    projectId: string,
    rule: PresetPinRule,
  ): Promise<void> {
    await ipcClient.presetsPinRule(presetId, platform, projectId, rule)
    await loadPresets()
  }

  async function removePinRule(
    presetId: string,
    platform: CompPlatform,
    projectId: string,
    fileId: string,
  ): Promise<void> {
    await ipcClient.presetsRemovePinRule(presetId, platform, projectId, fileId)
    await loadPresets()
  }
  async function removeItem(
    presetId: string,
    platform: CompPlatform,
    projectId: string,
  ): Promise<void> {
    await ipcClient.presetsRemoveItem(presetId, platform, projectId)
    await loadPresets()
  }

  async function addItems(presetId: string, items: PresetItem[]): Promise<AddItemOutcome[]> {
    const outcomes = await ipcClient.presetsAddItems(presetId, items)
    await loadPresets()
    return outcomes
  }

  /**
   * Runs an apply task end to end: schedule, wait for the background task, then collect the
   * report the backend parks under the task id. The report is cached per instance.
   */
  async function applyPresets(request: ApplyRequest): Promise<ApplyReport> {
    applying.value = true
    try {
      const taskId = await ipcClient.presetsApply(request)
      await awaitTask(taskId)
      const report = await ipcClient.presetsApplyReport(taskId)
      lastReports.value = { ...lastReports.value, [request.instanceId]: report }
      return report
    } finally {
      applying.value = false
    }
  }

  function lastReport(instanceId: string): ApplyReport | null {
    return lastReports.value[instanceId] ?? null
  }

  return {
    presets,
    pinRule,
    removePinRule,
    storeInfo,
    lastReports,
    loading,
    applying,
    loadPresets,
    loadStoreInfo,
    savePreset,
    deletePreset,
    pin,
    removeItem,
    addItems,
    applyPresets,
    lastReport,
  }
})

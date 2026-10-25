import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import {
  ipcClient,
  type AccountView,
  type DeviceCodeEvent,
  type AuthlibProfileView,
} from '../../ipc/client'

export const useAccountsStore = defineStore('accounts', () => {
  const accounts = ref<AccountView[]>([])
  const loading = ref(false)
  const initialized = ref(false)
  const deviceCodeDialog = ref(false)
  const deviceCodeData = ref<DeviceCodeEvent | null>(null)

  // Authlib multi-profile modal
  const profileSelectDialog = ref(false)
  const pendingSessionId = ref<string | null>(null)
  const pendingProfiles = ref<AuthlibProfileView[]>([])

  let unlistenDeviceCode: UnlistenFn | null = null

  const selectedAccount = computed(() => accounts.value.find((a) => a.selected) || null)

  async function startListening() {
    if (unlistenDeviceCode) return
    try {
      unlistenDeviceCode = await listen<DeviceCodeEvent>('accounts://deviceCode', (event) => {
        deviceCodeData.value = event.payload
        deviceCodeDialog.value = true
      })
    } catch {
      // In web/test mock mode, listen may not be backed by Tauri
    }
  }

  function stopListening() {
    if (unlistenDeviceCode) {
      unlistenDeviceCode()
      unlistenDeviceCode = null
    }
  }

  async function fetchAccounts() {
    loading.value = true
    try {
      accounts.value = await ipcClient.accountsList()
      initialized.value = true
    } finally {
      loading.value = false
    }
  }

  async function selectAccount(id: string) {
    accounts.value = await ipcClient.accountsSelect(id)
  }

  async function removeAccount(id: string) {
    accounts.value = await ipcClient.accountsRemove(id)
  }

  async function addOffline(username: string, uuidMode: string, customUuid?: string) {
    const newAcc = await ipcClient.accountsAddOffline(username, uuidMode, customUuid)
    await fetchAccounts()
    return newAcc
  }

  async function addMicrosoft() {
    const taskId = await ipcClient.accountsAddMicrosoft()
    return taskId
  }

  async function addAuthlib(server: string, username: string, password: string) {
    const resp = await ipcClient.accountsAddAuthlib(server, username, password)
    if (resp.pendingSessionId && resp.profiles.length > 0) {
      pendingSessionId.value = resp.pendingSessionId
      pendingProfiles.value = resp.profiles
      profileSelectDialog.value = true
      return null
    }
    await fetchAccounts()
    return resp.account ?? null
  }

  async function selectProfile(profileId: string) {
    if (!pendingSessionId.value) return
    const acc = await ipcClient.accountsSelectProfile(pendingSessionId.value, profileId)
    profileSelectDialog.value = false
    pendingSessionId.value = null
    pendingProfiles.value = []
    await fetchAccounts()
    return acc
  }

  async function refreshAccount(id: string) {
    const updated = await ipcClient.accountsRefresh(id)
    const idx = accounts.value.findIndex((a) => a.id === id)
    if (idx !== -1) {
      accounts.value[idx] = updated
    }
    return updated
  }

  async function uploadSkin(id: string, variant: string, pngPath: string) {
    const updated = await ipcClient.accountsSkinUpload(id, variant, pngPath)
    const idx = accounts.value.findIndex((a) => a.id === id)
    if (idx !== -1) {
      accounts.value[idx] = updated
    }
    return updated
  }

  async function setCape(id: string, capeId: string | null) {
    const updated = await ipcClient.accountsCapeSet(id, capeId)
    const idx = accounts.value.findIndex((a) => a.id === id)
    if (idx !== -1) {
      accounts.value[idx] = updated
    }
    return updated
  }

  return {
    accounts,
    loading,
    initialized,
    selectedAccount,
    deviceCodeDialog,
    deviceCodeData,
    profileSelectDialog,
    pendingSessionId,
    pendingProfiles,
    startListening,
    stopListening,
    fetchAccounts,
    selectAccount,
    removeAccount,
    addOffline,
    addMicrosoft,
    addAuthlib,
    selectProfile,
    refreshAccount,
    uploadSkin,
    setCape,
  }
})

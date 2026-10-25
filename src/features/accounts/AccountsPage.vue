<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAccountsStore } from './store'
import type { AccountView, AuthlibProfileView } from '../../ipc/client'
import { openUrl } from '@tauri-apps/plugin-opener'
import { MButton, MCard, MEmpty, MPageHeader, MSkeleton, useDialog, useToast } from '../../ui'
import AccountCardItem from './components/AccountCardItem.vue'
import AccountSkinPanel from './components/AccountSkinPanel.vue'
import AccountModals from './components/AccountModals.vue'

const { t } = useI18n()
const accountsStore = useAccountsStore()
const { confirmDanger } = useDialog()
const toast = useToast()

onMounted(async () => {
  await accountsStore.startListening()
  await accountsStore.fetchAccounts()
})

onUnmounted(() => {
  accountsStore.stopListening()
})

// Dialog controls
const showAddTypeDialog = ref(false)
const showAddOfflineDialog = ref(false)
const showAddAuthlibDialog = ref(false)
const showRenameDialog = ref(false)
const renameTargetAccount = ref<AccountView | null>(null)
const renameNewUsername = ref('')
const renameLoading = ref(false)

// Offline form state
const offlineUsername = ref('')
const offlineUuidMode = ref<'standard' | 'legacy' | 'custom'>('standard')
const offlineSkinModel = ref<'classic' | 'slim'>('classic')
const offlineCustomUuid = ref('')
const offlineError = ref('')

// Authlib form state
const authServer = ref('https://littleskin.cn/api/yggdrasil')
const authAccount = ref('')
const authPassword = ref('')
const authError = ref('')
const authLoading = ref(false)

// Skin upload form state
const showSkinUploadDialog = ref(false)
const skinVariant = ref<'classic' | 'slim'>('classic')
const skinPngPath = ref('')
const skinUploadLoading = ref(false)

function openAddDialog() {
  showAddTypeDialog.value = true
}

function startAddOffline() {
  showAddTypeDialog.value = false
  offlineUsername.value = ''
  offlineUuidMode.value = 'standard'
  offlineSkinModel.value = 'classic'
  offlineCustomUuid.value = ''
  offlineError.value = ''
  showAddOfflineDialog.value = true
}

async function handleAddOffline() {
  const username = offlineUsername.value.trim()
  if (!username) {
    offlineError.value = t('accounts.dialog.validation.usernameRequired')
    return
  }
  if (offlineUuidMode.value === 'custom') {
    const rawUuid = offlineCustomUuid.value.trim()
    const uuidClean = rawUuid.replace(/-/g, '')
    if (uuidClean.length !== 32 || !/^[0-9a-fA-F]{32}$/.test(uuidClean)) {
      offlineError.value = t('accounts.dialog.validation.uuidInvalid')
      return
    }
  }

  try {
    await accountsStore.addOfflineAccount(
      username,
      offlineUuidMode.value,
      offlineUuidMode.value === 'custom' ? offlineCustomUuid.value.trim() : undefined,
      offlineSkinModel.value,
    )
    showAddOfflineDialog.value = false
    toast.success(t('accounts.dialog.addSuccess'))
  } catch (err: unknown) {
    const msg =
      err && typeof err === 'object' && 'message' in err ? String(err.message) : t('error.unknown')
    offlineError.value = msg
  }
}

async function startAddMicrosoft() {
  showAddTypeDialog.value = false
  try {
    await accountsStore.addMicrosoft()
  } catch (err: unknown) {
    const msg =
      err && typeof err === 'object' && 'message' in err ? String(err.message) : t('error.unknown')
    toast.danger(msg)
  }
}

function startAddAuthlib() {
  showAddTypeDialog.value = false
  authAccount.value = ''
  authPassword.value = ''
  authError.value = ''
  showAddAuthlibDialog.value = true
}

async function handleAddAuthlib() {
  const server = authServer.value.trim()
  const account = authAccount.value.trim()
  const password = authPassword.value
  if (!server) {
    authError.value = t('accounts.dialog.validation.serverRequired')
    return
  }
  if (!account) {
    authError.value = t('accounts.dialog.validation.accountRequired')
    return
  }
  if (!password) {
    authError.value = t('accounts.dialog.validation.passwordRequired')
    return
  }

  authLoading.value = true
  authError.value = ''
  try {
    await accountsStore.addAuthlib(server, account, password)
    showAddAuthlibDialog.value = false
    toast.success(t('accounts.dialog.addSuccess'))
  } catch (err: unknown) {
    const msg =
      err && typeof err === 'object' && 'message' in err ? String(err.message) : t('error.unknown')
    authError.value = msg
  } finally {
    authLoading.value = false
  }
}

function copyDeviceCode() {
  if (accountsStore.deviceCodeData) {
    navigator.clipboard.writeText(accountsStore.deviceCodeData.userCode)
    toast.success(t('accounts.dialog.copiedCode'))
  }
}

async function openDeviceUrl() {
  if (accountsStore.deviceCodeData) {
    await openUrl(accountsStore.deviceCodeData.verificationUri)
  }
}

async function handleSelectProfile(profile: AuthlibProfileView) {
  try {
    await accountsStore.selectProfile(profile.id)
    toast.success(t('accounts.actions.profileSelected', { name: profile.name }))
  } catch {
    toast.danger(t('error.unknown'))
  }
}

async function handleSelectAccount(acc: AccountView) {
  if (acc.selected) return
  await accountsStore.selectAccount(acc.id)
}

async function handleRefreshAccount(acc: AccountView) {
  try {
    await accountsStore.refreshAccount(acc.id)
    toast.success(t('accounts.actions.refreshSuccess'))
  } catch {
    toast.danger(t('error.unknown'))
  }
}

async function handleRemoveAccount(acc: AccountView) {
  const confirmed = await confirmDanger({
    title: t('accounts.actions.deleteConfirmTitle'),
    content: t('accounts.actions.deleteConfirmMessage', { name: acc.username }),
  })
  if (confirmed) {
    await accountsStore.removeAccount(acc.id)
  }
}

async function handleCopyUuid(acc: AccountView) {
  try {
    await navigator.clipboard.writeText(acc.uuid)
    toast.success(t('accounts.actions.copyUuidSuccess'))
  } catch {
    toast.danger(t('accounts.actions.copyUuidFailed'))
  }
}

function openRenameDialog(acc: AccountView) {
  renameTargetAccount.value = acc
  renameNewUsername.value = acc.username
  showRenameDialog.value = true
}

async function handleRenameConfirm() {
  if (!renameTargetAccount.value) return
  const newName = renameNewUsername.value.trim()
  if (!newName) return
  renameLoading.value = true
  try {
    await accountsStore.renameOffline(renameTargetAccount.value.id, newName)
    showRenameDialog.value = false
    toast.success(t('accounts.actions.renameSuccess'))
  } catch (err: unknown) {
    const msg =
      err && typeof err === 'object' && 'message' in err ? String(err.message) : t('error.unknown')
    toast.danger(msg)
  } finally {
    renameLoading.value = false
  }
}

function openSkinUpload() {
  skinPngPath.value = ''
  skinVariant.value = 'classic'
  showSkinUploadDialog.value = true
}

async function handleSkinUpload() {
  if (!skinPngPath.value.trim()) return
  skinUploadLoading.value = true
  try {
    if (!accountsStore.selectedAccount) return
    await accountsStore.uploadSkin(
      accountsStore.selectedAccount.id,
      skinVariant.value,
      skinPngPath.value.trim(),
    )
    showSkinUploadDialog.value = false
    toast.success(t('accounts.skin.uploadSuccess'))
  } catch (err: unknown) {
    const msg =
      err && typeof err === 'object' && 'message' in err ? String(err.message) : t('error.unknown')
    toast.danger(msg)
  } finally {
    skinUploadLoading.value = false
  }
}

async function handleSelectCape(capeId: string | null) {
  try {
    if (!accountsStore.selectedAccount) return
    await accountsStore.setCape(accountsStore.selectedAccount.id, capeId)
    toast.success(t('accounts.skin.capeChanged'))
  } catch {
    toast.danger(t('error.unknown'))
  }
}

async function openCloset(serverUrl: string) {
  const closetUrl = serverUrl.replace(/\/api\/yggdrasil\/?$/, '') + '/user/closet'
  await openUrl(closetUrl)
}
</script>

<template>
  <div class="m-page">
    <MPageHeader
      :title="t('accounts.title')"
      :description="t('accounts.subtitle')"
      icon="i-lucide-users"
    >
      <template #actions>
        <MButton variant="primary" size="md" icon="i-lucide-plus" @click="openAddDialog">
          {{ t('accounts.addAccount') }}
        </MButton>
      </template>
    </MPageHeader>

    <!-- Skeleton Loading State -->
    <div
      v-if="
        (!accountsStore.initialized && accountsStore.loading) ||
        (accountsStore.loading && accountsStore.accounts.length === 0)
      "
      data-testid="accounts-skeleton"
      class="grid grid-cols-1 lg:grid-cols-12 gap-6 items-start"
    >
      <div class="lg:col-span-7 flex flex-col gap-3">
        <MCard v-for="n in 3" :key="n" class="p-4">
          <div class="flex items-center justify-between gap-4">
            <div class="flex items-center gap-3">
              <MSkeleton width="2.5rem" height="2.5rem" class="rounded-full" />
              <div class="space-y-1.5">
                <MSkeleton width="6rem" height="1.125rem" />
                <MSkeleton width="12rem" height="0.875rem" />
              </div>
            </div>
            <div class="flex items-center gap-1">
              <MSkeleton width="1.75rem" height="1.75rem" class="rounded-lg" />
              <MSkeleton width="1.75rem" height="1.75rem" class="rounded-lg" />
            </div>
          </div>
        </MCard>
      </div>
      <div class="lg:col-span-5 flex flex-col gap-4">
        <MCard class="p-6 h-[360px] flex flex-col items-center justify-center gap-4">
          <MSkeleton width="6rem" height="6rem" class="rounded-full" />
          <div class="space-y-2 flex flex-col items-center">
            <MSkeleton width="8rem" height="1.25rem" />
            <MSkeleton width="5rem" height="0.875rem" />
          </div>
          <MSkeleton width="75%" height="2.25rem" class="rounded-lg mt-2" />
        </MCard>
      </div>
    </div>

    <!-- Empty State -->
    <MCard v-else-if="accountsStore.accounts.length === 0" class="p-8">
      <MEmpty
        :title="t('accounts.empty')"
        :description="t('accounts.emptySubtitle')"
        icon="i-lucide-user-round"
      >
        <MButton variant="primary" icon="i-lucide-plus" class="mt-4" @click="openAddDialog">
          {{ t('accounts.addAccount') }}
        </MButton>
      </MEmpty>
    </MCard>

    <!-- Main Grid: Accounts List & Skin Panel -->
    <div v-else class="grid grid-cols-1 lg:grid-cols-12 gap-6 items-start">
      <TransitionGroup name="m-list" tag="div" class="lg:col-span-7 flex flex-col gap-3">
        <AccountCardItem
          v-for="(acc, index) in accountsStore.accounts"
          :key="acc.id"
          :account="acc"
          :index="index"
          @select="handleSelectAccount"
          @copy-uuid="handleCopyUuid"
          @rename="openRenameDialog"
          @refresh="handleRefreshAccount"
          @remove="handleRemoveAccount"
        />
      </TransitionGroup>

      <div class="lg:col-span-5 flex flex-col gap-4">
        <AccountSkinPanel
          :selected-account="accountsStore.selectedAccount"
          @upload-skin="openSkinUpload"
          @open-closet="openCloset"
          @select-cape="handleSelectCape"
        />
      </div>
    </div>

    <AccountModals
      v-model:show-add-type-dialog="showAddTypeDialog"
      v-model:show-add-offline-dialog="showAddOfflineDialog"
      v-model:show-add-authlib-dialog="showAddAuthlibDialog"
      v-model:show-skin-upload-dialog="showSkinUploadDialog"
      v-model:show-rename-dialog="showRenameDialog"
      v-model:offline-username="offlineUsername"
      v-model:offline-uuid-mode="offlineUuidMode"
      v-model:offline-skin-model="offlineSkinModel"
      v-model:offline-custom-uuid="offlineCustomUuid"
      v-model:offline-error="offlineError"
      v-model:auth-server="authServer"
      v-model:auth-account="authAccount"
      v-model:auth-password="authPassword"
      v-model:auth-error="authError"
      v-model:auth-loading="authLoading"
      v-model:skin-variant="skinVariant"
      v-model:skin-png-path="skinPngPath"
      v-model:skin-upload-loading="skinUploadLoading"
      v-model:rename-new-username="renameNewUsername"
      v-model:rename-loading="renameLoading"
      :accounts-store="accountsStore"
      @start-add-microsoft="startAddMicrosoft"
      @start-add-offline="startAddOffline"
      @start-add-authlib="startAddAuthlib"
      @confirm-add-offline="handleAddOffline"
      @confirm-add-authlib="handleAddAuthlib"
      @copy-device-code="copyDeviceCode"
      @open-device-url="openDeviceUrl"
      @select-profile="handleSelectProfile"
      @confirm-skin-upload="handleSkinUpload"
      @confirm-rename="handleRenameConfirm"
    />
  </div>
</template>

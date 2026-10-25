<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAccountsStore } from './store'
import type { AccountView, AuthlibProfileView } from '../../ipc/client'
import {
  MButton,
  MIconButton,
  MCard,
  MInput,
  MSelect,
  MBadge,
  MAvatar,
  MDialog,
  MEmpty,
  MPageHeader,
  MBrandIcon,
  MSkeletonList,
  getAccountKindMeta,
  useDialog,
  useToast,
} from '../../ui'
const { t } = useI18n()
const accountsStore = useAccountsStore()
const { confirm } = useDialog()
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

// Offline form state
const offlineUsername = ref('')
const offlineUuidMode = ref<'standard' | 'legacy' | 'custom'>('standard')
const offlineCustomUuid = ref('')
const offlineError = ref('')

const uuidModeOptions = computed(() => [
  { label: t('accounts.dialog.uuidModes.standard'), value: 'standard' },
  { label: t('accounts.dialog.uuidModes.legacy'), value: 'legacy' },
  { label: t('accounts.dialog.uuidModes.custom'), value: 'custom' },
])

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

const skinModelOptions = computed(() => [
  { label: t('accounts.skin.models.classic'), value: 'classic' },
  { label: t('accounts.skin.models.slim'), value: 'slim' },
])

function openAddDialog() {
  showAddTypeDialog.value = true
}

function startAddOffline() {
  showAddTypeDialog.value = false
  offlineUsername.value = ''
  offlineUuidMode.value = 'standard'
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
    const cleanUuid = rawUuid.replace(/-/g, '')
    if (cleanUuid.length !== 32 || !/^[0-9a-fA-F]+$/.test(cleanUuid)) {
      offlineError.value = t('accounts.dialog.validation.uuidInvalid')
      return
    }
  }

  try {
    await accountsStore.addOffline(
      username,
      offlineUuidMode.value,
      offlineUuidMode.value === 'custom' ? offlineCustomUuid.value.trim() : undefined,
    )
    showAddOfflineDialog.value = false
  } catch (err: unknown) {
    const msg = err && typeof err === 'object' && 'code' in err ? String(err.code) : 'unknown'
    offlineError.value = msg
  }
}

async function startAddMicrosoft() {
  showAddTypeDialog.value = false
  try {
    await accountsStore.addMicrosoft()
  } catch (err: unknown) {
    if (err && typeof err === 'object' && 'code' in err && (err as { code: string }).code === 'notConfigured') {
      toast.danger(t('error.notConfigured', { what: 'Microsoft Client ID' }))
    } else {
      toast.danger(t('error.unknown'))
    }
  }
}

function startAddAuthlib() {
  showAddTypeDialog.value = false
  authServer.value = 'https://littleskin.cn/api/yggdrasil'
  authAccount.value = ''
  authPassword.value = ''
  authError.value = ''
  showAddAuthlibDialog.value = true
}

async function handleAddAuthlib() {
  if (!authServer.value.trim()) {
    authError.value = t('accounts.dialog.validation.serverRequired')
    return
  }
  if (!authAccount.value.trim()) {
    authError.value = t('accounts.dialog.validation.accountRequired')
    return
  }
  if (!authPassword.value.trim()) {
    authError.value = t('accounts.dialog.validation.passwordRequired')
    return
  }

  authLoading.value = true
  authError.value = ''
  try {
    await accountsStore.addAuthlib(
      authServer.value.trim(),
      authAccount.value.trim(),
      authPassword.value,
    )
    showAddAuthlibDialog.value = false
  } catch (err: unknown) {
    const code = err && typeof err === 'object' && 'code' in err ? String(err.code) : 'auth.authlib.loginFailed'
    authError.value = t(`error.${code}`) || code
  } finally {
    authLoading.value = false
  }
}

async function handleSelectProfile(p: AuthlibProfileView) {
  try {
    await accountsStore.selectProfile(p.id)
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
    toast.success(t('common.refresh'))
  } catch {
    toast.danger(t('error.unknown'))
  }
}

async function handleRemoveAccount(acc: AccountView) {
  const confirmed = await confirm({
    title: t('accounts.actions.deleteConfirmTitle'),
    content: t('accounts.actions.deleteConfirmMessage', { name: acc.username }),
  })
  if (confirmed) {
    await accountsStore.removeAccount(acc.id)
  }
}

function copyDeviceCode() {
  if (accountsStore.deviceCodeData?.userCode) {
    navigator.clipboard?.writeText(accountsStore.deviceCodeData.userCode)
    toast.success(t('common.copied'))
  }
}

async function openDeviceUrl() {
  if (accountsStore.deviceCodeData?.verificationUri) {
    try {
      const opener = await import('@tauri-apps/plugin-opener')
      await opener.openUrl(accountsStore.deviceCodeData.verificationUri)
    } catch {
      window.open(accountsStore.deviceCodeData.verificationUri, '_blank')
    }
  }
}

function openSkinUpload() {
  skinPngPath.value = ''
  skinVariant.value = (accountsStore.selectedAccount?.defaultSkinModel as 'classic' | 'slim') || 'classic'
  showSkinUploadDialog.value = true
}

async function handleSkinUpload() {
  if (!accountsStore.selectedAccount || !skinPngPath.value.trim()) return
  skinUploadLoading.value = true
  try {
    await accountsStore.uploadSkin(
      accountsStore.selectedAccount.id,
      skinVariant.value,
      skinPngPath.value.trim(),
    )
    showSkinUploadDialog.value = false
    toast.success(t('accounts.skin.uploadSuccess'))
  } catch {
    toast.danger(t('error.auth.skin.uploadFailed'))
  } finally {
    skinUploadLoading.value = false
  }
}

async function handleSelectCape(capeId: string | null) {
  if (!accountsStore.selectedAccount) return
  try {
    await accountsStore.setCape(accountsStore.selectedAccount.id, capeId)
  } catch {
    toast.danger(t('error.auth.cape.setFailed'))
  }
}

async function openCloset(serverUrl?: string | null) {
  if (!serverUrl) return
  const closetUrl = `${serverUrl.replace(/\/+$/, '')}/user/closet`
  try {
    const opener = await import('@tauri-apps/plugin-opener')
    await opener.openUrl(closetUrl)
  } catch {
    window.open(closetUrl, '_blank')
  }
}
</script>

<template>
  <div class="m-page">
    <!-- Header -->
    <MPageHeader
      :title="t('accounts.title')"
      :description="t('accounts.emptySubtitle')"
      icon="i-lucide-user-round"
    >
      <template #actions>
        <MButton variant="primary" icon="i-lucide-plus" @click="openAddDialog">
          {{ t('accounts.addAccount') }}
        </MButton>
      </template>
    </MPageHeader>
    <!-- Loading Skeleton State -->
    <div v-if="!accountsStore.initialized && accountsStore.loading" class="grid grid-cols-1 lg:grid-cols-12 gap-6">
      <div class="lg:col-span-7">
        <MSkeletonList :rows="3" variant="card" />
      </div>
      <div class="lg:col-span-5">
        <MSkeletonList :rows="1" variant="card" />
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
    <div v-else class="grid grid-cols-1 lg:grid-cols-12 gap-6">
      <!-- Accounts List (Left 7 Cols) -->
      <TransitionGroup name="m-list" tag="div" class="lg:col-span-7 flex flex-col gap-3">
        <MCard
          v-for="(acc, index) in accountsStore.accounts"
          :key="acc.id"
          hoverable
          :variant="acc.selected ? 'tinted' : 'raised'"
          class="p-4 cursor-pointer m-stagger"
          :style="{ '--i': index }"
          @click="handleSelectAccount(acc)"
        >
          <div class="flex items-center justify-between gap-4">
            <div class="flex items-center gap-3 min-w-0">
              <MAvatar
                :src="acc.skinHeadDataUrl ?? undefined"
                :name="acc.username"
                :size="40"
              />
              <div class="min-w-0">
                <div class="flex items-center gap-2">
                  <span class="font-medium text-fg truncate">
                    {{ acc.username }}
                  </span>
                  <MBadge
                    :variant="acc.kind === 'microsoft' ? 'primary' : 'neutral'"
                    size="sm"
                  >
                    <MBrandIcon v-if="acc.kind === 'microsoft'" name="microsoft" class="w-3.5 h-3.5 mr-1" />
                    <span v-else :class="getAccountKindMeta(acc.kind).icon" class="w-3.5 h-3.5 mr-1 inline-block" />
                    {{ t(`accounts.types.${acc.kind}`) }}
                  </MBadge>
                  <MBadge v-if="acc.selected" variant="success" size="sm">
                    {{ t('accounts.status.current') }}
                  </MBadge>
                </div>
                <div class="text-xs text-fg-muted truncate mt-0.5">
                  <span v-if="acc.serverName">{{ acc.serverName }} · </span>
                  <span>{{ acc.uuid }}</span>
                </div>
              </div>
            </div>

            <div class="flex items-center gap-1 shrink-0" @click.stop>
              <MIconButton
                icon="i-lucide-refresh-cw"
                size="sm"
                variant="text"
                :title="t('accounts.actions.refresh')"
                @click="handleRefreshAccount(acc)"
              />
              <MIconButton
                icon="i-lucide-trash-2"
                size="sm"
                variant="text"
                class="text-danger"
                :title="t('accounts.actions.delete')"
                @click="handleRemoveAccount(acc)"
              />
            </div>
          </div>
        </MCard>
      </TransitionGroup>

      <!-- Skin & Cape Panel (Right 5 Cols) -->
      <div class="lg:col-span-5 flex flex-col gap-4">
        <MCard class="p-5 flex flex-col gap-4">
          <h2 class="text-lg font-semibold text-fg">
            {{ t('accounts.skin.title') }}
          </h2>

          <template v-if="accountsStore.selectedAccount">
            <!-- 2D Head Preview -->
            <div class="flex items-center gap-4 p-4 rounded-xl bg-sunken">
              <div class="w-16 h-16 rounded-lg bg-raised flex items-center justify-center border border-line overflow-hidden">
                <img
                  v-if="accountsStore.selectedAccount.skinHeadDataUrl"
                  :src="accountsStore.selectedAccount.skinHeadDataUrl"
                  alt="Skin Head"
                  class="w-full h-full object-contain image-rendering-pixelated"
                 loading="lazy" decoding="async"/>
                <MAvatar
                  v-else
                  :name="accountsStore.selectedAccount.username"
                  :size="64"
                />
              </div>
              <div class="flex-1 min-w-0">
                <div class="font-medium text-fg truncate">
                  {{ accountsStore.selectedAccount.username }}
                </div>
                <div class="text-xs text-fg-muted mt-1">
                  {{ t('accounts.skin.model') }}:
                  {{ t(`accounts.skin.models.${accountsStore.selectedAccount.defaultSkinModel || 'classic'}`) }}
                </div>
              </div>
            </div>

            <!-- Skin Actions -->
            <div class="flex flex-col gap-2">
              <MButton
                v-if="accountsStore.selectedAccount.kind !== 'offline'"
                variant="tonal"
                icon="i-lucide-upload"
                @click="openSkinUpload"
              >
                {{ t('accounts.skin.upload') }}
              </MButton>
              <MButton
                v-if="accountsStore.selectedAccount.kind === 'authlib' && accountsStore.selectedAccount.server"
                variant="text"
                icon="i-lucide-external-link"
                @click="openCloset(accountsStore.selectedAccount.server)"
              >
                {{ t('accounts.actions.openCloset') }}
              </MButton>
              <p v-if="accountsStore.selectedAccount.kind === 'offline'" class="text-xs text-fg-muted">
                {{ t('accounts.skin.unsupportedOffline') }}
              </p>
            </div>

            <!-- Cape Selection (for accounts that have capes) -->
            <div v-if="accountsStore.selectedAccount.capes.length > 0" class="flex flex-col gap-2 mt-2">
              <div class="text-sm font-medium text-fg">
                {{ t('accounts.skin.capes') }}
              </div>
              <div class="grid grid-cols-2 gap-2">
                <div
                  class="p-2 rounded-lg border text-center cursor-pointer text-xs transition"
                  :class="[
                    !accountsStore.selectedAccount.capes.some((c) => c.active)
                      ? 'border-primary bg-primary-soft text-primary font-medium'
                      : 'border-line text-fg-muted hover:bg-sunken',
                  ]"
                  @click="handleSelectCape(null)"
                >
                  {{ t('accounts.skin.noneCape') }}
                </div>
                <div
                  v-for="cape in accountsStore.selectedAccount.capes"
                  :key="cape.id"
                  class="p-2 rounded-lg border text-center cursor-pointer text-xs truncate transition"
                  :class="[
                    cape.active
                      ? 'border-primary bg-primary-soft text-primary font-medium'
                      : 'border-line text-fg hover:bg-sunken',
                  ]"
                  @click="handleSelectCape(cape.id)"
                >
                  {{ cape.alias }}
                </div>
              </div>
            </div>
          </template>

          <MEmpty v-else :description="t('accounts.skin.noAccount')" />
        </MCard>
      </div>
    </div>

    <!-- Dialog 1: Add Account Type Picker -->
    <MDialog
      v-model:open="showAddTypeDialog"
      :title="t('accounts.addAccount')"
    >
      <div class="flex flex-col gap-3 py-2">
        <MButton
          variant="tonal"
          class="justify-start py-3"
          @click="startAddMicrosoft"
        >
          <template #icon>
            <MBrandIcon name="microsoft" class="w-5 h-5 mr-2" />
          </template>
          <div class="text-left">
            <div class="font-medium">{{ t('accounts.types.microsoft') }}</div>
            <div class="text-xs text-fg-muted">OAuth 2.0 / Xbox Live</div>
          </div>
        </MButton>

        <MButton
          variant="tonal"
          icon="i-lucide-user-round"
          class="justify-start py-3"
          @click="startAddOffline"
        >
          <div class="text-left ml-2">
            <div class="font-medium">{{ t('accounts.types.offline') }}</div>
            <div class="text-xs text-fg-muted">Standard MD5 / Custom UUID</div>
          </div>
        </MButton>

        <MButton
          variant="tonal"
          icon="i-lucide-shield-check"
          class="justify-start py-3"
          @click="startAddAuthlib"
        >
          <div class="text-left ml-2">
            <div class="font-medium">{{ t('accounts.types.authlib') }}</div>
            <div class="text-xs text-fg-muted">Authlib-Injector / LittleSkin</div>
          </div>
        </MButton>
      </div>
      <template #footer>
        <MButton variant="text" @click="showAddTypeDialog = false">
          {{ t('common.cancel') }}
        </MButton>
      </template>
    </MDialog>

    <!-- Dialog 2: Add Offline Dialog -->
    <MDialog
      v-model:open="showAddOfflineDialog"
      :title="t('accounts.dialog.offlineTitle')"
    >
      <div class="flex flex-col gap-4 py-2">
        <div>
          <label class="text-xs font-medium text-fg-muted mb-1 block">
            {{ t('accounts.dialog.username') }}
          </label>
          <MInput
            v-model="offlineUsername"
            :placeholder="t('accounts.dialog.usernamePlaceholder')"
          />
        </div>

        <div>
          <label class="text-xs font-medium text-fg-muted mb-1 block">
            {{ t('accounts.dialog.uuidMode') }}
          </label>
          <MSelect
            v-model="offlineUuidMode"
            :options="uuidModeOptions"
          />
        </div>

        <div v-if="offlineUuidMode === 'custom'">
          <label class="text-xs font-medium text-fg-muted mb-1 block">
            {{ t('accounts.dialog.customUuid') }}
          </label>
          <MInput
            v-model="offlineCustomUuid"
            :placeholder="t('accounts.dialog.customUuidPlaceholder')"
          />
        </div>

        <div v-if="offlineError" class="text-xs text-danger">
          {{ offlineError }}
        </div>
      </div>
      <template #footer>
        <MButton variant="text" @click="showAddOfflineDialog = false">
          {{ t('common.cancel') }}
        </MButton>
        <MButton variant="primary" @click="handleAddOffline">
          {{ t('common.confirm') }}
        </MButton>
      </template>
    </MDialog>

    <!-- Dialog 3: Add Authlib Dialog -->
    <MDialog
      v-model:open="showAddAuthlibDialog"
      :title="t('accounts.dialog.authlibTitle')"
    >
      <div class="flex flex-col gap-4 py-2">
        <div>
          <label class="text-xs font-medium text-fg-muted mb-1 block">
            {{ t('accounts.dialog.authServer') }}
          </label>
          <MInput
            v-model="authServer"
            :placeholder="t('accounts.dialog.authServerPlaceholder')"
          />
        </div>

        <div>
          <label class="text-xs font-medium text-fg-muted mb-1 block">
            {{ t('accounts.dialog.authAccount') }}
          </label>
          <MInput
            v-model="authAccount"
            :placeholder="t('accounts.dialog.authAccountPlaceholder')"
          />
        </div>

        <div>
          <label class="text-xs font-medium text-fg-muted mb-1 block">
            {{ t('accounts.dialog.authPassword') }}
          </label>
          <MInput
            v-model="authPassword"
            type="password"
            :placeholder="t('accounts.dialog.authPasswordPlaceholder')"
          />
        </div>

        <div v-if="authError" class="text-xs text-danger">
          {{ authError }}
        </div>
      </div>
      <template #footer>
        <MButton variant="text" @click="showAddAuthlibDialog = false">
          {{ t('common.cancel') }}
        </MButton>
        <MButton
          variant="primary"
          :loading="authLoading"
          @click="handleAddAuthlib"
        >
          {{ t('common.confirm') }}
        </MButton>
      </template>
    </MDialog>

    <!-- Dialog 4: Microsoft Device Code Modal -->
    <MDialog
      v-model:open="accountsStore.deviceCodeDialog"
      :title="t('accounts.dialog.msDeviceTitle')"
    >
      <div v-if="accountsStore.deviceCodeData" class="flex flex-col gap-4 py-2">
        <p class="text-sm text-fg">
          {{ t('accounts.dialog.msDevicePrompt') }}
        </p>

        <div class="flex items-center justify-between p-4 rounded-xl m-well">
          <div>
            <div class="text-xs text-fg-muted">
              {{ t('accounts.dialog.msCodeLabel') }}
            </div>
            <div class="text-xl font-mono font-bold tracking-wider text-primary mt-0.5">
              {{ accountsStore.deviceCodeData.userCode }}
            </div>
          </div>
          <MButton variant="tonal" size="sm" icon="i-lucide-copy" @click="copyDeviceCode">
            {{ t('common.copy') }}
          </MButton>
        </div>

        <div class="flex items-center gap-2">
          <MButton variant="primary" icon="i-lucide-external-link" @click="openDeviceUrl">
            {{ t('accounts.dialog.msOpenBrowser') }}
          </MButton>
        </div>

        <p class="text-xs text-fg-muted flex items-center gap-2">
          <span class="i-lucide-loader animate-spin inline-block" />
          {{ t('accounts.dialog.msWaiting') }}
        </p>
      </div>
      <template #footer>
        <MButton variant="text" @click="accountsStore.deviceCodeDialog = false">
          {{ t('common.close') }}
        </MButton>
      </template>
    </MDialog>

    <!-- Dialog 5: Authlib Multi-profile Picker -->
    <MDialog
      v-model:open="accountsStore.profileSelectDialog"
      :title="t('accounts.actions.selectProfileTitle')"
    >
      <div class="flex flex-col gap-3 py-2">
        <p class="text-sm text-fg">
          {{ t('accounts.actions.selectProfilePrompt') }}
        </p>
        <div class="flex flex-col gap-2">
          <MCard
            v-for="p in accountsStore.pendingProfiles"
            :key="p.id"
            hoverable
            class="p-3 cursor-pointer border border-line"
            @click="handleSelectProfile(p)"
          >
            <div class="font-medium text-fg">{{ p.name }}</div>
            <div class="text-xs text-fg-muted">{{ p.id }}</div>
          </MCard>
        </div>
      </div>
      <template #footer>
        <MButton variant="text" @click="accountsStore.profileSelectDialog = false">
          {{ t('common.cancel') }}
        </MButton>
      </template>
    </MDialog>

    <!-- Dialog 6: Skin Upload Dialog -->
    <MDialog
      v-model:open="showSkinUploadDialog"
      :title="t('accounts.skin.upload')"
    >
      <div class="flex flex-col gap-4 py-2">
        <div>
          <label class="text-xs font-medium text-fg-muted mb-1 block">
            {{ t('accounts.skin.model') }}
          </label>
          <MSelect
            v-model="skinVariant"
            :options="skinModelOptions"
          />
        </div>

        <div>
          <label class="text-xs font-medium text-fg-muted mb-1 block">
            {{ t('accounts.skin.uploadPathPrompt') }}
          </label>
          <MInput
            v-model="skinPngPath"
            placeholder="C:\Users\...\skin.png"
          />
        </div>
      </div>
      <template #footer>
        <MButton variant="text" @click="showSkinUploadDialog = false">
          {{ t('common.cancel') }}
        </MButton>
        <MButton
          variant="primary"
          :loading="skinUploadLoading"
          :disabled="!skinPngPath.trim()"
          @click="handleSkinUpload"
        >
          {{ t('common.confirm') }}
        </MButton>
      </template>
    </MDialog>
  </div>
</template>

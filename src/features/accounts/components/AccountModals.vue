<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { useAccountsStore } from '../store'
import type { AuthlibProfileView } from '../../../ipc/client'
import { MDialog, MButton, MInput, MSelect, MPathPicker, MCard, MBrandIcon } from '../../../ui'

defineProps<{
  accountsStore: ReturnType<typeof useAccountsStore>
}>()

const showAddTypeDialog = defineModel<boolean>('showAddTypeDialog', { required: true })
const showAddOfflineDialog = defineModel<boolean>('showAddOfflineDialog', { required: true })
const showAddAuthlibDialog = defineModel<boolean>('showAddAuthlibDialog', { required: true })
const showSkinUploadDialog = defineModel<boolean>('showSkinUploadDialog', { required: true })
const showRenameDialog = defineModel<boolean>('showRenameDialog', { required: true })

const offlineUsername = defineModel<string>('offlineUsername', { required: true })
const offlineUuidMode = defineModel<'standard' | 'legacy' | 'custom'>('offlineUuidMode', {
  required: true,
})
const offlineSkinModel = defineModel<'classic' | 'slim'>('offlineSkinModel', { required: true })
const offlineCustomUuid = defineModel<string>('offlineCustomUuid', { required: true })
const offlineError = defineModel<string>('offlineError', { required: true })

const authServer = defineModel<string>('authServer', { required: true })
const authAccount = defineModel<string>('authAccount', { required: true })
const authPassword = defineModel<string>('authPassword', { required: true })
const authError = defineModel<string>('authError', { required: true })
const authLoading = defineModel<boolean>('authLoading', { required: true })

const skinVariant = defineModel<'classic' | 'slim'>('skinVariant', { required: true })
const skinPngPath = defineModel<string>('skinPngPath', { required: true })
const skinUploadLoading = defineModel<boolean>('skinUploadLoading', { required: true })

const renameNewUsername = defineModel<string>('renameNewUsername', { required: true })
const renameLoading = defineModel<boolean>('renameLoading', { required: true })

const emit = defineEmits<{
  (e: 'start-add-microsoft'): void
  (e: 'start-add-offline'): void
  (e: 'start-add-authlib'): void
  (e: 'confirm-add-offline'): void
  (e: 'confirm-add-authlib'): void
  (e: 'copy-device-code'): void
  (e: 'open-device-url'): void
  (e: 'select-profile', profile: AuthlibProfileView): void
  (e: 'confirm-skin-upload'): void
  (e: 'confirm-rename'): void
}>()

const { t } = useI18n()

const uuidModeOptions = computed(() => [
  { label: t('accounts.dialog.uuidModes.standard'), value: 'standard' },
  { label: t('accounts.dialog.uuidModes.legacy'), value: 'legacy' },
  { label: t('accounts.dialog.uuidModes.custom'), value: 'custom' },
])

const offlineSkinModelOptions = computed(() => [
  { label: t('accounts.dialog.skinModels.classic'), value: 'classic' },
  { label: t('accounts.dialog.skinModels.slim'), value: 'slim' },
])

const skinModelOptions = computed(() => [
  { label: t('accounts.skin.models.classic'), value: 'classic' },
  { label: t('accounts.skin.models.slim'), value: 'slim' },
])
</script>

<template>
  <!-- Dialog 1: Add Account Type Picker -->
  <MDialog v-model:open="showAddTypeDialog" :title="t('accounts.addAccount')">
    <div class="flex flex-col gap-2.5 py-2">
      <button
        type="button"
        class="w-full flex items-center px-4 py-3 rounded-xl border border-line/60 bg-raised hover:bg-sunken/60 active:scale-[0.99] transition-all cursor-pointer text-left select-none outline-none group"
        @click="emit('start-add-microsoft')"
      >
        <div
          class="w-10 h-10 rounded-xl bg-sunken/60 flex items-center justify-center shrink-0 text-primary"
        >
          <MBrandIcon name="microsoft" class="w-5 h-5" />
        </div>
        <div class="flex flex-col text-left min-w-0 ml-3.5">
          <span class="text-sm font-semibold text-fg">{{ t('accounts.types.microsoft') }}</span>
          <span class="text-xs text-fg-muted mt-0.5">OAuth 2.0 / Xbox Live</span>
        </div>
      </button>

      <button
        type="button"
        class="w-full flex items-center px-4 py-3 rounded-xl border border-line/60 bg-raised hover:bg-sunken/60 active:scale-[0.99] transition-all cursor-pointer text-left select-none outline-none group"
        @click="emit('start-add-offline')"
      >
        <div
          class="w-10 h-10 rounded-xl bg-sunken/60 flex items-center justify-center shrink-0 text-fg-muted group-hover:text-fg transition-colors"
        >
          <span class="i-lucide-user-round w-5 h-5 inline-block" />
        </div>
        <div class="flex flex-col text-left min-w-0 ml-3.5">
          <span class="text-sm font-semibold text-fg">{{ t('accounts.types.offline') }}</span>
          <span class="text-xs text-fg-muted mt-0.5">Standard MD5 / Custom UUID</span>
        </div>
      </button>

      <button
        type="button"
        class="w-full flex items-center px-4 py-3 rounded-xl border border-line/60 bg-raised hover:bg-sunken/60 active:scale-[0.99] transition-all cursor-pointer text-left select-none outline-none group"
        @click="emit('start-add-authlib')"
      >
        <div
          class="w-10 h-10 rounded-xl bg-sunken/60 flex items-center justify-center shrink-0 text-fg-muted group-hover:text-fg transition-colors"
        >
          <span class="i-lucide-shield-check w-5 h-5 inline-block" />
        </div>
        <div class="flex flex-col text-left min-w-0 ml-3.5">
          <span class="text-sm font-semibold text-fg">{{ t('accounts.types.authlib') }}</span>
          <span class="text-xs text-fg-muted mt-0.5">Authlib-Injector / LittleSkin</span>
        </div>
      </button>
    </div>
    <template #footer>
      <MButton variant="text" @click="showAddTypeDialog = false">
        {{ t('common.cancel') }}
      </MButton>
    </template>
  </MDialog>

  <!-- Dialog 2: Add Offline Dialog -->
  <MDialog v-model:open="showAddOfflineDialog" :title="t('accounts.dialog.offlineTitle')">
    <div class="flex flex-col gap-4 py-2">
      <div>
        <label class="text-xs font-medium text-fg-muted mb-1 block">
          {{ t('accounts.dialog.username') }}
        </label>
        <MInput v-model="offlineUsername" :placeholder="t('accounts.dialog.usernamePlaceholder')" />
      </div>

      <div>
        <label class="text-xs font-medium text-fg-muted mb-1 block">
          {{ t('accounts.dialog.uuidMode') }}
        </label>
        <MSelect v-model="offlineUuidMode" :options="uuidModeOptions" />
      </div>

      <div>
        <label class="text-xs font-medium text-fg-muted mb-1 block">
          {{ t('accounts.dialog.skinModel') }}
        </label>
        <MSelect v-model="offlineSkinModel" :options="offlineSkinModelOptions" />
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
      <MButton variant="primary" @click="emit('confirm-add-offline')">
        {{ t('common.confirm') }}
      </MButton>
    </template>
  </MDialog>

  <!-- Dialog 3: Add Authlib Dialog -->
  <MDialog v-model:open="showAddAuthlibDialog" :title="t('accounts.dialog.authlibTitle')">
    <div class="flex flex-col gap-4 py-2">
      <div>
        <label class="text-xs font-medium text-fg-muted mb-1 block">
          {{ t('accounts.dialog.authServer') }}
        </label>
        <MInput v-model="authServer" :placeholder="t('accounts.dialog.authServerPlaceholder')" />
      </div>

      <div>
        <label class="text-xs font-medium text-fg-muted mb-1 block">
          {{ t('accounts.dialog.authAccount') }}
        </label>
        <MInput v-model="authAccount" :placeholder="t('accounts.dialog.authAccountPlaceholder')" />
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
      <MButton variant="primary" :loading="authLoading" @click="emit('confirm-add-authlib')">
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
        <MButton variant="tonal" size="sm" icon="i-lucide-copy" @click="emit('copy-device-code')">
          {{ t('common.copy') }}
        </MButton>
      </div>

      <div class="flex items-center gap-2">
        <MButton variant="primary" icon="i-lucide-external-link" @click="emit('open-device-url')">
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
          @click="emit('select-profile', p)"
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
  <MDialog v-model:open="showSkinUploadDialog" :title="t('accounts.skin.upload')">
    <div class="flex flex-col gap-4 py-2">
      <div>
        <label class="text-xs font-medium text-fg-muted mb-1 block">
          {{ t('accounts.skin.model') }}
        </label>
        <MSelect v-model="skinVariant" :options="skinModelOptions" />
      </div>

      <div>
        <label class="text-xs font-medium text-fg-muted mb-1 block">
          {{ t('accounts.skin.uploadPathPrompt') }}
        </label>
        <MPathPicker
          v-model="skinPngPath"
          type="file"
          :filters="[{ name: 'PNG Image', extensions: ['png'] }]"
          placeholder="skin.png"
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
        @click="emit('confirm-skin-upload')"
      >
        {{ t('common.confirm') }}
      </MButton>
    </template>
  </MDialog>

  <!-- Dialog 7: Rename Offline Account Dialog -->
  <MDialog v-model:open="showRenameDialog" :title="t('accounts.actions.renameTitle')">
    <div class="flex flex-col gap-4 py-2">
      <div>
        <label class="text-xs font-medium text-fg-muted mb-1 block">
          {{ t('accounts.dialog.username') }}
        </label>
        <MInput
          v-model="renameNewUsername"
          :placeholder="t('accounts.dialog.usernamePlaceholder')"
          @keydown.enter="emit('confirm-rename')"
        />
      </div>
    </div>
    <template #footer>
      <MButton variant="text" @click="showRenameDialog = false">
        {{ t('common.cancel') }}
      </MButton>
      <MButton
        variant="primary"
        :loading="renameLoading"
        :disabled="!renameNewUsername.trim()"
        @click="emit('confirm-rename')"
      >
        {{ t('common.confirm') }}
      </MButton>
    </template>
  </MDialog>
</template>

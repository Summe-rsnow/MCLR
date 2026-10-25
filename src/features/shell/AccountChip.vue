<script setup lang="ts">
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useAccountsStore } from '../accounts/store'
import { getAccountKindMeta, MBrandIcon } from '../../ui'

defineProps<{
  collapsed?: boolean
}>()

const { t } = useI18n()
const router = useRouter()
const accountsStore = useAccountsStore()

function navigateToAccounts() {
  if (router?.push) {
    router.push('/accounts')
  }
}
</script>

<template>
  <button
    type="button"
    class="w-full flex items-center gap-2.5 p-2 rounded-xl text-left cursor-pointer transition-colors hover:bg-sunken border border-line"
    :title="accountsStore.selectedAccount?.username ?? t('shell.account.add')"
    @click="navigateToAccounts"
  >
    <!-- Avatar / Skin Head or Fallback Icon -->
    <div
      class="relative w-8 h-8 rounded-lg overflow-hidden bg-sunken flex items-center justify-center shrink-0 border border-line"
    >
      <img
        v-if="accountsStore.selectedAccount?.skinHeadDataUrl"
        :src="accountsStore.selectedAccount.skinHeadDataUrl"
        alt="Avatar"
        class="w-full h-full object-cover [image-rendering:pixelated]"
        loading="lazy"
        decoding="async"
      />
      <span v-else-if="accountsStore.selectedAccount" class="i-lucide-user text-fg-muted w-4 h-4" />
      <span v-else class="i-lucide-user-plus text-fg-muted w-4 h-4" />

      <!-- Kind Badge in Corner if selected -->
      <div
        v-if="accountsStore.selectedAccount"
        class="absolute -bottom-0.5 -right-0.5 w-3.5 h-3.5 rounded-full bg-raised border border-line flex items-center justify-center"
      >
        <MBrandIcon
          v-if="getAccountKindMeta(accountsStore.selectedAccount.kind).brand"
          :name="getAccountKindMeta(accountsStore.selectedAccount.kind).brand!"
          class="w-2.5 h-2.5"
        />
        <span
          v-else
          :class="getAccountKindMeta(accountsStore.selectedAccount.kind).icon"
          class="w-2.5 h-2.5 text-fg-muted"
        />
      </div>
    </div>

    <!-- Account Details (Hidden when collapsed) -->
    <div v-if="!collapsed" class="flex-1 min-w-0">
      <p class="text-xs font-medium text-fg truncate">
        {{
          accountsStore.selectedAccount
            ? accountsStore.selectedAccount.username
            : t('shell.account.add')
        }}
      </p>
      <p
        v-if="accountsStore.selectedAccount?.serverName"
        class="text-[10px] text-fg-muted truncate"
      >
        {{ accountsStore.selectedAccount.serverName }}
      </p>
    </div>
  </button>
</template>

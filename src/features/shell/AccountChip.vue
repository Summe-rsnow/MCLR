<script setup lang="ts">
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useAccountsStore } from '../accounts/store'
import { getAccountKindMeta, MBrandIcon, MImage } from '../../ui'

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
    <div class="relative shrink-0">
      <MImage
        v-if="accountsStore.selectedAccount?.skinHeadDataUrl"
        :src="accountsStore.selectedAccount.skinHeadDataUrl"
        alt="Avatar"
        pixelated
        size="sm"
        fallback-icon="i-lucide-user"
        class="border border-line bg-sunken"
      />
      <div
        v-else
        class="w-8 h-8 rounded-[6px] bg-sunken border border-line flex items-center justify-center"
      >
        <span
          :class="accountsStore.selectedAccount ? 'i-lucide-user' : 'i-lucide-user-plus'"
          class="w-4 h-4 text-fg-muted"
        />
      </div>

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

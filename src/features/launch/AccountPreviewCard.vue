<script setup lang="ts">
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import type { AccountView } from '../../ipc/client'
import { MCard, MAvatar, MBadge, MBrandIcon, MIconButton, getAccountKindMeta } from '../../ui'

const props = defineProps<{
  account: AccountView
}>()

const { t } = useI18n()
const router = useRouter()

const accountMeta = computed(() => {
  return getAccountKindMeta(props.account.kind)
})

function onManageAccounts() {
  router.push('/accounts')
}
</script>

<template>
  <MCard
    variant="well"
    padding="sm"
    class="flex items-center justify-between gap-3"
    data-testid="account-preview-card"
  >
    <div class="flex items-center gap-3 min-w-0 flex-1">
      <MAvatar :name="account.username" :src="account.skinHeadDataUrl || undefined" :size="36" />
      <div class="min-w-0 flex-1">
        <div class="flex items-center gap-1.5">
          <span class="font-semibold text-sm text-fg truncate">
            {{ account.username }}
          </span>
          <MBrandIcon
            v-if="accountMeta.brand"
            :name="accountMeta.brand"
            class="w-3.5 h-3.5 text-primary shrink-0"
          />
          <i v-else :class="[accountMeta.icon, 'text-xs text-fg-muted shrink-0']" />
        </div>
        <div class="flex items-center gap-2 text-xs text-fg-muted mt-0.5">
          <MBadge :variant="account.kind === 'microsoft' ? 'primary' : 'neutral'" size="sm">
            {{ t('accounts.types.' + account.kind) }}
          </MBadge>
          <span v-if="account.serverName" class="truncate"> • {{ account.serverName }} </span>
        </div>
      </div>
    </div>

    <div class="flex items-center gap-1 shrink-0">
      <MIconButton
        icon="i-lucide-user-cog"
        size="sm"
        variant="text"
        :tooltip="t('accounts.nav')"
        :title="t('accounts.nav')"
        data-testid="account-manage-btn"
        @click="onManageAccounts"
      />
    </div>
  </MCard>
</template>

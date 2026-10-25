<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { AccountView } from '../../../ipc/client'
import { MCard, MBadge, MIconButton, MAvatar, MBrandIcon, getAccountKindMeta } from '../../../ui'

defineProps<{
  account: AccountView
  index: number
}>()

const emit = defineEmits<{
  (e: 'select', account: AccountView): void
  (e: 'copy-uuid', account: AccountView): void
  (e: 'rename', account: AccountView): void
  (e: 'refresh', account: AccountView): void
  (e: 'remove', account: AccountView): void
}>()

const { t } = useI18n()
</script>

<template>
  <MCard
    hoverable
    :variant="account.selected ? 'tinted' : 'raised'"
    class="p-4 cursor-pointer m-stagger"
    :style="{ '--i': index }"
    @click="emit('select', account)"
  >
    <div class="flex items-center justify-between gap-4">
      <div class="flex items-center gap-3 min-w-0">
        <MAvatar :src="account.skinHeadDataUrl ?? undefined" :name="account.username" :size="40" />
        <div class="min-w-0">
          <div class="flex items-center gap-2">
            <span class="font-medium text-fg truncate select-text">
              {{ account.username }}
            </span>
            <MBadge :variant="account.kind === 'microsoft' ? 'primary' : 'neutral'" size="sm">
              <MBrandIcon
                v-if="account.kind === 'microsoft'"
                name="microsoft"
                class="w-3.5 h-3.5 mr-1"
              />
              <span
                v-else
                :class="getAccountKindMeta(account.kind).icon"
                class="w-3.5 h-3.5 mr-1 inline-block"
              />
              {{ t(`accounts.types.${account.kind}`) }}
            </MBadge>
            <MBadge v-if="account.selected" variant="success" size="sm">
              {{ t('accounts.status.current') }}
            </MBadge>
          </div>
          <div class="text-xs text-fg-muted truncate mt-0.5 select-text">
            <span v-if="account.serverName">{{ account.serverName }} · </span>
            <span>{{ account.uuid }}</span>
          </div>
        </div>
      </div>

      <div class="flex items-center gap-1 shrink-0" @click.stop>
        <MIconButton
          icon="i-lucide-copy"
          size="sm"
          variant="text"
          :title="t('accounts.actions.copyUuid')"
          @click="emit('copy-uuid', account)"
        />
        <MIconButton
          v-if="account.kind === 'offline'"
          icon="i-lucide-edit-2"
          size="sm"
          variant="text"
          :title="t('accounts.actions.rename')"
          @click="emit('rename', account)"
        />
        <MIconButton
          icon="i-lucide-refresh-cw"
          size="sm"
          variant="text"
          :title="t('accounts.actions.refresh')"
          @click="emit('refresh', account)"
        />
        <MIconButton
          icon="i-lucide-trash-2"
          size="sm"
          variant="text"
          class="text-danger"
          :title="t('accounts.actions.delete')"
          @click="emit('remove', account)"
        />
      </div>
    </div>
  </MCard>
</template>

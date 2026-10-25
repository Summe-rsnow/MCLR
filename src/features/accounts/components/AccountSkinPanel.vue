<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { AccountView } from '../../../ipc/client'
import { MCard, MButton, MAvatar, MEmpty } from '../../../ui'

const props = defineProps<{
  selectedAccount: AccountView | null
}>()

const emit = defineEmits<{
  (e: 'upload-skin'): void
  (e: 'open-closet', server: string): void
  (e: 'select-cape', capeId: string | null): void
}>()

const { t } = useI18n()
const hasActiveCape = computed(() => props.selectedAccount?.capes.some((c) => c.active) ?? false)
</script>

<template>
  <MCard class="p-5 flex flex-col gap-4">
    <h2 class="text-lg font-semibold text-fg">
      {{ t('accounts.skin.title') }}
    </h2>

    <template v-if="selectedAccount">
      <!-- 2D Head Preview -->
      <div class="flex items-center gap-4 p-4 rounded-xl bg-sunken">
        <div
          class="w-16 h-16 rounded-lg bg-raised flex items-center justify-center border border-line overflow-hidden"
        >
          <img
            v-if="selectedAccount.skinHeadDataUrl"
            :src="selectedAccount.skinHeadDataUrl"
            alt="Skin Head"
            class="w-full h-full object-contain image-rendering-pixelated"
            loading="lazy"
            decoding="async"
          />
          <MAvatar v-else :name="selectedAccount.username" :size="64" />
        </div>
        <div class="flex-1 min-w-0">
          <div class="font-medium text-fg truncate select-text">
            {{ selectedAccount.username }}
          </div>
          <div class="text-xs text-fg-muted mt-1 select-text">
            {{ t('accounts.skin.model') }}:
            {{ t(`accounts.skin.models.${selectedAccount.defaultSkinModel || 'classic'}`) }}
          </div>
        </div>
      </div>

      <!-- Skin Actions -->
      <div class="flex flex-col gap-2">
        <MButton
          v-if="selectedAccount.kind !== 'offline'"
          variant="tonal"
          icon="i-lucide-upload"
          @click="emit('upload-skin')"
        >
          {{ t('accounts.skin.upload') }}
        </MButton>
        <MButton
          v-if="selectedAccount.kind === 'authlib' && selectedAccount.server"
          variant="text"
          icon="i-lucide-external-link"
          @click="emit('open-closet', selectedAccount.server)"
        >
          {{ t('accounts.actions.openCloset') }}
        </MButton>
        <p v-if="selectedAccount.kind === 'offline'" class="text-xs text-fg-muted">
          {{ t('accounts.skin.unsupportedOffline') }}
        </p>
      </div>

      <!-- Cape Selection (for accounts that have capes) -->
      <div v-if="selectedAccount.capes.length > 0" class="flex flex-col gap-2 mt-2">
        <div class="text-sm font-medium text-fg">
          {{ t('accounts.skin.capes') }}
        </div>
        <div class="grid grid-cols-2 gap-2">
          <div
            class="p-2 rounded-lg border text-center cursor-pointer text-xs transition"
            :class="[
              !hasActiveCape
                ? 'border-primary bg-primary-soft text-primary font-medium'
                : 'border-line text-fg-muted hover:bg-sunken',
            ]"
            @click="emit('select-cape', null)"
          >
            {{ t('accounts.skin.noneCape') }}
          </div>
          <div
            v-for="cape in selectedAccount.capes"
            :key="cape.id"
            class="p-2 rounded-lg border text-center cursor-pointer text-xs truncate transition"
            :class="[
              cape.active
                ? 'border-primary bg-primary-soft text-primary font-medium'
                : 'border-line text-fg hover:bg-sunken',
            ]"
            @click="emit('select-cape', cape.id)"
          >
            {{ cape.alias }}
          </div>
        </div>
      </div>
    </template>

    <MEmpty v-else :description="t('accounts.skin.noAccount')" />
  </MCard>
</template>

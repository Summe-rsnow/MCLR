<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { UpdateCheckResult } from '../../../ipc/client'
import { MButton, MInput, MSelect, MSwitch, MSection, MSettingRow, MMarkdown } from '../../../ui'

const { t } = useI18n()

defineProps<{
  channel: string
  autoCheck: boolean
  announcementUrl: string
  checkingUpdate: boolean
  installingUpdate: boolean
  updateResult: UpdateCheckResult | null
  updateError: string | null
}>()

const emit = defineEmits<{
  (e: 'update:channel', val: string): void
  (e: 'update:autoCheck', val: boolean): void
  (e: 'update:announcementUrl', val: string): void
  (e: 'check'): void
  (e: 'install'): void
  (e: 'change'): void
}>()

function onUpdateChannel(val: string | number) {
  emit('update:channel', String(val))
  emit('change')
}

function onUpdateAutoCheck(val: boolean) {
  emit('update:autoCheck', val)
  emit('change')
}

function onUpdateAnnouncementUrl(val: string) {
  emit('update:announcementUrl', val)
  emit('change')
}
</script>

<template>
  <MSection
    :title="t('settingsSystem.update.title')"
    :description="t('settingsSystem.update.subtitle')"
    icon="i-lucide-refresh-cw"
  >
    <template #actions>
      <MButton
        variant="primary"
        size="sm"
        icon="i-lucide-search"
        data-testid="check-update-btn"
        :loading="checkingUpdate"
        @click="emit('check')"
      >
        {{ t('settingsSystem.update.checkButton') }}
      </MButton>
    </template>

    <div class="flex flex-col">
      <MSettingRow :label="t('settingsSystem.update.channel')">
        <MSelect
          :model-value="channel"
          :options="[
            { label: t('settingsSystem.update.channelStable'), value: 'stable' },
            { label: t('settingsSystem.update.channelBeta'), value: 'beta' },
          ]"
          size="sm"
          class="w-48"
          @update:model-value="onUpdateChannel"
        />
      </MSettingRow>

      <MSettingRow :label="t('settingsSystem.update.autoCheck')">
        <MSwitch :model-value="autoCheck" @update:model-value="onUpdateAutoCheck" />
      </MSettingRow>

      <MSettingRow :label="t('settingsSystem.update.announcementUrl')">
        <div class="w-80">
          <MInput
            :model-value="announcementUrl"
            size="sm"
            :placeholder="t('settingsSystem.update.announcementUrlPlaceholder')"
            @update:model-value="onUpdateAnnouncementUrl"
          />
        </div>
      </MSettingRow>

      <div
        v-if="updateError"
        data-testid="update-error"
        class="p-3 mt-3 rounded-xl bg-danger-soft text-danger-fg text-xs border border-line"
      >
        {{ updateError }}
      </div>

      <div
        v-if="updateResult"
        data-testid="update-result"
        class="p-4 mt-3 rounded-xl border border-line bg-sunken/40 space-y-3"
      >
        <div v-if="updateResult.available" class="space-y-2">
          <div class="flex items-center justify-between">
            <span class="font-semibold text-primary text-sm">
              {{ t('settingsSystem.update.available', { version: updateResult.version ?? '' }) }}
            </span>
            <MButton
              variant="primary"
              size="sm"
              data-testid="install-update-btn"
              :loading="installingUpdate"
              @click="emit('install')"
            >
              {{ t('settingsSystem.update.installButton') }}
            </MButton>
          </div>
          <div v-if="updateResult.date" class="text-xs text-fg-muted">
            {{ t('settingsSystem.update.releaseDate', { date: updateResult.date }) }}
          </div>
          <MMarkdown
            v-if="updateResult.body"
            :source="updateResult.body"
            class="prose text-xs text-fg mt-2"
          />
        </div>
        <div v-else class="text-xs text-success font-medium">
          {{ t('settingsSystem.update.upToDate', { version: updateResult.currentVersion }) }}
        </div>
      </div>
    </div>
  </MSection>
</template>

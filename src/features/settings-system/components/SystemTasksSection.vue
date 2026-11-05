<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { MSlider, MSwitch, MSection, MSettingRow } from '../../../ui'

const { t } = useI18n()

defineProps<{
  maxConcurrent: number
  autoRetry: number
  notify: boolean
  historyMax: number
  historyDays: number
}>()

const emit = defineEmits<{
  (e: 'update:maxConcurrent', val: number): void
  (e: 'update:autoRetry', val: number): void
  (e: 'update:notify', val: boolean): void
  (e: 'update:historyMax', val: number): void
  (e: 'update:historyDays', val: number): void
  (e: 'change'): void
}>()

function onUpdateMaxConcurrent(val: number) {
  emit('update:maxConcurrent', val)
  emit('change')
}

function onUpdateAutoRetry(val: number) {
  emit('update:autoRetry', val)
  emit('change')
}

function onUpdateNotify(val: boolean) {
  emit('update:notify', val)
  emit('change')
}

function onUpdateHistoryMax(val: number) {
  emit('update:historyMax', val)
  emit('change')
}

function onUpdateHistoryDays(val: number) {
  emit('update:historyDays', val)
  emit('change')
}
</script>

<template>
  <MSection
    :title="t('tasks.settings.title')"
    :description="t('tasks.settings.desc')"
    icon="i-lucide-list-checks"
  >
    <div class="flex flex-col">
      <MSettingRow
        :label="t('tasks.settings.maxConcurrent')"
        :hint="t('tasks.settings.maxConcurrentDesc')"
      >
        <div class="w-48 flex items-center gap-3">
          <MSlider
            :model-value="maxConcurrent"
            :min="1"
            :max="8"
            :step="1"
            class="flex-1"
            @update:model-value="onUpdateMaxConcurrent"
          />
          <span class="text-xs font-mono w-6 text-right text-fg-muted">{{ maxConcurrent }}</span>
        </div>
      </MSettingRow>

      <MSettingRow :label="t('tasks.settings.autoRetry')" :hint="t('tasks.settings.autoRetryDesc')">
        <div class="w-48 flex items-center gap-3">
          <MSlider
            :model-value="autoRetry"
            :min="0"
            :max="10"
            :step="1"
            class="flex-1"
            @update:model-value="onUpdateAutoRetry"
          />
          <span class="text-xs font-mono w-6 text-right text-fg-muted">{{ autoRetry }}</span>
        </div>
      </MSettingRow>

      <MSettingRow :label="t('tasks.settings.notify')" :hint="t('tasks.settings.notifyDesc')">
        <MSwitch :model-value="notify" @update:model-value="onUpdateNotify" />
      </MSettingRow>

      <MSettingRow
        :label="t('tasks.settings.historyMax')"
        :hint="t('tasks.settings.historyMaxDesc')"
      >
        <div class="w-48 flex items-center gap-3">
          <MSlider
            :model-value="historyMax"
            :min="20"
            :max="2000"
            :step="20"
            class="flex-1"
            @update:model-value="onUpdateHistoryMax"
          />
          <span class="text-xs font-mono w-8 text-right text-fg-muted">{{ historyMax }}</span>
        </div>
      </MSettingRow>

      <MSettingRow
        :label="t('tasks.settings.historyDays')"
        :hint="t('tasks.settings.historyDaysDesc')"
      >
        <div class="w-48 flex items-center gap-3">
          <MSlider
            :model-value="historyDays"
            :min="1"
            :max="365"
            :step="1"
            class="flex-1"
            @update:model-value="onUpdateHistoryDays"
          />
          <span class="text-xs font-mono w-8 text-right text-fg-muted">{{ historyDays }}</span>
        </div>
      </MSettingRow>
    </div>
  </MSection>
</template>

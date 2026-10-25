<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { MInput, MSelect, MSlider, MSwitch, MSection, MSettingRow, MHelpTooltip } from '../../../ui'

const { t } = useI18n()

defineProps<{
  proxyMode: 'none' | 'system' | 'custom'
  proxyUrl: string
  sourcePriority: 'mirrorFirst' | 'smart' | 'officialFirst'
  doh: boolean
  connectionsPerFile: number
  maxConcurrentFiles: number
}>()

const emit = defineEmits<{
  (e: 'update:proxyMode', val: 'none' | 'system' | 'custom'): void
  (e: 'update:proxyUrl', val: string): void
  (e: 'update:sourcePriority', val: 'mirrorFirst' | 'smart' | 'officialFirst'): void
  (e: 'update:doh', val: boolean): void
  (e: 'update:connectionsPerFile', val: number): void
  (e: 'update:maxConcurrentFiles', val: number): void
  (e: 'change'): void
}>()

function onUpdateProxyMode(val: string | number) {
  emit('update:proxyMode', val as 'none' | 'system' | 'custom')
  emit('change')
}

function onUpdateProxyUrl(val: string) {
  emit('update:proxyUrl', val)
  emit('change')
}

function onUpdateSourcePriority(val: string | number) {
  emit('update:sourcePriority', val as 'mirrorFirst' | 'smart' | 'officialFirst')
  emit('change')
}

function onUpdateDoh(val: boolean) {
  emit('update:doh', val)
  emit('change')
}

function onUpdateConnections(val: number) {
  emit('update:connectionsPerFile', val)
  emit('change')
}

function onUpdateConcurrent(val: number) {
  emit('update:maxConcurrentFiles', val)
  emit('change')
}
</script>

<template>
  <MSection
    :title="t('settingsSystem.misc.networkTitle')"
    :description="t('settingsSystem.misc.networkDesc')"
    icon="i-lucide-network"
  >
    <div class="flex flex-col">
      <MSettingRow :label="t('settingsSystem.misc.proxyMode')">
        <MSelect
          :model-value="proxyMode"
          :options="[
            { label: t('settingsSystem.misc.proxyModeNone'), value: 'none' },
            { label: t('settingsSystem.misc.proxyModeSystem'), value: 'system' },
            { label: t('settingsSystem.misc.proxyModeCustom'), value: 'custom' },
          ]"
          size="sm"
          class="w-48"
          @update:model-value="onUpdateProxyMode"
        />
      </MSettingRow>

      <MSettingRow v-if="proxyMode === 'custom'" :label="t('settingsSystem.misc.proxyUrl')">
        <div class="w-80">
          <MInput
            :model-value="proxyUrl"
            size="sm"
            :placeholder="t('settingsSystem.misc.proxyUrlPlaceholder')"
            @update:model-value="onUpdateProxyUrl"
          />
        </div>
      </MSettingRow>

      <MSettingRow :label="t('settingsSystem.misc.sourcePriority')">
        <MSelect
          :model-value="sourcePriority"
          :options="[
            { label: t('settingsSystem.misc.sourcePriorityMirrorFirst'), value: 'mirrorFirst' },
            { label: t('settingsSystem.misc.sourcePrioritySmart'), value: 'smart' },
            {
              label: t('settingsSystem.misc.sourcePriorityOfficialFirst'),
              value: 'officialFirst',
            },
          ]"
          size="sm"
          class="w-48"
          @update:model-value="onUpdateSourcePriority"
        />
      </MSettingRow>

      <MSettingRow :label="t('settingsSystem.misc.doh')">
        <template #label>
          <span>{{ t('settingsSystem.misc.doh') }}</span>
          <MHelpTooltip :text="t('settingsSystem.misc.dohDesc')" />
        </template>
        <MSwitch :model-value="doh" @update:model-value="onUpdateDoh" />
      </MSettingRow>

      <MSettingRow :label="t('settingsSystem.misc.connectionsPerFile', { n: connectionsPerFile })">
        <div class="w-64">
          <MSlider
            :model-value="connectionsPerFile"
            :min="1"
            :max="64"
            @update:model-value="onUpdateConnections"
          />
        </div>
      </MSettingRow>

      <MSettingRow :label="t('settingsSystem.misc.maxConcurrentFiles', { n: maxConcurrentFiles })">
        <div class="w-64">
          <MSlider
            :model-value="maxConcurrentFiles"
            :min="1"
            :max="64"
            @update:model-value="onUpdateConcurrent"
          />
        </div>
      </MSettingRow>
    </div>
  </MSection>
</template>

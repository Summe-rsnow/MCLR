<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { InstanceInfo } from '../../../ipc/client'
import { usePresetsStore } from '../../presets/store'
import { MBadge, MButton, MCard, MEmpty } from '../../../ui'
import PresetApplyDialog from '../../presets/components/PresetApplyDialog.vue'
import ApplyReportDialog from '../../presets/components/ApplyReportDialog.vue'

const props = defineProps<{
  instanceId: string
  instance?: InstanceInfo
}>()

const { t } = useI18n()
const presetsStore = usePresetsStore()

const showApplyDialog = ref(false)
const showReportDialog = ref(false)

const lastReport = computed(() => presetsStore.lastReports[props.instanceId])

const reportSummary = computed(() => {
  if (!lastReport.value) return null
  const rows = lastReport.value.rows
  return {
    installed: rows.filter((r) => r.status === 'installed').length,
    replaced: rows.filter((r) => r.status === 'replaced').length,
    alreadySame: rows.filter((r) => r.status === 'alreadySame').length,
    alreadyOtherVersion: rows.filter((r) => r.status === 'alreadyOtherVersion').length,
    failed: rows.filter((r) => r.status === 'failed').length,
    total: rows.length,
  }
})

function onApplied() {
  showApplyDialog.value = false
  if (lastReport.value) {
    showReportDialog.value = true
  }
}
</script>

<template>
  <div class="space-y-4">
    <div class="flex items-center justify-between">
      <div>
        <h3 class="text-base font-medium text-fg">
          {{ t('instanceDetail.tabs.presets') }}
        </h3>
        <p class="text-xs text-fg-muted">
          {{ t('instanceDetail.presets.emptyHint') }}
        </p>
      </div>
      <MButton variant="primary" @click="showApplyDialog = true">
        <template #icon>
          <span class="i-lucide-package-plus" />
        </template>
        {{ t('instanceDetail.presets.apply') }}
      </MButton>
    </div>

    <div v-if="lastReport && reportSummary">
      <MCard class="p-4 space-y-3">
        <div class="flex items-center justify-between">
          <div class="font-medium text-sm text-fg">
            {{ t('instanceDetail.presets.lastReport') }}
          </div>
          <MButton size="sm" variant="outline" @click="showReportDialog = true">
            {{ t('instanceDetail.presets.openReport') }}
          </MButton>
        </div>
        <div class="flex flex-wrap gap-2 text-xs">
          <MBadge v-if="reportSummary.installed > 0" variant="success">
            {{ t('presets.report.summary.installed') }}: {{ reportSummary.installed }}
          </MBadge>
          <MBadge v-if="reportSummary.replaced > 0" variant="info">
            {{ t('presets.report.summary.replaced') }}: {{ reportSummary.replaced }}
          </MBadge>
          <MBadge v-if="reportSummary.alreadySame > 0" variant="neutral">
            {{ t('presets.report.summary.alreadySame') }}: {{ reportSummary.alreadySame }}
          </MBadge>
          <MBadge v-if="reportSummary.alreadyOtherVersion > 0" variant="warning">
            {{ t('presets.report.summary.alreadyOtherVersion') }}:
            {{ reportSummary.alreadyOtherVersion }}
          </MBadge>
          <MBadge v-if="reportSummary.failed > 0" variant="danger">
            {{ t('presets.report.summary.failed') }}: {{ reportSummary.failed }}
          </MBadge>
        </div>
      </MCard>
    </div>

    <MEmpty v-else :description="t('instanceDetail.presets.empty')">
      <template #action>
        <MButton variant="primary" @click="showApplyDialog = true">
          {{ t('instanceDetail.presets.apply') }}
        </MButton>
      </template>
    </MEmpty>

    <PresetApplyDialog
      v-model:open="showApplyDialog"
      :instance-id="instanceId"
      @applied="onApplied"
    />

    <ApplyReportDialog v-model:open="showReportDialog" :report="lastReport ?? null" />
  </div>
</template>

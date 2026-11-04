<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter, type Router } from 'vue-router'
import type { ApplyReport, ApplyRow, ApplyStatus } from '../../../ipc/client'
import { MBadge, MButton, MDialog, MEmpty, MImage, MBrandIcon, compTypeMeta } from '../../../ui'

const props = defineProps<{
  open: boolean
  report: ApplyReport | null
}>()

const emit = defineEmits<{
  (e: 'update:open', val: boolean): void
}>()

const { t, te } = useI18n()
// Overlays can render outside a router context (unit tests, preview harness); navigation then
// degrades to a no-op instead of throwing.
const router = useRouter() as Router | undefined

const STATUS_ORDER: ApplyStatus[] = [
  'installed',
  'replaced',
  'alreadySame',
  'alreadyOtherVersion',
  'noGameVersion',
  'noLoader',
  'noFile',
  'needsWorld',
  'distributionDisabled',
  'failed',
]

const SUMMARY_STATUSES: ApplyStatus[] = [
  'installed',
  'replaced',
  'alreadySame',
  'alreadyOtherVersion',
  'noGameVersion',
  'noLoader',
  'needsWorld',
  'distributionDisabled',
  'failed',
]

const STATUS_ICON: Record<ApplyStatus, string> = {
  installed: 'i-lucide-circle-check',
  replaced: 'i-lucide-refresh-cw',
  alreadySame: 'i-lucide-equal',
  alreadyOtherVersion: 'i-lucide-git-compare',
  noGameVersion: 'i-lucide-calendar-x',
  noLoader: 'i-lucide-puzzle',
  noFile: 'i-lucide-file-x',
  needsWorld: 'i-lucide-globe',
  distributionDisabled: 'i-lucide-ban',
  failed: 'i-lucide-x-circle',
}

const STATUS_VARIANT: Record<
  ApplyStatus,
  'success' | 'primary' | 'warning' | 'danger' | 'neutral'
> = {
  installed: 'success',
  replaced: 'primary',
  alreadySame: 'neutral',
  alreadyOtherVersion: 'warning',
  noGameVersion: 'warning',
  noLoader: 'warning',
  noFile: 'warning',
  needsWorld: 'warning',
  distributionDisabled: 'danger',
  failed: 'danger',
}

const rows = computed(() => props.report?.rows ?? [])

const groups = computed(() =>
  STATUS_ORDER.map((status) => ({
    status,
    rows: rows.value.filter((row) => row.status === status),
  })).filter((group) => group.rows.length > 0),
)

const failedGates = computed(() => (props.report?.gates ?? []).filter((gate) => !gate.ok))

function statusCount(status: ApplyStatus): number {
  return rows.value.filter((row) => row.status === status).length
}

function statusLabel(status: ApplyStatus): string {
  switch (status) {
    case 'installed':
      return t('presets.report.status.installed')
    case 'replaced':
      return t('presets.report.status.replaced')
    case 'alreadySame':
      return t('presets.report.status.alreadySame')
    case 'alreadyOtherVersion':
      return t('presets.report.status.alreadyOtherVersion')
    case 'noGameVersion':
      return t('presets.report.status.noGameVersion')
    case 'noLoader':
      return t('presets.report.status.noLoader')
    case 'noFile':
      return t('presets.report.status.noFile')
    case 'needsWorld':
      return t('presets.report.status.needsWorld')
    case 'distributionDisabled':
      return t('presets.report.status.distributionDisabled')
    case 'failed':
      return t('presets.report.status.failed')
    default:
      return status
  }
}

function summaryLabel(status: ApplyStatus): string {
  switch (status) {
    case 'installed':
      return t('presets.report.summary.installed')
    case 'replaced':
      return t('presets.report.summary.replaced')
    case 'alreadySame':
      return t('presets.report.summary.alreadySame')
    case 'alreadyOtherVersion':
      return t('presets.report.summary.alreadyOtherVersion')
    case 'noGameVersion':
      return t('presets.report.summary.noGameVersion')
    case 'noLoader':
      return t('presets.report.summary.noLoader')
    case 'needsWorld':
      return t('presets.report.summary.needsWorld')
    case 'distributionDisabled':
      return t('presets.report.summary.distributionDisabled')
    case 'failed':
      return t('presets.report.summary.failed')
    default:
      return statusLabel(status)
  }
}

function linkModeLabel(mode: ApplyRow['linkMode']): string {
  switch (mode) {
    case 'hardLink':
      return t('presets.report.linkMode.hardLink')
    case 'copy':
      return t('presets.report.linkMode.copy')
    case 'extracted':
      return t('presets.report.linkMode.extracted')
    default:
      return ''
  }
}

function messageText(code: string | null | undefined): string {
  if (!code) return t('error.unknown')
  const key = `error.${code}`
  return te(key) ? t(key) : code
}

function overlapText(row: ApplyRow): string {
  return t('presets.report.overlap', {
    presets: row.fromPresets.join(t('presets.report.separator')),
  })
}

function openProject(row: ApplyRow): void {
  void router?.push(`/comp/project/${row.platform}/${row.projectId}`)
  emit('update:open', false)
}
</script>

<template>
  <MDialog
    :open="open"
    :title="t('presets.report.title')"
    :hide-footer="true"
    max-width="xl"
    @update:open="emit('update:open', $event)"
  >
    <div class="flex flex-col gap-4 max-h-[70vh] overflow-y-auto pr-1" data-testid="apply-report">
      <div v-if="failedGates.length > 0" class="flex flex-col gap-1.5" data-testid="gate-warnings">
        <div
          v-for="gate in failedGates"
          :key="gate.presetId"
          class="flex items-start gap-2 rounded-xl border border-warning/40 bg-warning-soft px-3 py-2 text-xs text-warning-fg"
        >
          <span class="i-lucide-triangle-alert w-4 h-4 shrink-0 mt-0.5" />
          <span>{{ t('presets.report.gateSkipped', { name: gate.name }) }}</span>
        </div>
      </div>

      <div v-if="rows.length > 0" class="flex flex-wrap gap-1.5">
        <template v-for="status in SUMMARY_STATUSES" :key="status">
          <MBadge
            v-if="statusCount(status) > 0"
            :variant="STATUS_VARIANT[status]"
            size="sm"
            :icon="STATUS_ICON[status]"
            :data-testid="`summary-${status}`"
          >
            {{ summaryLabel(status) }} {{ statusCount(status) }}
          </MBadge>
        </template>
      </div>

      <MEmpty
        v-if="rows.length === 0"
        :title="t('presets.report.empty')"
        icon="i-lucide-package-check"
      />

      <div v-else class="flex flex-col gap-4">
        <div v-for="group in groups" :key="group.status" class="flex flex-col gap-1.5">
          <div class="flex items-center gap-2 text-xs font-semibold text-fg-muted">
            <span :class="STATUS_ICON[group.status]" class="w-3.5 h-3.5" />
            <span>{{ statusLabel(group.status) }}</span>
            <span class="text-fg-subtle">({{ group.rows.length }})</span>
          </div>

          <div class="flex flex-col gap-1.5">
            <div
              v-for="row in group.rows"
              :key="`${row.platform}:${row.projectId}:${row.status}`"
              class="flex flex-col gap-1.5 rounded-xl border border-line px-3 py-2 transition-colors"
              :class="{
                'hover:bg-sunken cursor-pointer': row.status !== 'failed',
              }"
              data-testid="report-row"
              @click="openProject(row)"
            >
              <div class="flex items-center gap-2 min-w-0">
                <MImage
                  :src="null"
                  :alt="row.title"
                  :fallback-icon="compTypeMeta[row.kind]?.icon"
                  class="w-7 h-7 rounded-md shrink-0"
                />
                <div class="min-w-0 flex-1">
                  <div class="flex items-center gap-1.5 min-w-0">
                    <span class="text-sm font-medium text-fg truncate">{{ row.title }}</span>
                    <MBrandIcon :name="row.platform" class="w-3 h-3 text-fg-muted" />
                  </div>
                  <div v-if="row.zhTitle" class="text-xs text-fg-subtle truncate">
                    {{ row.zhTitle }}
                  </div>
                </div>
                <MBadge :variant="STATUS_VARIANT[row.status]" size="sm">
                  {{ statusLabel(row.status) }}
                </MBadge>
                <span class="text-xs text-fg-subtle whitespace-nowrap">
                  {{ t('presets.kinds.' + row.kind) }}
                </span>
              </div>

              <div class="flex items-center gap-1.5 flex-wrap text-xs">
                <MBadge v-if="row.overlap" variant="info" size="sm" icon="i-lucide-copy">
                  {{ overlapText(row) }}
                </MBadge>
                <MBadge
                  v-if="row.versionConflict"
                  variant="warning"
                  size="sm"
                  icon="i-lucide-triangle-alert"
                >
                  {{
                    t('presets.report.versionConflict', {
                      preset: row.fromPresets[0] ?? '',
                    })
                  }}
                </MBadge>
                <button
                  v-if="row.viaDependencyOf"
                  type="button"
                  class="inline-flex items-center gap-1 text-xs text-primary hover:underline cursor-pointer"
                  data-testid="via-dependency"
                  @click.stop="openProject(row)"
                >
                  <span class="i-lucide-link-2 w-3 h-3" />
                  {{ t('presets.report.viaDependency', { via: row.viaDependencyOf }) }}
                </button>
                <span v-if="row.fileName" class="text-fg-muted truncate">
                  {{ row.fileName }}
                </span>
                <span v-if="row.linkMode" class="text-fg-subtle">
                  {{ linkModeLabel(row.linkMode) }}
                </span>
              </div>

              <div v-if="row.installedFileName" class="text-xs text-fg-subtle">
                {{
                  t('presets.report.existingFile', {
                    name: row.installedFileName,
                  })
                }}
              </div>
              <div v-if="row.status === 'failed'" class="text-xs text-danger-fg">
                {{
                  t('presets.report.failedCode', {
                    message: messageText(row.messageCode),
                  })
                }}
              </div>
            </div>
          </div>
        </div>
      </div>

      <div class="flex items-center justify-end pt-1">
        <MButton variant="tonal" icon="i-lucide-check" @click="emit('update:open', false)">
          {{ t('common.close') }}
        </MButton>
      </div>
    </div>
  </MDialog>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { save as saveFileDialog } from '@tauri-apps/plugin-dialog'
import { useCrashStore } from './store'
import type { CrashConfidence } from '../../ipc/client'
import {
  MDialog,
  MButton,
  MBadge,
  useToast,
} from '../../ui'

const { t, te } = useI18n()
const router = useRouter()
const toast = useToast()
const crashStore = useCrashStore()

const expandedEvidence = ref<Record<number, boolean>>({})

function toggleEvidence(index: number) {
  expandedEvidence.value[index] = !expandedEvidence.value[index]
}

function getConfidenceVariant(conf: CrashConfidence): 'danger' | 'warning' | 'primary' {
  switch (conf) {
    case 'high':
      return 'danger'
    case 'medium':
      return 'warning'
    case 'low':
      return 'primary'
    default:
      return 'primary'
  }
}

function getCauseTitle(code: string, params: Record<string, string>): string {
  const key = `crash.cause.${code}.title`
  return te(key) ? t(key, params) : code
}

function getCauseAdvice(code: string, params: Record<string, string>): string {
  const key = `crash.cause.${code}.advice`
  return te(key) ? t(key, params) : ''
}

async function onExportReport() {
  const instanceId = crashStore.currentInstanceId || 'instance'
  const defaultName = `crash-report-${instanceId}-${new Date().toISOString().replace(/[:.]/g, '-')}.zip`
  let chosenPath = defaultName

  try {
    const selected = await saveFileDialog({
      defaultPath: defaultName,
      filters: [
        {
          name: 'Crash Report (*.zip)',
          extensions: ['zip'],
        },
      ],
    })
    if (selected && typeof selected === 'string') {
      chosenPath = selected
    } else if (!selected) {
      // User cancelled file selection
      return
    }
  } catch {
    // Non-tauri browser mode fallback
  }

  try {
    await crashStore.exportReport(chosenPath)
    toast.success(t('crash.dialog.actions.exportSuccess'))
  } catch (e: unknown) {
    toast.danger(String(e))
  }
}

function onOpenLogs() {
  crashStore.closeDialog()
  router.push('/logs')
}

function onClose() {
  crashStore.closeDialog()
}
</script>
<template>
  <MDialog
    :open="crashStore.isOpen"
    :title="t('crash.dialog.title')"
    @update:open="!$event && onClose()"
  >
    <div class="space-y-4 max-h-[70vh] overflow-y-auto pr-1">
      <!-- Subtitle banner -->
      <div class="flex items-center gap-3 p-3 rounded-xl bg-danger-soft text-danger-fg border border-danger/20">
        <span class="w-5 h-5 i-lucide-alert-triangle shrink-0" />
        <div class="text-xs">
          <div class="font-semibold">{{ t('crash.dialog.subtitle') }}</div>
          <div class="flex flex-wrap items-center gap-x-4 mt-0.5 text-fg-muted">
            <span v-if="crashStore.currentInstanceId">
              {{ t('crash.dialog.instance', { name: crashStore.currentInstanceId }) }}
            </span>
            <span v-if="crashStore.currentResult?.analyzedAt">
              {{ t('crash.dialog.analyzedAt', { time: new Date(crashStore.currentResult.analyzedAt).toLocaleTimeString() }) }}
            </span>
          </div>
        </div>
      </div>

      <!-- Causes list -->
      <div
        v-if="crashStore.currentResult && crashStore.currentResult.causes.length > 0"
        class="space-y-3"
      >
        <div
          v-for="(cause, index) in crashStore.currentResult.causes"
          :key="index"
          class="p-4 rounded-xl border border-line bg-sunken space-y-2.5"
        >
          <div class="flex items-center justify-between gap-2">
            <h3 class="text-sm font-semibold text-fg flex items-center gap-2">
              <span class="w-2 h-2 rounded-full bg-danger" />
              {{ getCauseTitle(cause.code, cause.params) }}
            </h3>
            <MBadge
              :variant="getConfidenceVariant(cause.confidence)"
              size="sm"
            >
              {{ t('crash.dialog.confidence.' + cause.confidence) }}
            </MBadge>
          </div>

          <p class="text-xs text-fg-muted leading-relaxed">
            {{ getCauseAdvice(cause.code, cause.params) }}
          </p>

          <!-- Evidence lines -->
          <div v-if="cause.evidence && cause.evidence.length > 0" class="pt-1">
            <button
              type="button"
              class="text-xs text-primary hover:underline flex items-center gap-1 cursor-pointer"
              @click="toggleEvidence(index)"
            >
              <span
                class="w-3.5 h-3.5 transition-transform"
                :class="expandedEvidence[index] ? 'i-lucide-chevron-down' : 'i-lucide-chevron-right'"
              />
              <span>
                {{ expandedEvidence[index] ? t('crash.dialog.evidence.toggle') : t('crash.dialog.evidence.lines', { count: cause.evidence.length }) }}
              </span>
            </button>
            <div
              v-if="expandedEvidence[index]"
              class="mt-2 p-2.5 rounded-lg m-well font-mono text-[11px] leading-tight overflow-x-auto max-h-40 select-text"
            >
              <div
                v-for="(line, lIdx) in cause.evidence"
                :key="lIdx"
                class="whitespace-pre-wrap py-0.5 text-danger-fg"
              >
                {{ line }}
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Fallback when no causes detected -->
      <div
        v-else
        class="p-6 rounded-xl border border-line bg-sunken text-center space-y-2"
      >
        <span class="w-8 h-8 i-lucide-help-circle mx-auto text-fg-subtle block" />
        <h3 class="text-sm font-semibold text-fg">
          {{ t('crash.dialog.noCausesFoundTitle') }}
        </h3>
        <p class="text-xs text-fg-muted max-w-md mx-auto">
          {{ t('crash.dialog.noCausesFoundAdvice') }}
        </p>
      </div>
    </div>

    <!-- Footer Actions -->
    <template #footer>
      <div class="flex flex-wrap items-center justify-between gap-3 w-full">
        <div class="flex items-center gap-2">
          <MButton
            variant="tonal"
            size="sm"
            icon="i-lucide-file-archive"
            :loading="crashStore.isExporting"
            @click="onExportReport"
          >
            {{ t('crash.dialog.actions.exportReport') }}
          </MButton>
          <MButton
            variant="tonal"
            size="sm"
            icon="i-lucide-terminal"
            @click="onOpenLogs"
          >
            {{ t('crash.dialog.actions.openLogs') }}
          </MButton>
        </div>

        <MButton
          variant="primary"
          size="sm"
          @click="onClose"
        >
          {{ t('crash.dialog.actions.close') }}
        </MButton>
      </div>
    </template>
  </MDialog>
</template>

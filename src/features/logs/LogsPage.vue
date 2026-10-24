<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useLogsStore } from './store'
import { useToast } from '../../ui/useToast'
import { useDialog } from '../../ui/useDialog'
import { useCrashStore } from '../crash/store'
import type { LogLevel } from '../../ipc/client'
import {
  MPageHeader,
  MButton,
  MInput,
  MSegmented,
  MSwitch,
  MSelect,
  MBadge,
  MCard,
} from '../../ui'

const { t } = useI18n()
const toast = useToast()
const dialog = useDialog()
const logsStore = useLogsStore()
const crashStore = useCrashStore()

async function onAnalyzeCrash() {
  const instanceId = logsStore.selectedInstanceId || '1.20.1-vanilla'
  await crashStore.analyzeInstance(instanceId, null, true)
}

const logContainer = ref<HTMLElement | null>(null)
const isKilling = ref(false)

const levelOptions = [
  { label: 'logs.levels.all', value: 'all' },
  { label: 'logs.levels.info', value: 'info' },
  { label: 'logs.levels.warn', value: 'warn' },
  { label: 'logs.levels.error', value: 'error' },
  { label: 'logs.levels.debug', value: 'debug' },
]

const processOptions = computed(() => {
  const options = [{ label: t('logs.allProcesses'), value: 'all' }]
  for (const proc of logsStore.runningProcesses) {
    options.push({
      label: t('logs.processItem', {
        instance: proc.instanceId,
        pid: proc.processId,
      }),
      value: String(proc.processId),
    })
  }
  return options
})

// Virtualized / windowed viewport calculation
const ROW_HEIGHT = 24
const scrollTop = ref(0)
const viewportHeight = ref(600)
const overscan = 20

const totalCount = computed(() => logsStore.filteredLogs.length)
const totalHeight = computed(() => totalCount.value * ROW_HEIGHT)

const startIndex = computed(() => {
  return Math.max(0, Math.floor(scrollTop.value / ROW_HEIGHT) - overscan)
})

const endIndex = computed(() => {
  const visibleCount = Math.ceil(viewportHeight.value / ROW_HEIGHT)
  return Math.min(
    totalCount.value,
    Math.floor(scrollTop.value / ROW_HEIGHT) + visibleCount + overscan,
  )
})

const visibleLogs = computed(() => {
  return logsStore.filteredLogs
    .slice(startIndex.value, endIndex.value)
    .map((line, idx) => ({
      ...line,
      top: (startIndex.value + idx) * ROW_HEIGHT,
      lineId: `${line.timestamp}-${startIndex.value + idx}`,
    }))
})

function onScroll(e: Event) {
  const target = e.target as HTMLElement
  scrollTop.value = target.scrollTop
  // If user scrolled up significantly, autoScroll should ideally pause or stay as-is
}

function scrollToBottom() {
  if (!logsStore.autoScroll || !logContainer.value) return
  nextTick(() => {
    if (logContainer.value) {
      logContainer.value.scrollTop = logContainer.value.scrollHeight
    }
  })
}

watch(
  () => logsStore.logs.length,
  () => {
    scrollToBottom()
  },
)

onMounted(async () => {
  await logsStore.fetchLogs()
  await logsStore.startListening()
  if (logContainer.value) {
    viewportHeight.value = logContainer.value.clientHeight || 600
  }
  scrollToBottom()
})

onUnmounted(() => {
  logsStore.stopListening()
})

async function onCopyLogs() {
  try {
    const text = logsStore.filteredLogs.map((l) => l.text).join('\n')
    await navigator.clipboard.writeText(text)
    toast.show({
      message: t('logs.copied'),
      type: 'success',
    })
  } catch (err: unknown) {
    toast.show({
      message: String(err),
      type: 'danger',
    })
  }
}

async function onExportLogs() {
  try {
    const text = logsStore.filteredLogs.map((l) => l.text).join('\n')
    const blob = new Blob([text], { type: 'text/plain;charset=utf-8' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `mclr-log-${Date.now()}.txt`
    a.click()
    URL.revokeObjectURL(url)
    toast.show({
      message: t('logs.exported'),
      type: 'success',
    })
  } catch (err: unknown) {
    toast.show({
      message: String(err),
      type: 'danger',
    })
  }
}

async function onKillClick() {
  const proc = logsStore.activeProcess
  if (!proc) return

  const confirmed = await dialog.confirm({
    title: t('logs.killConfirmTitle'),
    content: t('logs.killConfirmMessage', { pid: proc.processId }),
    confirmText: t('common.confirm'),
    cancelText: t('common.cancel'),
  })

  if (!confirmed) return

  isKilling.value = true
  try {
    await logsStore.killRunningProcess(proc.processId)
    toast.show({
      message: t('common.save'),
      type: 'success',
    })
  } catch (err: unknown) {
    toast.show({
      message: String(err),
      type: 'danger',
    })
  } finally {
    isKilling.value = false
  }
}

function getLevelBadgeVariant(level: LogLevel): 'primary' | 'success' | 'warning' | 'danger' | 'neutral' {
  switch (level.toLowerCase()) {
    case 'error':
    case 'fatal':
      return 'danger'
    case 'warn':
      return 'warning'
    case 'debug':
      return 'neutral'
    default:
      return 'primary'
  }
}

function getLineTextColor(level: LogLevel) {
  switch (level.toLowerCase()) {
    case 'error':
    case 'fatal':
      return 'text-danger'
    case 'warn':
      return 'text-warning'
    case 'debug':
      return 'text-fg-muted'
    default:
      return 'text-fg'
  }
}
</script>

<template>
  <div class="m-page max-w-none h-full">
    <!-- Header -->
    <MPageHeader
      :title="t('logs.title')"
      :description="logsStore.activeProcess ? `${t('logs.status.running')} (PID: ${logsStore.activeProcess.processId})` : undefined"
      icon="i-lucide-terminal"
    >
      <template #actions>
        <div class="flex flex-wrap items-center gap-2">
          <MButton
            variant="tonal"
            size="sm"
            icon="i-lucide-copy"
            @click="onCopyLogs"
          >
            {{ t('common.copy') }}
          </MButton>
          <MButton
            variant="tonal"
            size="sm"
            icon="i-lucide-download"
            @click="onExportLogs"
          >
            {{ t('logs.export') }}
          </MButton>
          <MButton
            variant="text"
            size="sm"
            icon="i-lucide-trash-2"
            @click="logsStore.clearLogs"
          >
            {{ t('logs.clear') }}
          </MButton>
          <MButton
            variant="tonal"
            size="sm"
            icon="i-lucide-activity"
            @click="onAnalyzeCrash"
          >
            {{ t('logs.crashAnalyze') }}
          </MButton>
          <MButton
            v-if="logsStore.activeProcess"
            variant="danger"
            size="sm"
            icon="i-lucide-power"
            :loading="isKilling"
            @click="onKillClick"
          >
            {{ t('logs.killProcess') }}
          </MButton>
        </div>
      </template>
    </MPageHeader>

    <!-- Toolbar: Filters, Search, Auto-scroll -->
    <MCard padding="sm" class="flex flex-wrap items-center justify-between gap-3 shrink-0">
      <div class="flex flex-wrap items-center gap-3">
        <MSegmented
          v-model="logsStore.levelFilter"
          :options="levelOptions.map(o => ({ label: t(o.label), value: o.value }))"
          size="sm"
        />

        <div class="w-64">
          <MInput
            v-model="logsStore.searchQuery"
            :placeholder="t('logs.searchPlaceholder')"
            icon="i-lucide-search"
            size="sm"
            clearable
          />
        </div>

        <div v-if="logsStore.runningProcesses.length > 1" class="w-56">
          <MSelect
            :model-value="logsStore.selectedProcessId != null ? String(logsStore.selectedProcessId) : 'all'"
            :options="processOptions"
            size="sm"
            @update:model-value="(val) => logsStore.selectedProcessId = val === 'all' ? null : Number(val)"
          />
        </div>
      </div>

      <div class="flex items-center gap-2 pl-2">
        <span class="text-xs text-fg-muted">{{ t('logs.autoScroll') }}</span>
        <MSwitch v-model="logsStore.autoScroll" size="sm" />
      </div>
    </MCard>

    <!-- Virtualized Log Output -->
    <div
      ref="logContainer"
      class="flex-1 bg-sunken rounded-xl border border-line p-4 font-mono text-xs overflow-y-auto relative select-text min-h-[300px]"
      @scroll="onScroll"
    >
      <div
        v-if="totalCount === 0"
        class="h-full flex items-center justify-center text-fg-muted"
      >
        {{ t('logs.empty') }}
      </div>

      <div
        v-else
        :style="{ height: `${totalHeight}px` }"
        class="relative w-full"
      >
        <div
          v-for="line in visibleLogs"
          :key="line.lineId"
          :style="{ top: `${line.top}px`, height: `${ROW_HEIGHT}px` }"
          class="absolute left-0 right-0 flex items-center gap-2 px-1 hover:bg-sunken whitespace-nowrap overflow-hidden text-ellipsis leading-tight"
        >
          <MBadge
            :variant="getLevelBadgeVariant(line.level)"
            size="sm"
            class="uppercase shrink-0 text-[10px] px-1 py-0 h-4"
          >
            {{ line.level }}
          </MBadge>
          <span :class="[getLineTextColor(line.level), 'select-text truncate']">
            {{ line.text }}
          </span>
        </div>
      </div>
    </div>
  </div>
</template>

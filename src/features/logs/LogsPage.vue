<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute } from 'vue-router'
import { useLogsStore } from './store'
import { useToast } from '../../ui/useToast'
import { useDialog } from '../../ui/useDialog'
import { useCrashStore } from '../crash/store'
import { useTasksStore } from '../tasks/store'
import { ipcClient } from '../../ipc/client'
import LauncherLogsPanel from './LauncherLogsPanel.vue'
import GameLogTerminal from './components/GameLogTerminal.vue'
import TasksManagerPanel from './components/TasksManagerPanel.vue'
import { MPageHeader, MTabs, MButton, MIconButton } from '../../ui'

const { t } = useI18n()
const toast = useToast()
const dialog = useDialog()
const logsStore = useLogsStore()
const crashStore = useCrashStore()
const tasksStore = useTasksStore()

const route = useRoute()
type TabKey = 'game' | 'launcher' | 'tasks'
const initialTab = ((): TabKey => {
  const q = route?.query?.tab
  if (q === 'launcher' || q === 'tasks') return q
  return 'game'
})()
const activeTab = ref<TabKey>(initialTab)

watch(
  () => route?.query?.tab,
  (newTab) => {
    if (newTab === 'launcher' || newTab === 'tasks' || newTab === 'game') {
      activeTab.value = newTab
    }
  },
)

const launcherLogsCount = ref(0)

async function fetchLauncherLogsCount() {
  try {
    const files = await ipcClient.launcherLogsList()
    launcherLogsCount.value = files.length
  } catch {
    launcherLogsCount.value = 0
  }
}

const tabs = computed(() => [
  { key: 'game', label: t('logs.tabs.game'), icon: 'i-lucide-terminal' },
  {
    key: 'launcher',
    label:
      launcherLogsCount.value > 0
        ? `${t('logs.tabs.launcher')} (${launcherLogsCount.value})`
        : t('logs.tabs.launcher'),
    icon: 'i-lucide-file-text',
  },
  { key: 'tasks', label: t('logs.tabs.tasks'), icon: 'i-lucide-list-checks' },
])

async function onAnalyzeCrash() {
  const instanceId = logsStore.selectedInstanceId || '1.20.1-vanilla'
  await crashStore.analyzeInstance(instanceId, null, true)
}

const isKilling = ref(false)

onMounted(async () => {
  await Promise.all([
    (async () => {
      await logsStore.fetchLogs()
      await logsStore.startListening()
    })(),
    (async () => {
      await tasksStore.fetchTasks()
      await tasksStore.startListening()
    })(),
    fetchLauncherLogsCount(),
  ])
})

onUnmounted(() => {
  logsStore.stopListening()
  tasksStore.stopListening()
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

async function onClearLogs() {
  const confirmed = await dialog.confirmDanger({
    title: t('logs.clearConfirmTitle'),
    content: t('logs.clearConfirmContent'),
    confirmText: t('logs.clear'),
    cancelText: t('common.cancel'),
  })
  if (!confirmed) return
  logsStore.clearLogs()
}

async function onKillClick() {
  const proc = logsStore.activeProcess
  if (!proc) return

  const confirmed = await dialog.confirmDanger({
    title: t('logs.killConfirmTitle'),
    content: t('logs.killConfirmMessage', { pid: proc.processId }),
    confirmText: t('logs.killProcess'),
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
</script>

<template>
  <div class="m-page h-full flex flex-col gap-5 overflow-hidden">
    <!-- Header -->
    <MPageHeader
      :title="t('logs.title')"
      :description="
        activeTab === 'game' && logsStore.activeProcess
          ? `${t('logs.status.running')} (PID: ${logsStore.activeProcess.processId})`
          : activeTab === 'tasks'
            ? t('tasks.indicator.activeTasks', { count: tasksStore.activeTasksCount })
            : t('logs.description')
      "
      :icon="
        activeTab === 'tasks'
          ? 'i-lucide-list-checks'
          : activeTab === 'launcher'
            ? 'i-lucide-file-text'
            : 'i-lucide-terminal'
      "
    >
      <template v-if="activeTab === 'game'" #actions>
        <div class="flex flex-wrap items-center gap-2">
          <MButton variant="tonal" size="sm" icon="i-lucide-copy" @click="onCopyLogs">
            {{ t('common.copy') }}
          </MButton>
          <MButton variant="tonal" size="sm" icon="i-lucide-download" @click="onExportLogs">
            {{ t('logs.export') }}
          </MButton>
          <MButton
            variant="text"
            size="sm"
            icon="i-lucide-trash"
            data-testid="clear-logs-btn"
            @click="onClearLogs"
          >
            {{ t('logs.clear') }}
          </MButton>
          <MButton variant="tonal" size="sm" icon="i-lucide-activity" @click="onAnalyzeCrash">
            {{ t('logs.crashAnalyze') }}
          </MButton>
          <MButton
            v-if="logsStore.activeProcess"
            variant="danger"
            size="sm"
            icon="i-lucide-power"
            :loading="isKilling"
            data-testid="kill-process-btn"
            @click="onKillClick"
          >
            {{ t('logs.killProcess') }}
          </MButton>
        </div>
      </template>
      <template v-else-if="activeTab === 'tasks'" #actions>
        <div class="flex items-center gap-2">
          <MButton
            v-if="tasksStore.tasks.length > 0"
            variant="tonal"
            size="sm"
            @click="tasksStore.clearFinished"
          >
            {{ t('tasks.clearFinished') }}
          </MButton>
          <MIconButton
            icon="i-lucide-rotate-cw"
            size="sm"
            :title="t('common.refresh')"
            :loading="tasksStore.loading"
            @click="tasksStore.fetchTasks"
          />
        </div>
      </template>
    </MPageHeader>

    <!-- Tabs navigation -->
    <MTabs v-model="activeTab" :tabs="tabs" class="shrink-0" />

    <!-- Game Logs Tab -->
    <div v-if="activeTab === 'game'" class="flex-1 min-h-0 flex flex-col overflow-hidden">
      <GameLogTerminal @switch-tab="activeTab = 'launcher'" />
    </div>

    <!-- Launcher Logs Tab -->
    <div v-else-if="activeTab === 'launcher'" class="flex-1 min-h-0 flex flex-col overflow-hidden">
      <LauncherLogsPanel @files-loaded="(files) => (launcherLogsCount = files.length)" />
    </div>

    <!-- Tasks Tab -->
    <div v-else-if="activeTab === 'tasks'" class="flex-1 overflow-y-auto px-2 py-3">
      <TasksManagerPanel />
    </div>
  </div>
</template>

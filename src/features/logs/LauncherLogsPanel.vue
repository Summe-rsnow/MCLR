<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { ipcClient } from '../../ipc/client'
import type { LauncherLogFile } from '../../ipc/client'
import { MSelect, MSkeleton, MVirtualList } from '../../ui'
const emit = defineEmits<{
  (e: 'files-loaded', files: LauncherLogFile[]): void
}>()

const { t } = useI18n()

const logFiles = ref<LauncherLogFile[]>([])
const selectedLogFile = ref('')
const logContent = ref('')
const readError = ref(false)
const loading = ref(false)

async function loadLogFiles() {
  loading.value = true
  try {
    logFiles.value = await ipcClient.launcherLogsList()
    // Cleanup may have removed the file that was on screen; fall back to the newest.
    const stillThere = logFiles.value.some((f) => f.fileName === selectedLogFile.value)
    if (!stillThere) {
      selectedLogFile.value = logFiles.value[0]?.fileName ?? ''
      await readSelectedLog()
    }
    emit('files-loaded', logFiles.value)
  } catch {
    logFiles.value = []
    emit('files-loaded', [])
  } finally {
    loading.value = false
  }
}

async function readSelectedLog() {
  if (!selectedLogFile.value) {
    logContent.value = ''
    readError.value = false
    return
  }
  readError.value = false
  try {
    logContent.value = await ipcClient.launcherLogRead(selectedLogFile.value)
  } catch {
    logContent.value = ''
    readError.value = true
  }
}

const displayContent = computed(() => {
  if (logContent.value) return logContent.value
  if (readError.value) return t('logs.launcher.readFailed')
  if (selectedLogFile.value) return t('logs.launcher.noContent')
  return t('logs.launcher.empty')
})

// Single scroll container shared by the panel and MVirtualList, which needs it to
// measure rows against the real scroller instead of hunting for an ancestor.
const scrollContainer = ref<HTMLElement | null>(null)
const logLines = computed(() => displayContent.value.split(/\r?\n/))

async function onSelectLogFile(val: string | number) {
  selectedLogFile.value = String(val)
  await readSelectedLog()
}

onMounted(async () => {
  await loadLogFiles()
})

// Cleanup lives in the page header; it reloads the list through this handle.
defineExpose({ reload: loadLogFiles })
</script>

<template>
  <!-- `px-1 pt-1` keeps the outer focus rings of the controls inside this clipping panel. -->
  <div class="flex-1 min-h-0 flex flex-col space-y-3 overflow-hidden px-1 pt-1">
    <!-- Controls Header -->
    <div class="flex flex-wrap items-center gap-3 shrink-0">
      <div class="w-full sm:w-80">
        <div v-if="loading && logFiles.length === 0">
          <MSkeleton class="h-9 w-48 rounded-lg" />
        </div>
        <MSelect
          v-else
          :model-value="selectedLogFile"
          :options="logFiles.map((f) => ({ label: f.fileName, value: f.fileName }))"
          :placeholder="t('logs.launcher.filePlaceholder')"
          size="sm"
          @update:model-value="onSelectLogFile"
        />
      </div>
    </div>

    <!-- Log output container (rounded-xl overflow-hidden clips inner scrollbar) -->
    <div
      class="flex-1 min-h-0 rounded-xl overflow-hidden border border-line bg-sunken/50 flex flex-col"
    >
      <div v-if="loading" class="flex-1 p-4 space-y-2 overflow-hidden">
        <MSkeleton class="h-3 w-[95%]" />
        <MSkeleton class="h-3 w-[80%]" />
        <MSkeleton class="h-3 w-[90%]" />
        <MSkeleton class="h-3 w-[60%]" />
        <MSkeleton class="h-3 w-[88%]" />
        <MSkeleton class="h-3 w-[75%]" />
        <MSkeleton class="h-3 w-[92%]" />
        <MSkeleton class="h-3 w-[85%]" />
        <MSkeleton class="h-3 w-[68%]" />
        <MSkeleton class="h-3 w-[78%]" />
        <MSkeleton class="h-3 w-[90%]" />
        <MSkeleton class="h-3 w-[70%]" />
      </div>
      <div
        v-else
        ref="scrollContainer"
        data-testid="launcher-log-content"
        class="flex-1 p-4 pr-3 font-mono text-xs overflow-y-auto text-fg select-text h-full"
      >
        <!-- MVirtualList renders plain DOM below 200 lines and virtualizes above. -->
        <MVirtualList
          :items="logLines"
          :item-key="(_line, i) => i"
          :estimate-size="20"
          :threshold="200"
          :scroll-el="scrollContainer"
        >
          <template #item="{ item: line }">
            <div class="whitespace-pre-wrap break-all leading-5 min-h-[20px]">{{ line }}</div>
          </template>
        </MVirtualList>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useLogsStore } from '../store'
import type { LogLevel } from '../../../ipc/client'
import {
  MCard,
  MInput,
  MSegmented,
  MSwitch,
  MSelect,
  MBadge,
  MEmpty,
  MButton,
  MVirtualList,
  MSkeleton,
} from '../../../ui'

const emit = defineEmits<{
  (e: 'switch-tab', tab: 'launcher'): void
}>()

const { t } = useI18n()
const logsStore = useLogsStore()

const logContainer = ref<HTMLElement | null>(null)
const isAtBottom = ref(true)

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

const totalCount = computed(() => logsStore.filteredLogs.length)

function onScroll(e: Event) {
  const target = e.target as HTMLElement
  isAtBottom.value = target.scrollHeight - target.scrollTop - target.clientHeight < 40
}

let scrollRaf: number | null = null

function scrollToBottom() {
  if (!logsStore.autoScroll || !isAtBottom.value || !logContainer.value) return
  if (typeof requestAnimationFrame === 'undefined') {
    logContainer.value.scrollTop = logContainer.value.scrollHeight
    return
  }
  if (scrollRaf !== null) return
  // One rAF, one scrollHeight read, one write: no second forced layout after nextTick
  scrollRaf = requestAnimationFrame(() => {
    scrollRaf = null
    const el = logContainer.value
    if (!el || !isAtBottom.value) return
    el.scrollTop = el.scrollHeight
  })
}

watch(() => logsStore.flushTick, scrollToBottom)

onMounted(scrollToBottom)

onUnmounted(() => {
  if (scrollRaf !== null && typeof cancelAnimationFrame !== 'undefined') {
    cancelAnimationFrame(scrollRaf)
    scrollRaf = null
  }
})

function getLevelBadgeVariant(
  level: LogLevel,
): 'primary' | 'success' | 'warning' | 'danger' | 'neutral' {
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
  <div class="flex-1 flex flex-col gap-4 min-h-0">
    <!-- Toolbar: Filters, Search, Auto-scroll -->
    <MCard padding="sm" class="flex flex-wrap items-center justify-between gap-3 shrink-0">
      <div class="flex flex-wrap items-center gap-3">
        <MSegmented
          v-model="logsStore.levelFilter"
          :options="levelOptions.map((o) => ({ label: t(o.label), value: o.value }))"
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
            :model-value="
              logsStore.selectedProcessId != null ? String(logsStore.selectedProcessId) : 'all'
            "
            :options="processOptions"
            size="sm"
            @update:model-value="
              (val) => (logsStore.selectedProcessId = val === 'all' ? null : Number(val))
            "
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
      <div v-if="logsStore.loading && totalCount === 0" class="space-y-2 py-1">
        <MSkeleton class="h-4 w-[95%]" />
        <MSkeleton class="h-4 w-[80%]" />
        <MSkeleton class="h-4 w-[90%]" />
        <MSkeleton class="h-4 w-[60%]" />
        <MSkeleton class="h-4 w-[88%]" />
        <MSkeleton class="h-4 w-[75%]" />
        <MSkeleton class="h-4 w-[92%]" />
        <MSkeleton class="h-4 w-[85%]" />
        <MSkeleton class="h-4 w-[68%]" />
        <MSkeleton class="h-4 w-[78%]" />
        <MSkeleton class="h-4 w-[90%]" />
        <MSkeleton class="h-4 w-[70%]" />
      </div>

      <div
        v-else-if="totalCount === 0"
        class="h-full flex flex-col items-center justify-center p-8 text-center"
      >
        <template v-if="logsStore.entries.length === 0">
          <MEmpty
            :title="t('logs.empty')"
            :description="t('logs.emptyHint')"
            icon="i-lucide-terminal"
          >
            <template #action>
              <MButton
                variant="tonal"
                size="sm"
                icon="i-lucide-file-text"
                data-testid="switch-to-launcher-logs-btn"
                @click="emit('switch-tab', 'launcher')"
              >
                {{ t('logs.switchToLauncher') }}
              </MButton>
            </template>
          </MEmpty>
        </template>
        <div v-else class="text-fg-muted">
          {{ t('logs.empty') }}
        </div>
      </div>

      <MVirtualList
        v-else
        :items="logsStore.filteredLogs"
        :item-key="(line, index) => `${line.timestamp}-${index}`"
        :estimate-size="24"
        :threshold="60"
        :scroll-el="logContainer"
      >
        <template #item="{ item: line }">
          <div
            class="flex items-center gap-2 px-1 py-0.5 hover:bg-sunken whitespace-nowrap overflow-hidden text-ellipsis leading-tight h-[24px]"
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
        </template>
      </MVirtualList>
    </div>
  </div>
</template>

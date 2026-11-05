<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import type { TaskNode, StepState } from '../../../ipc/client'
import { useTasksStore } from '../store'
import {
  resolveTaskTitle,
  formatSpeed,
  getTaskBadgeVariant,
  formatDuration,
  stepLabel,
} from '../format'
import { useDialog } from '../../../ui/useDialog'
import { useErrorText } from '../../../composables/useErrorText'
import { MCard, MProgress, MBadge, MButton, MEmpty, MCollapse } from '../../../ui'

const props = defineProps<{
  task: TaskNode | null
}>()

const { t, te } = useI18n()
const tasksStore = useTasksStore()
const dialog = useDialog()
const { formatError } = useErrorText()
const showErrorDetail = ref(false)

// Clock ticker for elapsed duration & retry countdown
const now = ref(Date.now())
let ticker: number | null = null

onMounted(() => {
  ticker = window.setInterval(() => {
    now.value = Date.now()
  }, 1000)
})

onUnmounted(() => {
  if (ticker !== null) {
    clearInterval(ticker)
    ticker = null
  }
})

const retryCountdown = computed(() => {
  if (!props.task?.nextRetryAtMs) return null
  const diffSec = Math.max(0, Math.round((props.task.nextRetryAtMs - now.value) / 1000))
  return diffSec
})

const elapsedDuration = computed(() => {
  if (!props.task) return '0s'
  const start = props.task.startedAtMs || props.task.createdAtMs
  if (!start) return '0s'
  const end = props.task.finishedAtMs || now.value
  return formatDuration(Math.max(0, end - start))
})

// Sparkline SVG path generation
const sparklinePoints = computed(() => {
  if (!props.task) return []
  const samples = tasksStore.speedSamples[props.task.id] || []
  if (samples.length < 2) return []
  const max = Math.max(...samples, 1024)
  const width = 100
  const height = 24
  return samples.map((val, idx) => {
    const x = (idx / (samples.length - 1)) * width
    const y = height - (val / max) * height
    return { x, y }
  })
})

const sparklineD = computed(() => {
  const pts = sparklinePoints.value
  if (pts.length < 2) return ''
  let d = `M ${pts[0].x} ${pts[0].y}`
  for (let i = 1; i < pts.length; i++) {
    d += ` L ${pts[i].x} ${pts[i].y}`
  }
  // Close path along bottom for fill
  const last = pts[pts.length - 1]
  const first = pts[0]
  d += ` L ${last.x} 24 L ${first.x} 24 Z`
  return d
})

const sparklineLine = computed(() => {
  const pts = sparklinePoints.value
  if (pts.length < 2) return ''
  let d = `M ${pts[0].x} ${pts[0].y}`
  for (let i = 1; i < pts.length; i++) {
    d += ` L ${pts[i].x} ${pts[i].y}`
  }
  return d
})

async function onCancel() {
  if (!props.task) return
  const confirmed = await dialog.confirmDanger({
    title: t('tasks.cancelConfirmTitle'),
    content: t('tasks.cancelConfirmContent'),
    confirmText: t('tasks.actions.cancel'),
    cancelText: t('common.cancel'),
  })
  if (!confirmed) return
  await tasksStore.cancel(props.task.id)
}

async function onMoveFirst() {
  if (!props.task || props.task.state !== 'waiting') return
  const waitingIds = tasksStore.tasks.filter((t) => t.state === 'waiting').map((t) => t.id)
  const filtered = waitingIds.filter((id) => id !== props.task!.id)
  filtered.unshift(props.task.id)
  await tasksStore.reorder(filtered)
}

function getStepIconClass(state: StepState): string {
  switch (state) {
    case 'done':
      return 'i-lucide-check-circle-2 text-success'
    case 'running':
      return 'i-lucide-loader-2 text-primary animate-spin'
    case 'failed':
      return 'i-lucide-x-circle text-danger'
    case 'skipped':
      return 'i-lucide-minus-circle text-fg-muted'
    case 'pending':
    default:
      return 'i-lucide-circle text-fg-muted/40'
  }
}
</script>

<template>
  <div class="h-full flex flex-col min-w-0">
    <!-- Empty Detail State -->
    <div v-if="!task" class="h-full flex items-center justify-center p-8">
      <MEmpty
        :title="t('tasks.empty')"
        :description="t('tasks.emptySubtitle')"
        icon="i-lucide-list-checks"
      />
    </div>

    <!-- Active Task Detail Card -->
    <MCard v-else padding="lg" class="h-full flex flex-col space-y-5 overflow-y-auto">
      <!-- Header -->
      <div class="flex items-start justify-between gap-4 pb-4 border-b border-line shrink-0">
        <div class="space-y-1.5 min-w-0 flex-1">
          <div class="flex items-center gap-2.5 flex-wrap">
            <h2 class="text-lg font-semibold text-fg truncate">
              {{ resolveTaskTitle(task.titleKey, task.titleParams || {}, t, te) }}
            </h2>
            <MBadge :variant="getTaskBadgeVariant(task.state)">
              {{ t('tasks.state.' + task.state) }}
            </MBadge>
            <MBadge v-if="task.attempt > 0" variant="neutral" size="sm">
              {{ t('tasks.detail.attempt', { attempt: task.attempt + 1, max: task.maxAttempts }) }}
            </MBadge>
          </div>

          <!-- Retry status / countdown banner -->
          <div
            v-if="task.state === 'retrying'"
            class="text-xs text-warning flex items-center gap-1.5 font-medium"
          >
            <span class="i-lucide-alert-circle w-3.5 h-3.5" />
            <span v-if="retryCountdown !== null">
              {{ t('tasks.retry.in', { seconds: retryCountdown }) }}
            </span>
            <span v-else>
              {{ t('tasks.retry.waitingNetwork') }}
            </span>
          </div>

          <div class="text-xs text-fg-muted font-mono flex items-center gap-3">
            <span>ID: {{ task.id.slice(0, 8) }}</span>
            <span>{{
              task.resumable ? t('tasks.detail.resumable') : t('tasks.detail.nonResumable')
            }}</span>
          </div>
        </div>

        <!-- Action Buttons per State -->
        <div class="flex items-center gap-2 shrink-0">
          <template v-if="task.state === 'running'">
            <MButton
              variant="tonal"
              size="sm"
              icon="i-lucide-pause"
              @click="tasksStore.pause(task.id)"
            >
              {{ t('tasks.actions.pause') }}
            </MButton>
            <MButton
              variant="danger"
              size="sm"
              icon="i-lucide-x"
              data-testid="task-cancel-btn"
              @click="onCancel"
            >
              {{ t('tasks.actions.cancel') }}
            </MButton>
          </template>

          <template v-else-if="task.state === 'paused'">
            <MButton
              variant="primary"
              size="sm"
              icon="i-lucide-play"
              @click="tasksStore.resume(task.id)"
            >
              {{ t('tasks.actions.resume') }}
            </MButton>
            <MButton variant="danger" size="sm" icon="i-lucide-x" @click="onCancel">
              {{ t('tasks.actions.cancel') }}
            </MButton>
          </template>

          <template v-else-if="task.state === 'waiting'">
            <MButton
              variant="tonal"
              size="sm"
              icon="i-lucide-arrow-up-to-line"
              @click="onMoveFirst"
            >
              {{ t('tasks.actions.moveFirst') }}
            </MButton>
            <MButton variant="danger" size="sm" icon="i-lucide-x" @click="onCancel">
              {{ t('tasks.actions.cancel') }}
            </MButton>
          </template>

          <template v-else-if="task.state === 'interrupted'">
            <MButton
              v-if="task.resumable"
              variant="primary"
              size="sm"
              icon="i-lucide-play"
              data-testid="task-restore-btn"
              @click="tasksStore.restore(task.id)"
            >
              {{ t('tasks.actions.restore') }}
            </MButton>
            <MButton
              variant="text"
              size="sm"
              icon="i-lucide-trash"
              @click="tasksStore.remove(task.id)"
            >
              {{ t('tasks.actions.remove') }}
            </MButton>
          </template>

          <template v-else-if="task.state === 'failed'">
            <MButton
              v-if="task.resumable"
              variant="primary"
              size="sm"
              icon="i-lucide-rotate-cw"
              data-testid="task-retry-btn"
              @click="tasksStore.retry(task.id)"
            >
              {{ t('tasks.actions.retry') }}
            </MButton>
            <MButton
              variant="text"
              size="sm"
              icon="i-lucide-trash"
              @click="tasksStore.remove(task.id)"
            >
              {{ t('tasks.actions.remove') }}
            </MButton>
          </template>

          <template v-else-if="task.state === 'success' || task.state === 'canceled'">
            <MButton
              variant="text"
              size="sm"
              icon="i-lucide-trash"
              @click="tasksStore.remove(task.id)"
            >
              {{ t('tasks.actions.remove') }}
            </MButton>
          </template>
        </div>
      </div>

      <!-- Main Progress & Metric Panel -->
      <div class="space-y-4 p-4 rounded-xl m-well">
        <div class="space-y-2">
          <div class="flex items-center justify-between text-sm">
            <span class="font-medium text-fg"> {{ Math.round((task.progress || 0) * 100) }}% </span>
            <div class="flex items-center gap-4 text-xs font-mono text-fg-muted">
              <span>{{ t('tasks.detail.elapsed') }}: {{ elapsedDuration }}</span>
              <span v-if="task.state === 'running'">
                {{ t('tasks.detail.speed') }}: {{ formatSpeed(task.speedBps) }}
              </span>
            </div>
          </div>
          <MProgress :value="(task.progress || 0) * 100" size="md" />
        </div>

        <!-- Speed Sparkline -->
        <div v-if="sparklinePoints.length > 1 && task.state === 'running'" class="pt-2">
          <svg viewBox="0 0 100 24" class="w-full h-8 text-primary overflow-visible">
            <path :d="sparklineD" fill="currentColor" opacity="0.15" />
            <path
              :d="sparklineLine"
              fill="none"
              stroke="currentColor"
              stroke-width="1.5"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </svg>
        </div>
      </div>

      <!-- Error Display Block -->
      <div
        v-if="task.error"
        class="p-4 rounded-xl bg-danger-soft/30 border border-danger/30 space-y-2"
      >
        <div class="flex items-center gap-2 text-sm font-semibold text-danger">
          <span class="i-lucide-alert-triangle w-4 h-4 shrink-0" />
          <span>{{ t('tasks.detail.errorTitle') }}</span>
        </div>
        <p class="text-sm text-fg">
          {{ formatError(task.error) }}
        </p>
        <div v-if="task.error.detail || task.error.code" class="pt-1">
          <button
            type="button"
            class="text-xs text-danger/80 hover:text-danger font-medium flex items-center gap-1 cursor-pointer transition-colors"
            @click="showErrorDetail = !showErrorDetail"
          >
            <span>{{ t('tasks.detail.errorDetail') }}</span>
            <span
              class="w-3.5 h-3.5 transition-transform"
              :class="showErrorDetail ? 'i-lucide-chevron-up' : 'i-lucide-chevron-down'"
            />
          </button>
          <MCollapse :open="showErrorDetail" class="mt-2">
            <div class="p-2.5 rounded-lg bg-canvas font-mono text-xs text-fg-muted space-y-1">
              <div>code: {{ task.error.code }}</div>
              <div v-if="task.error.detail" class="break-all whitespace-pre-wrap">
                detail: {{ task.error.detail }}
              </div>
            </div>
          </MCollapse>
        </div>
      </div>

      <!-- Steps Timeline -->
      <div class="space-y-3 flex-1 min-h-0 pt-2">
        <h3 class="text-xs font-semibold uppercase tracking-wider text-fg-muted">
          {{ t('tasks.detail.steps') }} ({{ task.steps.length }})
        </h3>

        <div v-if="task.steps.length === 0" class="text-xs text-fg-muted py-4 text-center">
          {{ t('tasks.detail.noSteps') }}
        </div>

        <div v-else class="space-y-2">
          <div
            v-for="step in task.steps"
            :key="step.key"
            class="flex items-start gap-3 p-3 rounded-xl transition-all"
            :class="[
              step.state === 'running' ? 'bg-primary-soft/30 border border-primary/20' : 'm-well',
            ]"
          >
            <!-- Step Status Icon -->
            <span class="w-4 h-4 mt-0.5" :class="getStepIconClass(step.state)" />

            <!-- Step Info -->
            <div class="flex-1 min-w-0 space-y-1">
              <div class="flex items-center justify-between gap-2">
                <span
                  class="text-sm font-medium"
                  :class="step.state === 'running' ? 'text-primary font-semibold' : 'text-fg'"
                >
                  {{ stepLabel(step.key, t, te) }}
                </span>
                <span v-if="step.itemsTotal > 0" class="text-xs font-mono text-fg-muted shrink-0">
                  {{ step.itemsDone }} / {{ step.itemsTotal }}
                </span>
              </div>

              <!-- Sub-step Error -->
              <div v-if="step.error" class="text-xs text-danger">
                {{ formatError(step.error) }}
              </div>
            </div>
          </div>
        </div>
      </div>
    </MCard>
  </div>
</template>

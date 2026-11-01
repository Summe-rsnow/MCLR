<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useTasksStore } from '../../tasks/store'
import { useDialog } from '../../../ui/useDialog'
import { MCard, MEmpty, MProgress, MBadge, MIconButton, MSkeleton } from '../../../ui'
import type { TaskNode } from '../../../ipc/client'

const { t, te } = useI18n()
const tasksStore = useTasksStore()
const dialog = useDialog()

function formatSpeed(bps: number): string {
  if (bps <= 0) return '0 B/s'
  const units = ['B/s', 'KB/s', 'MB/s', 'GB/s']
  let val = bps
  let u = 0
  while (val >= 1024 && u < units.length - 1) {
    val /= 1024
    u++
  }
  return `${val.toFixed(u === 0 ? 0 : 1)} ${units[u]}`
}

function resolveTaskTitle(key: string, params: Record<string, string>): string {
  let lookupKey = key
  if (key.startsWith('task.')) {
    lookupKey = 'tasks.item.' + key.slice(5)
  }
  if (lookupKey === 'tasks.item.download.group' || key === 'task.download.group') {
    if (!params?.count) {
      const raw = te(lookupKey) ? t(lookupKey) : te(key) ? t(key) : ''
      if (raw) {
        return raw.replace(/\s*\([^)]*\)/g, '').trim()
      }
      return key
    }
  }
  if (te(lookupKey)) {
    return t(lookupKey, params)
  }
  if (te(key)) {
    return t(key, params)
  }
  return key
}

function taskStage(task: TaskNode): string {
  // The stage is only meaningful while work is happening.
  if (task.state !== 'running' && task.state !== 'paused') return ''
  const stage = task.titleParams?.stage
  if (!stage) return ''
  const key = `tasks.stages.${stage}`
  return te(key) ? t(key) : ''
}

function getTaskBadgeVariant(state: string): 'primary' | 'success' | 'danger' | 'neutral' {
  switch (state) {
    case 'running':
      return 'primary'
    case 'success':
      return 'success'
    case 'failed':
      return 'danger'
    case 'paused':
    case 'waiting':
    default:
      return 'neutral'
  }
}

async function onCancelTask(taskId: string) {
  const confirmed = await dialog.confirmDanger({
    title: t('tasks.cancelConfirmTitle'),
    content: t('tasks.cancelConfirmContent'),
    confirmText: t('tasks.actions.cancel'),
    cancelText: t('common.cancel'),
  })
  if (!confirmed) return
  await tasksStore.cancel(taskId)
}
</script>

<template>
  <div class="space-y-3">
    <!-- Loading Skeleton State -->
    <div v-if="!tasksStore.initialized && tasksStore.loading" class="space-y-3 py-1">
      <MCard v-for="n in 3" :key="n" padding="md" class="space-y-3">
        <div class="flex items-center justify-between gap-3">
          <div class="flex items-center gap-2.5 min-w-0">
            <MSkeleton width="1rem" height="1rem" class="shrink-0" />
            <MSkeleton :width="n % 2 === 0 ? '12rem' : '8rem'" height="1.125rem" />
            <MSkeleton width="3.5rem" height="1.25rem" class="rounded-md" />
          </div>
          <div class="flex items-center gap-2 shrink-0">
            <MSkeleton width="4.5rem" height="1rem" />
            <div class="flex items-center gap-1">
              <MSkeleton width="1.75rem" height="1.75rem" class="rounded-lg" />
              <MSkeleton width="1.75rem" height="1.75rem" class="rounded-lg" />
            </div>
          </div>
        </div>
        <div class="space-y-1">
          <MSkeleton width="100%" height="0.5rem" class="rounded-full" />
          <div class="flex justify-between">
            <MSkeleton width="2.5rem" height="0.75rem" />
            <MSkeleton width="4rem" height="0.75rem" />
          </div>
        </div>
      </MCard>
    </div>

    <!-- Empty State -->
    <div v-else-if="tasksStore.tasks.length === 0" class="py-16">
      <MEmpty
        :title="t('tasks.empty')"
        :description="t('tasks.emptySubtitle')"
        icon="i-lucide-list-checks"
      />
    </div>

    <!-- Task List -->
    <TransitionGroup v-else name="m-list" tag="div" class="space-y-3">
      <template v-for="(task, index) in tasksStore.taskTree" :key="task.id">
        <MCard padding="md" class="space-y-3 m-stagger" :style="{ '--i': index }">
          <div class="flex items-center justify-between gap-3">
            <div class="flex items-center gap-2.5 min-w-0">
              <span
                class="w-4 h-4 shrink-0"
                :class="{
                  'i-lucide-loader-circle animate-spin text-primary': task.state === 'running',
                  'i-lucide-circle-check-big text-success': task.state === 'success',
                  'i-lucide-circle-x text-danger': task.state === 'failed',
                  'i-lucide-circle-pause text-warning': task.state === 'paused',
                  'i-lucide-clock text-fg-muted': task.state === 'waiting',
                  'i-lucide-ban text-fg-muted': task.state === 'canceled',
                }"
              />
              <div class="min-w-0">
                <span class="block text-sm font-semibold text-fg truncate">
                  {{ resolveTaskTitle(task.titleKey, task.titleParams) }}
                </span>
                <span v-if="taskStage(task)" class="block text-[11px] text-fg-muted truncate">
                  {{ taskStage(task) }}
                </span>
              </div>
            </div>

            <div class="flex items-center gap-2 shrink-0">
              <span
                v-if="task.state === 'running' && task.speedBps > 0"
                class="text-xs font-mono text-fg-muted"
              >
                {{ formatSpeed(task.speedBps) }}
              </span>
              <MBadge :variant="getTaskBadgeVariant(task.state)">
                {{ t(`tasks.state.${task.state}`) }}
              </MBadge>

              <!-- Actions -->
              <div class="flex items-center gap-1">
                <MIconButton
                  v-if="task.state === 'running'"
                  icon="i-lucide-pause"
                  size="sm"
                  :title="t('tasks.actions.pause')"
                  @click="tasksStore.pause(task.id)"
                />
                <MIconButton
                  v-if="task.state === 'paused'"
                  icon="i-lucide-play"
                  size="sm"
                  :title="t('tasks.actions.resume')"
                  @click="tasksStore.resume(task.id)"
                />
                <MIconButton
                  v-if="
                    task.state === 'running' || task.state === 'waiting' || task.state === 'paused'
                  "
                  icon="i-lucide-x"
                  size="sm"
                  :title="t('tasks.actions.cancel')"
                  @click="onCancelTask(task.id)"
                />
              </div>
            </div>
          </div>
          <!-- Task Error -->
          <div
            v-if="task.error"
            class="text-xs text-danger bg-danger-soft p-2 rounded-lg select-text"
          >
            {{ task.error.detail || task.error.code }}
          </div>

          <!-- Progress Bar -->
          <div v-if="task.state === 'running' || task.state === 'paused'" class="space-y-1">
            <MProgress :value="Math.round(task.progress * 100)" :max="100" size="sm" />
            <div class="flex justify-between text-[11px] text-fg-muted font-mono">
              <span>{{ Math.round(task.progress * 100) }}%</span>
              <span v-if="task.speedBps > 0">{{ formatSpeed(task.speedBps) }}</span>
            </div>
          </div>

          <!-- Child Tasks -->
          <div
            v-if="task.children.length > 0"
            class="pl-4 pt-2 border-l-2 border-line space-y-2 mt-2"
          >
            <div
              v-for="child in task.children"
              :key="child.id"
              class="flex items-center justify-between text-xs py-1"
            >
              <div class="flex items-center gap-2 truncate">
                <span
                  class="w-3.5 h-3.5 shrink-0"
                  :class="{
                    'i-lucide-loader-circle animate-spin text-primary': child.state === 'running',
                    'i-lucide-check text-success': child.state === 'success',
                    'i-lucide-x text-danger': child.state === 'failed',
                    'i-lucide-pause text-warning': child.state === 'paused',
                    'i-lucide-clock text-fg-muted': child.state === 'waiting',
                    'i-lucide-ban text-fg-muted': child.state === 'canceled',
                  }"
                />
                <span class="truncate text-fg">{{
                  resolveTaskTitle(child.titleKey, child.titleParams)
                }}</span>
              </div>

              <div class="flex items-center gap-2">
                <span
                  v-if="child.state === 'running' && child.speedBps > 0"
                  class="font-mono text-fg-muted"
                >
                  {{ formatSpeed(child.speedBps) }}
                </span>
                <span class="text-fg-muted font-mono">{{ Math.round(child.progress * 100) }}%</span>
              </div>
            </div>
          </div>
        </MCard>
      </template>
    </TransitionGroup>
  </div>
</template>

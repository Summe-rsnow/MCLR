<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useTasksStore } from './store'
import {
  MPageHeader,
  MButton,
  MIconButton,
  MProgress,
  MBadge,
  MCard,
  MEmpty,
  MSkeletonList,
} from '../../ui'

const { t, te } = useI18n()
const tasksStore = useTasksStore()

onMounted(async () => {
  await tasksStore.fetchTasks()
  await tasksStore.startListening()
})

onUnmounted(() => {
  tasksStore.stopListening()
})

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
  // If key matches task.<section>.<name>, map to tasks.item.<section>.<name>
  let lookupKey = key
  if (key.startsWith('task.')) {
    lookupKey = 'tasks.item.' + key.slice(5)
  }
  if (te(lookupKey)) {
    return t(lookupKey, params)
  }
  if (te(key)) {
    return t(key, params)
  }
  return key
}

function getBadgeVariant(state: string): 'primary' | 'success' | 'danger' | 'neutral' {
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
</script>

<template>
  <div class="m-page">
    <!-- Header -->
    <MPageHeader
      :title="t('tasks.title')"
      :description="t('tasks.indicator.activeTasks', { count: tasksStore.activeTasksCount })"
      icon="i-lucide-list-checks"
    >
      <template #actions>
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

    <!-- Loading Skeleton State -->
    <div v-if="!tasksStore.initialized && tasksStore.loading" class="py-4">
      <MSkeletonList :rows="3" variant="card" />
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
                  'i-lucide-loader-2 animate-spin text-primary': task.state === 'running',
                  'i-lucide-check-circle-2 text-success': task.state === 'success',
                  'i-lucide-x-circle text-danger': task.state === 'failed',
                  'i-lucide-pause-circle text-warning': task.state === 'paused',
                  'i-lucide-clock text-fg-muted': task.state === 'waiting',
                  'i-lucide-ban text-fg-muted': task.state === 'canceled',
                }"
              />
              <span class="text-sm font-semibold text-fg truncate">
                {{ resolveTaskTitle(task.titleKey, task.titleParams) }}
              </span>
            </div>

            <div class="flex items-center gap-2 shrink-0">
              <span v-if="task.state === 'running' && task.speedBps > 0" class="text-xs font-mono text-fg-muted">
                {{ formatSpeed(task.speedBps) }}
              </span>
              <MBadge :variant="getBadgeVariant(task.state)">
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
                  v-if="task.state === 'running' || task.state === 'waiting' || task.state === 'paused'"
                  icon="i-lucide-x"
                  size="sm"
                  :title="t('tasks.actions.cancel')"
                  @click="tasksStore.cancel(task.id)"
                />
              </div>
            </div>
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
          <div v-if="task.children.length > 0" class="pl-4 pt-2 border-l-2 border-line space-y-2 mt-2">
            <div
              v-for="child in task.children"
              :key="child.id"
              class="flex items-center justify-between text-xs py-1"
            >
              <div class="flex items-center gap-2 truncate">
                <span
                  class="w-3.5 h-3.5 shrink-0"
                  :class="{
                    'i-lucide-loader-2 animate-spin text-primary': child.state === 'running',
                    'i-lucide-check text-success': child.state === 'success',
                    'i-lucide-x text-danger': child.state === 'failed',
                    'i-lucide-pause text-warning': child.state === 'paused',
                    'i-lucide-clock text-fg-muted': child.state === 'waiting',
                    'i-lucide-ban text-fg-muted': child.state === 'canceled',
                  }"
                />
                <span class="truncate text-fg">{{ resolveTaskTitle(child.titleKey, child.titleParams) }}</span>
              </div>

              <div class="flex items-center gap-2">
                <span v-if="child.state === 'running' && child.speedBps > 0" class="font-mono text-fg-muted">
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

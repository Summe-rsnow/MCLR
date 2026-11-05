<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useTasksStore } from './store'
import TaskList from './components/TaskList.vue'
import TaskDetail from './components/TaskDetail.vue'
import TaskCleanupDialog from './components/TaskCleanupDialog.vue'
import { MPageHeader, MButton, MIconButton, MCard } from '../../ui'

const { t } = useI18n()
const tasksStore = useTasksStore()

const cleanupDialogOpen = ref(false)
const selectedTaskId = ref<string | null>(null)

const selectedTask = computed(() => {
  return tasksStore.tasks.find((t) => t.id === selectedTaskId.value) ?? null
})

const hasResumableInterrupted = computed(() => {
  return tasksStore.interruptedTasks.some((t) => t.resumable)
})

const hasFinishedTasks = computed(() => {
  return tasksStore.tasks.some(
    (t) => t.state === 'success' || t.state === 'canceled' || t.state === 'failed',
  )
})

// Auto-select first active task if nothing selected or selected task removed
watch(
  () => tasksStore.tasks,
  (tasks) => {
    if (tasks.length === 0) {
      selectedTaskId.value = null
      return
    }
    const currentStillExists = tasks.some((t) => t.id === selectedTaskId.value)
    if (!selectedTaskId.value || !currentStillExists) {
      const active = tasksStore.activeTasks
      if (active.length > 0) {
        selectedTaskId.value = active[0].id
      } else {
        selectedTaskId.value = tasks[0].id
      }
    }
  },
  { immediate: true },
)
onMounted(async () => {
  if (!tasksStore.initialized) {
    await tasksStore.fetchTasks()
  }
})
</script>

<template>
  <div class="m-page h-full flex flex-col gap-4 overflow-hidden">
    <!-- Header -->
    <MPageHeader
      :title="t('tasks.title')"
      :description="t('tasks.indicator.activeTasks', { count: tasksStore.activeTasksCount })"
      icon="i-lucide-list-checks"
      class="shrink-0"
    >
      <template #actions>
        <div class="flex items-center gap-2">
          <!-- Restore All Button -->
          <MButton
            v-if="hasResumableInterrupted"
            variant="primary"
            size="sm"
            icon="i-lucide-play"
            @click="tasksStore.restoreAll"
          >
            {{ t('tasks.actions.restoreAll') }}
          </MButton>

          <!-- Clear Finished Button -->
          <MButton
            v-if="hasFinishedTasks"
            variant="tonal"
            size="sm"
            @click="tasksStore.clearFinished"
          >
            {{ t('tasks.clearFinished') }}
          </MButton>

          <!-- Clean Leftovers Button -->
          <MButton
            variant="tonal"
            size="sm"
            icon="i-lucide-eraser"
            @click="cleanupDialogOpen = true"
          >
            {{ t('tasks.actions.cleanLeftovers') }}
          </MButton>

          <!-- Refresh Button -->
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

    <!-- Corrupt DB Reset Notice Banner -->
    <MCard
      v-if="tasksStore.storeStatus.resetNotice"
      padding="sm"
      class="bg-warning-soft text-warning border border-warning/30 flex items-center justify-between gap-3 shrink-0"
    >
      <div class="flex items-center gap-2 text-xs font-medium">
        <span class="i-lucide-alert-triangle w-4 h-4 shrink-0" />
        <span>{{ t('tasks.banner.reset') }}</span>
      </div>
      <MButton variant="tonal" size="xs" @click="tasksStore.ackReset">
        {{ t('tasks.banner.resetDismiss') }}
      </MButton>
    </MCard>

    <!-- Memory Mode Info Banner -->
    <MCard
      v-if="tasksStore.storeStatus.mode === 'memory'"
      padding="sm"
      class="bg-primary-soft/40 text-primary-soft-fg border border-primary/20 flex items-center gap-2 text-xs shrink-0"
    >
      <span class="i-lucide-info w-4 h-4 shrink-0" />
      <span>{{ t('tasks.banner.memory') }}</span>
    </MCard>

    <!-- Two Panes Layout -->
    <div class="flex-1 min-h-0 grid grid-cols-[360px_1fr] gap-4 overflow-hidden">
      <!-- Left List Pane -->
      <TaskList v-model:selected-id="selectedTaskId" />

      <!-- Right Detail Pane -->
      <TaskDetail :task="selectedTask" />
    </div>

    <!-- Cleanup Leftovers Dialog -->
    <TaskCleanupDialog v-model:open="cleanupDialogOpen" />
  </div>
</template>

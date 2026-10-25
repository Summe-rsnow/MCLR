<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useTasksStore } from './store'
import { MPageHeader, MButton, MIconButton } from '../../ui'
import TasksManagerPanel from '../logs/components/TasksManagerPanel.vue'

const { t } = useI18n()
const tasksStore = useTasksStore()

onMounted(async () => {
  await tasksStore.fetchTasks()
  await tasksStore.startListening()
})

onUnmounted(() => {
  tasksStore.stopListening()
})
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

    <TasksManagerPanel />
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useTasksStore } from '../store'
import { resolveTaskTitle, formatSpeed, getTaskBadgeVariant, stepLabel } from '../format'
import {
  MSearchBar,
  MSegmented,
  MScroll,
  MListItem,
  MBadge,
  MProgress,
  MEmpty,
  MSkeleton,
} from '../../../ui'

const props = defineProps<{
  selectedId: string | null
}>()

const emit = defineEmits<{
  (e: 'update:selectedId', val: string): void
}>()

const { t, te } = useI18n()
const tasksStore = useTasksStore()

const searchQuery = ref('')
const activeFilter = ref<'all' | 'active' | 'interrupted' | 'success' | 'failed' | 'canceled'>(
  'all',
)

const filterOptions = computed(() => [
  { value: 'all', label: t('tasks.filter.all') },
  { value: 'active', label: t('tasks.filter.active') },
  { value: 'interrupted', label: t('tasks.filter.interrupted') },
  { value: 'success', label: t('tasks.filter.success') },
  { value: 'failed', label: t('tasks.filter.failed') },
  { value: 'canceled', label: t('tasks.filter.canceled') },
])

const filteredTasks = computed(() => {
  const query = searchQuery.value.trim().toLowerCase()
  return tasksStore.tasks.filter((task) => {
    // Filter by tab
    if (activeFilter.value === 'active') {
      if (
        task.state !== 'waiting' &&
        task.state !== 'running' &&
        task.state !== 'paused' &&
        task.state !== 'retrying'
      ) {
        return false
      }
    } else if (activeFilter.value === 'interrupted') {
      if (task.state !== 'interrupted') return false
    } else if (activeFilter.value === 'success') {
      if (task.state !== 'success') return false
    } else if (activeFilter.value === 'failed') {
      if (task.state !== 'failed') return false
    } else if (activeFilter.value === 'canceled') {
      if (task.state !== 'canceled') return false
    }

    if (!query) return true

    const title = resolveTaskTitle(task.titleKey, task.titleParams || {}, t, te).toLowerCase()
    return (
      title.includes(query) ||
      task.id.toLowerCase().includes(query) ||
      task.kind.toLowerCase().includes(query)
    )
  })
})

// Native HTML5 DnD for Waiting tasks
let draggedId: string | null = null

function onDragStart(event: DragEvent, id: string) {
  draggedId = id
  if (event.dataTransfer) {
    event.dataTransfer.effectAllowed = 'move'
    event.dataTransfer.setData('text/plain', id)
  }
}

function onDragOver(event: DragEvent) {
  event.preventDefault()
  if (event.dataTransfer) {
    event.dataTransfer.dropEffect = 'move'
  }
}

async function onDrop(event: DragEvent, targetId: string) {
  event.preventDefault()
  if (!draggedId || draggedId === targetId) {
    draggedId = null
    return
  }

  const waitingTasks = tasksStore.tasks.filter((t) => t.state === 'waiting')
  const fromIndex = waitingTasks.findIndex((t) => t.id === draggedId)
  const toIndex = waitingTasks.findIndex((t) => t.id === targetId)

  if (fromIndex !== -1 && toIndex !== -1) {
    const ids = waitingTasks.map((t) => t.id)
    const [moved] = ids.splice(fromIndex, 1)
    ids.splice(toIndex, 0, moved)
    await tasksStore.reorder(ids)
  }

  draggedId = null
}
</script>

<template>
  <div class="flex flex-col h-full space-y-3 min-w-0">
    <!-- Search & Filter Controls -->
    <div class="space-y-2 shrink-0">
      <MSearchBar v-model="searchQuery" :placeholder="t('common.search')" size="sm" />
      <MSegmented v-model="activeFilter" :options="filterOptions" size="sm" class="w-full" />
    </div>

    <!-- Task List Scroll Area -->
    <div class="flex-1 min-h-0">
      <MScroll class="h-full pr-1">
        <!-- Skeleton Loading -->
        <div v-if="!tasksStore.initialized && tasksStore.loading" class="space-y-2 py-1">
          <div v-for="n in 5" :key="n" class="p-3 rounded-xl m-well space-y-2">
            <div class="flex items-center justify-between gap-2">
              <MSkeleton :width="n % 2 === 0 ? '10rem' : '7rem'" height="1rem" />
              <MSkeleton width="3.5rem" height="1.25rem" class="rounded-md" />
            </div>
            <MSkeleton width="100%" height="0.375rem" class="rounded-full" />
          </div>
        </div>

        <!-- Empty State -->
        <div v-else-if="filteredTasks.length === 0" class="py-12">
          <MEmpty
            :title="t('tasks.empty')"
            :description="t('tasks.emptySubtitle')"
            icon="i-lucide-list-checks"
          />
        </div>

        <!-- Items List -->
        <div v-else class="space-y-1.5 pb-2">
          <div
            v-for="task in filteredTasks"
            :key="task.id"
            :draggable="task.state === 'waiting'"
            :class="[task.state === 'waiting' ? 'cursor-grab active:cursor-grabbing' : '']"
            @dragstart="onDragStart($event, task.id)"
            @dragover="onDragOver"
            @drop="onDrop($event, task.id)"
          >
            <MListItem
              :active="task.id === selectedId"
              clickable
              data-testid="task-item"
              class="rounded-xl transition-all"
              @click="emit('update:selectedId', task.id)"
            >
              <div class="w-full space-y-2 py-0.5">
                <!-- Row Header -->
                <div class="flex items-center justify-between gap-2 min-w-0">
                  <span class="text-sm font-medium text-fg truncate flex-1 min-w-0">
                    {{ resolveTaskTitle(task.titleKey, task.titleParams || {}, t, te) }}
                  </span>
                  <MBadge :variant="getTaskBadgeVariant(task.state)" size="sm" class="shrink-0">
                    {{ t('tasks.state.' + task.state) }}
                  </MBadge>
                </div>

                <!-- Progress Bar & Speed -->
                <div
                  v-if="
                    task.state === 'running' || task.state === 'paused' || task.state === 'retrying'
                  "
                  class="space-y-1"
                >
                  <MProgress :value="(task.progress || 0) * 100" size="sm" />
                  <div class="flex items-center justify-between text-xs text-fg-muted font-mono">
                    <span class="truncate">
                      {{ task.currentStep ? stepLabel(task.currentStep, t, te) : '' }}
                    </span>
                    <span v-if="task.state === 'running' && task.speedBps > 0" class="shrink-0">
                      {{ formatSpeed(task.speedBps) }}
                    </span>
                  </div>
                </div>

                <!-- Waiting / Other State Hints -->
                <div v-else-if="task.currentStep" class="text-xs text-fg-muted truncate">
                  {{ stepLabel(task.currentStep, t, te) }}
                </div>
              </div>
            </MListItem>
          </div>
        </div>
      </MScroll>
    </div>
  </div>
</template>

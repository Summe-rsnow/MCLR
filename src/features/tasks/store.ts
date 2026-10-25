import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { ipcClient, type TaskNode } from '../../ipc/client'

export interface TaskTreeItem extends TaskNode {
  children: TaskTreeItem[]
}

export const useTasksStore = defineStore('tasks', () => {
  const tasks = ref<TaskNode[]>([])
  const loading = ref(false)
  const initialized = ref(false)
  let unlisten: UnlistenFn | null = null

  const activeTasks = computed(() =>
    tasks.value.filter((t) => t.state === 'running' || t.state === 'waiting' || t.state === 'paused'),
  )

  const activeTasksCount = computed(() => activeTasks.value.length)

  const aggregateSpeedBps = computed(() => {
    return tasks.value.reduce((acc, t) => {
      if (t.state === 'running') {
        return acc + (t.speedBps || 0)
      }
      return acc
    }, 0)
  })

  const aggregateProgress = computed(() => {
    const list = activeTasks.value
    if (list.length === 0) return 0
    const sum = list.reduce((acc, t) => acc + (t.progress || 0), 0)
    return sum / list.length
  })

  const taskTree = computed<TaskTreeItem[]>(() => {
    const map = new Map<string, TaskTreeItem>()
    const roots: TaskTreeItem[] = []

    for (const t of tasks.value) {
      map.set(t.id, { ...t, children: [] })
    }

    for (const item of map.values()) {
      if (item.parentId && map.has(item.parentId)) {
        map.get(item.parentId)!.children.push(item)
      } else {
        roots.push(item)
      }
    }

    return roots
  })

  async function fetchTasks() {
    loading.value = true
    try {
      tasks.value = await ipcClient.tasksList()
      initialized.value = true
    } catch {
      // In tests or if not ready
    } finally {
      loading.value = false
    }
  }

  function handleTaskUpdate(diff: TaskNode | TaskNode[]) {
    const updates = Array.isArray(diff) ? diff : [diff]
    for (const u of updates) {
      const idx = tasks.value.findIndex((t) => t.id === u.id)
      if (idx >= 0) {
        tasks.value[idx] = { ...tasks.value[idx], ...u }
      } else {
        tasks.value.push(u)
      }
    }
  }

  async function startListening() {
    if (unlisten) return
    try {
      unlisten = await listen<TaskNode | TaskNode[]>('tasks://update', (event) => {
        handleTaskUpdate(event.payload)
      })
    } catch {
      // Ignore if event system unavailable (e.g. non-Tauri browser mode)
    }
  }

  function stopListening() {
    if (unlisten) {
      unlisten()
      unlisten = null
    }
  }

  async function pause(id: string) {
    await ipcClient.taskPause(id)
    const t = tasks.value.find((x) => x.id === id)
    if (t) {
      t.state = 'paused'
      t.speedBps = 0
    }
  }

  async function resume(id: string) {
    await ipcClient.taskResume(id)
    const t = tasks.value.find((x) => x.id === id)
    if (t) {
      t.state = 'running'
    }
  }

  async function cancel(id: string) {
    await ipcClient.taskCancel(id)
    const t = tasks.value.find((x) => x.id === id)
    if (t) {
      t.state = 'canceled'
      t.speedBps = 0
    }
  }

  function clearFinished() {
    tasks.value = tasks.value.filter(
      (t) => t.state === 'running' || t.state === 'waiting' || t.state === 'paused',
    )
  }

  return {
    tasks,
    loading,
    initialized,
    activeTasksCount,
    aggregateSpeedBps,
    aggregateProgress,
    taskTree,
    fetchTasks,
    startListening,
    stopListening,
    pause,
    resume,
    cancel,
    clearFinished,
  }
})

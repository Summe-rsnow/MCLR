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
  // Ids the user already cleared: a broadcast that was in flight when the task was dropped must
  // not put it back on screen. Task ids are UUIDs, so an id is never reused.
  const dismissed = new Set<string>()
  let unlisten: UnlistenFn | null = null

  const activeTasks = computed(() =>
    tasks.value.filter(
      (t) => t.state === 'running' || t.state === 'waiting' || t.state === 'paused',
    ),
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
    const updates = (Array.isArray(diff) ? diff : [diff]).filter((u) => !dismissed.has(u.id))
    if (updates.length === 0) return
    // Apply the whole batch to a clone and publish once, so derived computeds
    // (taskTree / activeTasks / aggregates) evaluate once per IPC batch instead of per item.
    const next = tasks.value.slice()
    const indexById = new Map(next.map((t, i) => [t.id, i] as const))
    for (const u of updates) {
      const idx = indexById.get(u.id)
      if (idx !== undefined) {
        next[idx] = { ...next[idx], ...u }
      } else {
        indexById.set(u.id, next.length)
        next.push(u)
      }
    }
    tasks.value = next
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

  /**
   * Removes finished tasks for good.
   *
   * Filtering only the local list is not enough: the registry keeps every task of the session, so
   * the next `fetchTasks` would bring all the failed ones straight back. The backend drops them
   * first, then the local list follows its answer.
   */
  async function clearFinished() {
    try {
      const removed = await ipcClient.tasksClearFinished()
      for (const id of removed) dismissed.add(id)
      const gone = new Set(removed)
      tasks.value = tasks.value.filter((t) => !gone.has(t.id))
    } catch {
      // Backend unavailable (tests, browser mock): fall back to hiding them locally.
      tasks.value = tasks.value.filter(
        (t) => t.state === 'running' || t.state === 'waiting' || t.state === 'paused',
      )
    }
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

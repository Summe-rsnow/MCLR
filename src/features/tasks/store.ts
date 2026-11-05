import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { ipcClient, type TaskNode, type TaskStoreStatus } from '../../ipc/client'

export const useTasksStore = defineStore('tasks', () => {
  const tasks = ref<TaskNode[]>([])
  const loading = ref(false)
  const initialized = ref(false)
  const storeStatus = ref<TaskStoreStatus>({ mode: 'persistent', resetNotice: false })
  const speedSamples = ref<Record<string, number[]>>({})

  // Ids the user already cleared/dismissed: a broadcast that was in flight when the task was dropped must
  // not put it back on screen.
  const dismissed = new Set<string>()
  const unlistens: UnlistenFn[] = []

  const activeTasks = computed(() =>
    tasks.value.filter(
      (t) =>
        t.state === 'running' ||
        t.state === 'waiting' ||
        t.state === 'paused' ||
        t.state === 'retrying',
    ),
  )

  const interruptedTasks = computed(() => tasks.value.filter((t) => t.state === 'interrupted'))

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

  async function fetchTasks() {
    loading.value = true
    try {
      const [list, status] = await Promise.all([
        ipcClient.tasksList(),
        ipcClient
          .tasksStoreStatus()
          .catch(() => ({ mode: 'persistent' as const, resetNotice: false })),
      ])
      tasks.value = list
      storeStatus.value = status
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

      // Record speed sample for running task
      if (u.state === 'running') {
        const samples = speedSamples.value[u.id] || []
        samples.push(u.speedBps || 0)
        if (samples.length > 60) samples.shift()
        speedSamples.value[u.id] = samples
      }
    }
    tasks.value = next
  }

  function handleRemoved(ids: string[]) {
    for (const id of ids) dismissed.add(id)
    const gone = new Set(ids)
    tasks.value = tasks.value.filter((t) => !gone.has(t.id))
    for (const id of ids) {
      delete speedSamples.value[id]
    }
  }

  async function startListening() {
    if (unlistens.length > 0) return
    try {
      const u1 = await listen<TaskNode | TaskNode[]>('tasks://update', (event) => {
        handleTaskUpdate(event.payload)
      })
      unlistens.push(u1)

      const u2 = await listen<string[]>('tasks://removed', (event) => {
        handleRemoved(event.payload)
      })
      unlistens.push(u2)

      const u3 = await listen<TaskStoreStatus>('tasks://store', (event) => {
        storeStatus.value = event.payload
      })
      unlistens.push(u3)
    } catch {
      // Ignore if event system unavailable (e.g. non-Tauri browser mode)
    }
  }

  function stopListening() {
    while (unlistens.length > 0) {
      const u = unlistens.pop()
      u?.()
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

  async function restore(id: string) {
    await ipcClient.taskRestore(id)
    const t = tasks.value.find((x) => x.id === id)
    if (t) {
      t.state = 'waiting'
      t.error = null
    }
  }

  async function restoreAll() {
    const ids = await ipcClient.tasksRestoreAll()
    const idSet = new Set(ids)
    for (const t of tasks.value) {
      if (idSet.has(t.id)) {
        t.state = 'waiting'
        t.error = null
      }
    }
  }

  async function retry(id: string) {
    await ipcClient.taskRetry(id)
    const t = tasks.value.find((x) => x.id === id)
    if (t) {
      t.state = 'waiting'
      t.error = null
    }
  }

  async function remove(id: string) {
    await ipcClient.taskRemove(id)
    dismissed.add(id)
    tasks.value = tasks.value.filter((t) => t.id !== id)
    delete speedSamples.value[id]
  }

  async function reorder(orderedIds: string[]) {
    await ipcClient.tasksReorder(orderedIds)
    // Local sort reorder
    const waitingMap = new Map(orderedIds.map((id, index) => [id, index]))
    tasks.value.sort((a, b) => {
      const idxA = waitingMap.get(a.id)
      const idxB = waitingMap.get(b.id)
      if (idxA !== undefined && idxB !== undefined) return idxA - idxB
      return 0
    })
  }

  async function ackReset() {
    await ipcClient.tasksAckReset()
    storeStatus.value.resetNotice = false
  }

  async function clearFinished() {
    try {
      const removed = await ipcClient.tasksClearFinished()
      handleRemoved(removed)
    } catch {
      // Backend unavailable (tests, browser mock): fall back to hiding them locally.
      tasks.value = tasks.value.filter(
        (t) =>
          t.state === 'running' ||
          t.state === 'waiting' ||
          t.state === 'paused' ||
          t.state === 'retrying',
      )
    }
  }

  return {
    tasks,
    loading,
    initialized,
    storeStatus,
    speedSamples,
    activeTasks,
    interruptedTasks,
    activeTasksCount,
    aggregateSpeedBps,
    aggregateProgress,
    fetchTasks,
    startListening,
    stopListening,
    pause,
    resume,
    cancel,
    restore,
    restoreAll,
    retry,
    remove,
    reorder,
    ackReset,
    clearFinished,
  }
})

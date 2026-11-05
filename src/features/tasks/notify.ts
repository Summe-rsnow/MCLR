import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from '@tauri-apps/plugin-notification'
import type { TaskNode, TaskState } from '../../ipc/client'
import { ipcClient } from '../../ipc/client'
import { resolveTaskTitle } from './format'
import { i18n } from '../../i18n'

export interface TaskStoreNotifierSource {
  tasks: TaskNode[]
  $subscribe: (callback: (mutation: unknown, state: { tasks: TaskNode[] }) => void) => () => void
}

let permissionChecked = false
let hasPermission = false

async function ensurePermission(): Promise<boolean> {
  if (permissionChecked) return hasPermission
  try {
    let granted = await isPermissionGranted()
    if (!granted) {
      const permission = await requestPermission()
      granted = permission === 'granted'
    }
    hasPermission = granted
  } catch {
    hasPermission = false
  } finally {
    permissionChecked = true
  }
  return hasPermission
}

export function startTaskNotifications(store: TaskStoreNotifierSource): () => void {
  const previousStates = new Map<string, TaskState>()

  for (const t of store.tasks) {
    previousStates.set(t.id, t.state)
  }

  const unsubscribe = store.$subscribe(async (_mutation, state) => {
    if (typeof document !== 'undefined' && document.hasFocus()) {
      for (const t of state.tasks) {
        previousStates.set(t.id, t.state)
      }
      return
    }

    let notifyEnabled = true
    try {
      const s = await ipcClient.settingsGet()
      notifyEnabled = s.tasks?.notify ?? true
    } catch {
      notifyEnabled = true
    }

    if (!notifyEnabled) {
      for (const t of state.tasks) {
        previousStates.set(t.id, t.state)
      }
      return
    }

    const t = i18n.global.t
    const te = i18n.global.te

    for (const task of state.tasks) {
      const prev = previousStates.get(task.id)
      previousStates.set(task.id, task.state)

      if (task.kind === 'game.launch' || task.kind === 'auth.microsoft') {
        continue
      }

      if (prev && prev !== task.state && (task.state === 'success' || task.state === 'failed')) {
        const canNotify = await ensurePermission()
        if (!canNotify) continue

        const titleKey = task.state === 'success' ? 'tasks.notify.success' : 'tasks.notify.failed'
        const title = te(titleKey)
          ? t(titleKey)
          : task.state === 'success'
            ? 'Task Completed'
            : 'Task Failed'
        const body = resolveTaskTitle(task.titleKey, task.titleParams || {}, t, te)

        try {
          sendNotification({ title, body })
        } catch {
          // Silent in browser or tests
        }
      }
    }
  })

  return () => {
    unsubscribe()
  }
}

import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { ipcClient, type TaskNode, type TaskState } from '../../ipc/client'

/** Terminal states of a background task: nothing else will arrive for it. */
const TERMINAL_STATES: readonly TaskState[] = ['success', 'failed', 'canceled']

export interface TaskOutcome {
  /** `unknown` when the runtime has no task event stream (browser / tests). */
  state: TaskState | 'unknown'
  error: TaskNode['error'] | null
}

function isTerminal(state: TaskState | undefined): boolean {
  return state !== undefined && TERMINAL_STATES.includes(state)
}

/**
 * Waits for a background task to reach a terminal state.
 *
 * Install commands (modpack download → unpack → loaders → base game) return the task id
 * immediately and keep working in the background, so anything user-visible — a success
 * toast, a refreshed instance list — must wait for this instead of assuming completion.
 */
export async function awaitTask(taskId: string): Promise<TaskOutcome> {
  let unlisten: UnlistenFn | null = null
  let settle: (outcome: TaskOutcome) => void = () => {}
  const finished = new Promise<TaskOutcome>((resolve) => {
    settle = resolve
  })

  try {
    unlisten = await listen<TaskNode | TaskNode[]>('tasks://update', (event) => {
      const updates = Array.isArray(event.payload) ? event.payload : [event.payload]
      const node = updates.find((u) => u.id === taskId)
      if (node && isTerminal(node.state)) {
        settle({ state: node.state, error: node.error ?? null })
      }
    })
  } catch {
    return { state: 'unknown', error: null }
  }

  try {
    // The task can reach its terminal state before this listener is attached.
    const current = (await ipcClient.tasksList()).find((t) => t.id === taskId)
    if (current && isTerminal(current.state)) {
      return { state: current.state, error: current.error ?? null }
    }
    return await finished
  } catch {
    return { state: 'unknown', error: null }
  } finally {
    unlisten?.()
  }
}

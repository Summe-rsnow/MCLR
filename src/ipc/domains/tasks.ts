import { invokeCommand } from '../client'
import type { AppError } from '../bindings'

export type TaskState =
  | 'waiting'
  | 'running'
  | 'paused'
  | 'retrying'
  | 'interrupted'
  | 'success'
  | 'canceled'
  | 'failed'

export type StepState = 'pending' | 'running' | 'done' | 'failed' | 'skipped'

export interface StepNode {
  key: string
  state: StepState
  weight: number
  itemsDone: number
  itemsTotal: number
  startedAtMs?: number | null
  finishedAtMs?: number | null
  error?: AppError | null
}

export interface TaskNode {
  id: string
  kind: string
  titleKey: string
  titleParams: Record<string, string>
  state: TaskState
  progress: number
  speedBps: number
  steps: StepNode[]
  currentStep?: string | null
  attempt: number
  maxAttempts: number
  nextRetryAtMs?: number | null
  resumable: boolean
  queueSeq: number
  createdAtMs: number
  startedAtMs?: number | null
  finishedAtMs?: number | null
  error?: AppError | null
}

export interface TaskStoreStatus {
  mode: 'persistent' | 'memory'
  resetNotice: boolean
}

export interface TaskLeftovers {
  bytes: number
  files: number
}

export const tasksIpc = {
  tasksList: (): Promise<TaskNode[]> => invokeCommand<TaskNode[]>('tasks_list'),
  /** Drops finished tasks from the backend registry and returns the removed ids. */
  tasksClearFinished: (): Promise<string[]> => invokeCommand<string[]>('tasks_clear_finished'),
  taskCancel: (id: string): Promise<void> => invokeCommand<void>('task_cancel', { id }),
  taskPause: (id: string): Promise<void> => invokeCommand<void>('task_pause', { id }),
  taskResume: (id: string): Promise<void> => invokeCommand<void>('task_resume', { id }),
  taskRestore: (id: string): Promise<void> => invokeCommand<void>('task_restore', { id }),
  taskRetry: (id: string): Promise<void> => invokeCommand<void>('task_retry', { id }),
  taskRemove: (id: string): Promise<void> => invokeCommand<void>('task_remove', { id }),
  tasksRestoreAll: (): Promise<string[]> => invokeCommand<string[]>('tasks_restore_all'),
  tasksReorder: (orderedIds: string[]): Promise<void> =>
    invokeCommand<void>('tasks_reorder', { orderedIds }),
  tasksStoreStatus: (): Promise<TaskStoreStatus> =>
    invokeCommand<TaskStoreStatus>('tasks_store_status'),
  tasksAckReset: (): Promise<void> => invokeCommand<void>('tasks_ack_reset'),
  tasksLeftoversInfo: (): Promise<TaskLeftovers> =>
    invokeCommand<TaskLeftovers>('tasks_leftovers_info'),
  tasksCleanupLeftovers: (): Promise<TaskLeftovers> =>
    invokeCommand<TaskLeftovers>('tasks_cleanup_leftovers'),
}

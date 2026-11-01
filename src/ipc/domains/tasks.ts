import { invokeCommand } from '../client'
import type { AppError } from '../bindings'

export type TaskState = 'waiting' | 'running' | 'paused' | 'success' | 'canceled' | 'failed'

export interface TaskNode {
  id: string
  parentId?: string | null
  titleKey: string
  titleParams: Record<string, string>
  state: TaskState
  progress: number
  speedBps: number
  error?: AppError | null
}

export const tasksIpc = {
  tasksList: (): Promise<TaskNode[]> => invokeCommand<TaskNode[]>('tasks_list'),
  /** Drops finished tasks from the backend registry and returns the removed ids. */
  tasksClearFinished: (): Promise<string[]> => invokeCommand<string[]>('tasks_clear_finished'),
  taskCancel: (id: string): Promise<void> => invokeCommand<void>('task_cancel', { id }),
  taskPause: (id: string): Promise<void> => invokeCommand<void>('task_pause', { id }),
  taskResume: (id: string): Promise<void> => invokeCommand<void>('task_resume', { id }),
}

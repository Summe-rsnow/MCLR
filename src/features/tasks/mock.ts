import type { TaskNode } from '../../ipc/client'

export const mockTasks: TaskNode[] = [
  {
    id: 'task-dl-group-1',
    parentId: null,
    titleKey: 'task.download.group',
    titleParams: { count: '3' },
    state: 'running',
    progress: 0.65,
    speedBps: 2450000,
  },
  {
    id: 'task-dl-vanilla-1',
    parentId: 'task-dl-group-1',
    titleKey: 'task.download.clientJar',
    titleParams: { version: '1.20.1' },
    state: 'success',
    progress: 1.0,
    speedBps: 0,
  },
  {
    id: 'task-dl-libraries-1',
    parentId: 'task-dl-group-1',
    titleKey: 'task.download.libraries',
    titleParams: { current: '42', total: '65' },
    state: 'running',
    progress: 0.64,
    speedBps: 2450000,
  },
  {
    id: 'task-dl-assets-1',
    parentId: 'task-dl-group-1',
    titleKey: 'task.download.assets',
    titleParams: { count: '1024' },
    state: 'waiting',
    progress: 0,
    speedBps: 0,
  },
  {
    id: 'task-java-dl',
    parentId: null,
    titleKey: 'task.download.java',
    titleParams: { component: 'java-runtime-gamma' },
    state: 'paused',
    progress: 0.35,
    speedBps: 0,
  },
]

const handlers = {
  tasks_list: () => [...mockTasks],
  task_cancel: (args: Record<string, unknown>) => {
    const id = String(args.id)
    const task = mockTasks.find((t) => t.id === id)
    if (task) {
      task.state = 'canceled'
      task.speedBps = 0
    }
    return null
  },
  task_pause: (args: Record<string, unknown>) => {
    const id = String(args.id)
    const task = mockTasks.find((t) => t.id === id)
    if (task) {
      task.state = 'paused'
      task.speedBps = 0
    }
    return null
  },
  task_resume: (args: Record<string, unknown>) => {
    const id = String(args.id)
    const task = mockTasks.find((t) => t.id === id)
    if (task) {
      task.state = 'running'
      task.speedBps = 1500000
    }
    return null
  },
}

export default handlers

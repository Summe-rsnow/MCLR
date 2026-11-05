import type { TaskNode, TaskStoreStatus, TaskLeftovers } from '../../ipc/client'

const now = Date.now()

export const mockTasks: TaskNode[] = [
  {
    id: 'task-running-1',
    kind: 'install.create',
    titleKey: 'tasks.item.install.create',
    titleParams: { name: 'Fabric 1.20.1', mcVersion: '1.20.1' },
    state: 'running',
    progress: 0.65,
    speedBps: 3500000,
    steps: [
      {
        key: 'staging',
        state: 'done',
        weight: 0.3,
        itemsDone: 1,
        itemsTotal: 1,
        startedAtMs: now - 30000,
        finishedAtMs: now - 25000,
      },
      {
        key: 'merge',
        state: 'done',
        weight: 0.05,
        itemsDone: 1,
        itemsTotal: 1,
        startedAtMs: now - 25000,
        finishedAtMs: now - 24000,
      },
      {
        key: 'libraries',
        state: 'done',
        weight: 0.05,
        itemsDone: 42,
        itemsTotal: 42,
        startedAtMs: now - 24000,
        finishedAtMs: now - 20000,
      },
      {
        key: 'repair',
        state: 'running',
        weight: 0.6,
        itemsDone: 120,
        itemsTotal: 340,
        startedAtMs: now - 20000,
      },
    ],
    currentStep: 'repair',
    attempt: 0,
    maxAttempts: 4,
    nextRetryAtMs: null,
    resumable: true,
    queueSeq: 1,
    createdAtMs: now - 32000,
    startedAtMs: now - 30000,
  },
  {
    id: 'task-waiting-1',
    kind: 'comp.download',
    titleKey: 'tasks.titles.compDownload',
    titleParams: { name: 'Sodium', fileName: 'sodium-0.5.8.jar' },
    state: 'waiting',
    progress: 0,
    speedBps: 0,
    steps: [
      {
        key: 'download',
        state: 'pending',
        weight: 1.0,
        itemsDone: 0,
        itemsTotal: 0,
      },
    ],
    currentStep: null,
    attempt: 0,
    maxAttempts: 4,
    nextRetryAtMs: null,
    resumable: true,
    queueSeq: 2,
    createdAtMs: now - 10000,
  },
  {
    id: 'task-waiting-2',
    kind: 'comp.download',
    titleKey: 'tasks.titles.compDownload',
    titleParams: { name: 'Iris Shaders', fileName: 'iris-1.7.0.jar' },
    state: 'waiting',
    progress: 0,
    speedBps: 0,
    steps: [
      {
        key: 'download',
        state: 'pending',
        weight: 1.0,
        itemsDone: 0,
        itemsTotal: 0,
      },
    ],
    currentStep: null,
    attempt: 0,
    maxAttempts: 4,
    nextRetryAtMs: null,
    resumable: true,
    queueSeq: 3,
    createdAtMs: now - 8000,
  },
  {
    id: 'task-paused-1',
    kind: 'tools.customDownload',
    titleKey: 'tasks.titles.customDownload',
    titleParams: { name: 'OpenJDK21U.zip' },
    state: 'paused',
    progress: 0.45,
    speedBps: 0,
    steps: [
      {
        key: 'download',
        state: 'running',
        weight: 1.0,
        itemsDone: 0,
        itemsTotal: 0,
        startedAtMs: now - 60000,
      },
    ],
    currentStep: 'download',
    attempt: 0,
    maxAttempts: 4,
    nextRetryAtMs: null,
    resumable: true,
    queueSeq: 4,
    createdAtMs: now - 65000,
    startedAtMs: now - 60000,
  },
  {
    id: 'task-retrying-countdown',
    kind: 'modpack.install',
    titleKey: 'tasks.titles.modpackInstall',
    titleParams: { name: 'Fabulously Optimized', fileName: 'FO-5.8.0.mrpack' },
    state: 'retrying',
    progress: 0.12,
    speedBps: 0,
    steps: [
      {
        key: 'download',
        state: 'running',
        weight: 0.1,
        itemsDone: 0,
        itemsTotal: 0,
        startedAtMs: now - 40000,
      },
      {
        key: 'inspect',
        state: 'pending',
        weight: 0.05,
        itemsDone: 0,
        itemsTotal: 0,
      },
      {
        key: 'baseGame',
        state: 'pending',
        weight: 0.55,
        itemsDone: 0,
        itemsTotal: 0,
      },
      {
        key: 'files',
        state: 'pending',
        weight: 0.3,
        itemsDone: 0,
        itemsTotal: 0,
      },
    ],
    currentStep: 'download',
    attempt: 1,
    maxAttempts: 4,
    nextRetryAtMs: now + 5000,
    resumable: true,
    queueSeq: 5,
    createdAtMs: now - 45000,
    startedAtMs: now - 40000,
    error: {
      code: 'net.timeout',
      detail: 'Connection timed out',
    },
  },
  {
    id: 'task-retrying-waiting-network',
    kind: 'install.repair',
    titleKey: 'tasks.item.install.repair',
    titleParams: { instanceId: 'Vanilla 1.21' },
    state: 'retrying',
    progress: 0.2,
    speedBps: 0,
    steps: [
      {
        key: 'resolve',
        state: 'done',
        weight: 0.05,
        itemsDone: 1,
        itemsTotal: 1,
        startedAtMs: now - 50000,
        finishedAtMs: now - 48000,
      },
      {
        key: 'verify',
        state: 'done',
        weight: 0.15,
        itemsDone: 50,
        itemsTotal: 50,
        startedAtMs: now - 48000,
        finishedAtMs: now - 45000,
      },
      {
        key: 'download',
        state: 'running',
        weight: 0.8,
        itemsDone: 5,
        itemsTotal: 80,
        startedAtMs: now - 45000,
      },
    ],
    currentStep: 'download',
    attempt: 2,
    maxAttempts: 4,
    nextRetryAtMs: null,
    resumable: true,
    queueSeq: 6,
    createdAtMs: now - 55000,
    startedAtMs: now - 50000,
    error: {
      code: 'net.sourcesExhausted',
      detail: 'All mirror sources offline',
    },
  },
  {
    id: 'task-interrupted-resumable',
    kind: 'presets.apply',
    titleKey: 'tasks.titles.presetApply',
    titleParams: { instance: 'MyModded1.20', count: '12' },
    state: 'interrupted',
    progress: 0.25,
    speedBps: 0,
    steps: [
      {
        key: 'resolve',
        state: 'done',
        weight: 0.15,
        itemsDone: 12,
        itemsTotal: 12,
        startedAtMs: now - 120000,
        finishedAtMs: now - 110000,
      },
      {
        key: 'scan',
        state: 'done',
        weight: 0.05,
        itemsDone: 1,
        itemsTotal: 1,
        startedAtMs: now - 110000,
        finishedAtMs: now - 105000,
      },
      {
        key: 'plan',
        state: 'done',
        weight: 0.05,
        itemsDone: 12,
        itemsTotal: 12,
        startedAtMs: now - 105000,
        finishedAtMs: now - 100000,
      },
      {
        key: 'install',
        state: 'pending',
        weight: 0.75,
        itemsDone: 0,
        itemsTotal: 12,
      },
    ],
    currentStep: null,
    attempt: 0,
    maxAttempts: 4,
    nextRetryAtMs: null,
    resumable: true,
    queueSeq: 7,
    createdAtMs: now - 130000,
    startedAtMs: now - 120000,
  },
  {
    id: 'task-interrupted-non-resumable',
    kind: 'update.download',
    titleKey: 'tasks.titles.update.download',
    titleParams: { channel: 'stable' },
    state: 'interrupted',
    progress: 0.5,
    speedBps: 0,
    steps: [
      {
        key: 'check',
        state: 'done',
        weight: 0.02,
        itemsDone: 1,
        itemsTotal: 1,
        startedAtMs: now - 90000,
        finishedAtMs: now - 88000,
      },
      {
        key: 'download',
        state: 'pending',
        weight: 0.98,
        itemsDone: 0,
        itemsTotal: 0,
      },
    ],
    currentStep: null,
    attempt: 0,
    maxAttempts: 1,
    nextRetryAtMs: null,
    resumable: false,
    queueSeq: 8,
    createdAtMs: now - 95000,
    startedAtMs: now - 90000,
  },
  {
    id: 'task-failed-1',
    kind: 'export.modpack',
    titleKey: 'tasks.titles.export.title',
    titleParams: {
      instanceId: 'TestInstance',
      packName: 'TestPack',
      outputPath: 'C:/export.mrpack',
    },
    state: 'failed',
    progress: 0.7,
    speedBps: 0,
    steps: [
      {
        key: 'collect',
        state: 'done',
        weight: 0.1,
        itemsDone: 1,
        itemsTotal: 1,
        startedAtMs: now - 80000,
        finishedAtMs: now - 78000,
      },
      {
        key: 'hash',
        state: 'done',
        weight: 0.6,
        itemsDone: 45,
        itemsTotal: 45,
        startedAtMs: now - 78000,
        finishedAtMs: now - 70000,
      },
      {
        key: 'write',
        state: 'failed',
        weight: 0.3,
        itemsDone: 10,
        itemsTotal: 45,
        startedAtMs: now - 70000,
        finishedAtMs: now - 68000,
        error: {
          code: 'core.io',
          detail: 'Disk full (os error 112)',
        },
      },
    ],
    currentStep: null,
    attempt: 1,
    maxAttempts: 1,
    nextRetryAtMs: null,
    resumable: true,
    queueSeq: 9,
    createdAtMs: now - 85000,
    startedAtMs: now - 80000,
    finishedAtMs: now - 68000,
    error: {
      code: 'core.io',
      detail: 'Disk full (os error 112)',
    },
  },
  {
    id: 'task-success-1',
    kind: 'install.vanilla',
    titleKey: 'tasks.item.download.group',
    titleParams: { version: '1.20.1', name: 'Vanilla 1.20.1' },
    state: 'success',
    progress: 1.0,
    speedBps: 0,
    steps: [
      {
        key: 'manifest',
        state: 'done',
        weight: 0.03,
        itemsDone: 1,
        itemsTotal: 1,
        startedAtMs: now - 300000,
        finishedAtMs: now - 298000,
      },
      {
        key: 'metadata',
        state: 'done',
        weight: 0.05,
        itemsDone: 1,
        itemsTotal: 1,
        startedAtMs: now - 298000,
        finishedAtMs: now - 295000,
      },
      {
        key: 'verify',
        state: 'done',
        weight: 0.12,
        itemsDone: 80,
        itemsTotal: 80,
        startedAtMs: now - 295000,
        finishedAtMs: now - 290000,
      },
      {
        key: 'download',
        state: 'done',
        weight: 0.78,
        itemsDone: 80,
        itemsTotal: 80,
        startedAtMs: now - 290000,
        finishedAtMs: now - 240000,
      },
      {
        key: 'finalize',
        state: 'done',
        weight: 0.02,
        itemsDone: 1,
        itemsTotal: 1,
        startedAtMs: now - 240000,
        finishedAtMs: now - 239000,
      },
    ],
    currentStep: null,
    attempt: 0,
    maxAttempts: 4,
    nextRetryAtMs: null,
    resumable: true,
    queueSeq: 10,
    createdAtMs: now - 305000,
    startedAtMs: now - 300000,
    finishedAtMs: now - 239000,
  },
]

let storeStatus: TaskStoreStatus = {
  mode: 'persistent',
  resetNotice: false,
}

const leftovers: TaskLeftovers = {
  bytes: 14500000,
  files: 3,
}

const handlers = {
  tasks_list: () => [...mockTasks],
  task_cancel: (args: Record<string, unknown>) => {
    const id = args.id as string
    const target = mockTasks.find((t) => t.id === id)
    if (target) {
      target.state = 'canceled'
      target.speedBps = 0
    }
    return null
  },
  task_pause: (args: Record<string, unknown>) => {
    const id = args.id as string
    const target = mockTasks.find((t) => t.id === id)
    if (target && target.state === 'running') {
      target.state = 'paused'
      target.speedBps = 0
    }
    return null
  },
  task_resume: (args: Record<string, unknown>) => {
    const id = args.id as string
    const target = mockTasks.find((t) => t.id === id)
    if (target && target.state === 'paused') {
      target.state = 'running'
    }
    return null
  },
  task_restore: (args: Record<string, unknown>) => {
    const id = args.id as string
    const target = mockTasks.find((t) => t.id === id)
    if (target && target.state === 'interrupted' && target.resumable) {
      target.state = 'waiting'
      target.error = null
    }
    return null
  },
  task_retry: (args: Record<string, unknown>) => {
    const id = args.id as string
    const target = mockTasks.find((t) => t.id === id)
    if (target && target.state === 'failed' && target.resumable) {
      target.state = 'waiting'
      target.error = null
    }
    return null
  },
  task_remove: (args: Record<string, unknown>) => {
    const id = args.id as string
    const idx = mockTasks.findIndex((t) => t.id === id)
    if (idx !== -1) {
      mockTasks.splice(idx, 1)
    }
    return null
  },
  tasks_restore_all: () => {
    const restored: string[] = []
    for (const t of mockTasks) {
      if (t.state === 'interrupted' && t.resumable) {
        t.state = 'waiting'
        t.error = null
        restored.push(t.id)
      }
    }
    return restored
  },
  tasks_reorder: (args: Record<string, unknown>) => {
    const orderedIds = args.orderedIds as string[]
    const waitingMap = new Map(orderedIds.map((id, index) => [id, index]))
    mockTasks.sort((a, b) => {
      const idxA = waitingMap.get(a.id)
      const idxB = waitingMap.get(b.id)
      if (idxA !== undefined && idxB !== undefined) return idxA - idxB
      return 0
    })
    return null
  },
  tasks_store_status: () => ({ ...storeStatus }),
  tasks_ack_reset: () => {
    storeStatus.resetNotice = false
    return null
  },
  tasks_clear_finished: () => {
    const finishedIds: string[] = []
    for (let i = mockTasks.length - 1; i >= 0; i--) {
      const state = mockTasks[i].state
      if (state === 'success' || state === 'failed' || state === 'canceled') {
        finishedIds.push(mockTasks[i].id)
        mockTasks.splice(i, 1)
      }
    }
    return finishedIds
  },
  tasks_leftovers_info: () => ({ ...leftovers }),
  tasks_cleanup_leftovers: () => {
    const result = { ...leftovers }
    leftovers.bytes = 0
    leftovers.files = 0
    return result
  },
}

export default handlers

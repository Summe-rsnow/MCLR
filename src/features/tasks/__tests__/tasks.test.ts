import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { mount, flushPromises } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createRouter, createMemoryHistory } from 'vue-router'
import { useTasksStore } from '../store'
import { awaitTask } from '../awaitTask'
import TasksPage from '../TasksPage.vue'
import TaskIndicator from '../TaskIndicator.vue'
import { useDialog } from '../../../ui/useDialog'
import { setupMockIpc } from '../../../ipc/mock'
import { mockTasks } from '../mock'
import { emit } from '@tauri-apps/api/event'
import zhCnTasks from '../../../locales/zh-CN/tasks.json'
import zhCnCommon from '../../../locales/zh-CN/common.json'

const i18n = createI18n({
  legacy: false,
  locale: 'zh-CN',
  fallbackLocale: 'zh-CN',
  messages: {
    'zh-CN': {
      tasks: zhCnTasks,
      common: zhCnCommon,
    },
  },
})

/** The tasks mock is module state shared by every test here, so mutations are rolled back. */
async function withFreshTasksMock(run: () => Promise<void>): Promise<void> {
  const snapshot = mockTasks.map((t) => ({ ...t }))
  try {
    await run()
  } finally {
    mockTasks.length = 0
    mockTasks.push(...snapshot)
  }
}

describe('Tasks feature', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    setupMockIpc()
  })

  it('clears finished tasks in the backend so they do not come back on the next fetch', async () => {
    await withFreshTasksMock(async () => {
      const store = useTasksStore()
      await store.fetchTasks()
      const taskDone = store.tasks.find((t) => t.state === 'success')
      expect(taskDone).toBeDefined()

      await store.clearFinished()
      expect(store.tasks.some((t) => t.id === taskDone!.id)).toBe(false)
      await store.fetchTasks()
      expect(store.tasks.some((t) => t.id === taskDone!.id)).toBe(false)
    })
  })

  it('ignores a late update for a task the user just cleared', async () => {
    await withFreshTasksMock(async () => {
      const store = useTasksStore()
      await store.fetchTasks()
      const pausedTask = store.tasks.find((t) => t.state === 'paused')
      expect(pausedTask).toBeDefined()

      await store.startListening()
      await store.cancel(pausedTask!.id)
      await store.clearFinished()
      expect(store.tasks.some((t) => t.id === pausedTask!.id)).toBe(false)

      // Late update arrives
      await emit('tasks://update', { ...pausedTask!, state: 'success', progress: 1, speedBps: 0 })
      expect(store.tasks.some((t) => t.id === pausedTask!.id)).toBe(false)
      store.stopListening()
    })
  })

  it('counts active, retrying, and interrupted tasks correctly', async () => {
    await withFreshTasksMock(async () => {
      const store = useTasksStore()
      await store.fetchTasks()

      // Active includes running, waiting x2, paused, retrying x2 = 6 active tasks
      expect(store.activeTasksCount).toBe(6)
      // Interrupted includes 2 tasks
      expect(store.interruptedTasks.length).toBe(2)
      expect(store.aggregateSpeedBps).toBe(3500000)
    })
  })

  it('awaitTask resolves on interrupted state as well as success/failed/canceled', async () => {
    const promise = awaitTask('task-await-test')
    await emit('tasks://update', {
      id: 'task-await-test',
      kind: 'test',
      titleKey: 'test',
      titleParams: {},
      state: 'interrupted',
      progress: 0.5,
      speedBps: 0,
      steps: [],
      attempt: 0,
      maxAttempts: 1,
      resumable: true,
      queueSeq: 1,
      createdAtMs: Date.now(),
    })
    const outcome = await promise
    expect(outcome.state).toBe('interrupted')
  })

  it('can pause, resume, cancel, restore, retry, and remove tasks', async () => {
    await withFreshTasksMock(async () => {
      const store = useTasksStore()
      await store.fetchTasks()

      // Pause & Resume
      await store.pause('task-running-1')
      expect(store.tasks.find((t) => t.id === 'task-running-1')?.state).toBe('paused')
      await store.resume('task-running-1')
      expect(store.tasks.find((t) => t.id === 'task-running-1')?.state).toBe('running')

      // Cancel
      await store.cancel('task-running-1')
      expect(store.tasks.find((t) => t.id === 'task-running-1')?.state).toBe('canceled')

      // Restore interrupted
      await store.restore('task-interrupted-resumable')
      expect(store.tasks.find((t) => t.id === 'task-interrupted-resumable')?.state).toBe('waiting')

      // Retry failed
      await store.retry('task-failed-1')
      expect(store.tasks.find((t) => t.id === 'task-failed-1')?.state).toBe('waiting')

      // Remove
      await store.remove('task-success-1')
      expect(store.tasks.some((t) => t.id === 'task-success-1')).toBe(false)
    })
  })

  it('renders TasksPage with tasks list and details', async () => {
    await withFreshTasksMock(async () => {
      const wrapper = mount(TasksPage, {
        global: {
          plugins: [i18n],
        },
      })
      await flushPromises()

      expect(wrapper.text()).toContain('任务中心')
      expect(wrapper.text()).toContain('Fabric 1.20.1')

      // Selecting interrupted task shows restore button
      const interruptedRow = wrapper
        .findAll('[data-testid="task-item"]')
        .find((el) => el.text().includes('应用资源预设'))
      expect(interruptedRow).toBeDefined()
      await interruptedRow!.trigger('click')
      await flushPromises()

      expect(wrapper.text()).toContain('恢复')
    })
  })

  it('prompts confirmDanger when canceling a task via UI', async () => {
    await withFreshTasksMock(async () => {
      const store = useTasksStore()
      await store.fetchTasks()
      const { activeDialog, handleConfirm } = useDialog()

      const wrapper = mount(TasksPage, {
        global: {
          plugins: [i18n],
        },
      })
      await flushPromises()

      const cancelBtn = wrapper.find('[data-testid="task-cancel-btn"]')
      expect(cancelBtn.exists()).toBe(true)
      await cancelBtn.trigger('click')
      await flushPromises()

      expect(activeDialog.value?.open).toBe(true)
      expect(activeDialog.value?.severity).toBe('danger')

      handleConfirm()
      await flushPromises()

      expect(store.tasks.find((t) => t.id === 'task-running-1')?.state).toBe('canceled')
    })
  })

  it('renders TaskIndicator and navigates to /tasks on click', async () => {
    await withFreshTasksMock(async () => {
      const store = useTasksStore()
      await store.fetchTasks()

      const testRouter = createRouter({
        history: createMemoryHistory(),
        routes: [{ path: '/tasks', component: { template: '<div>tasks</div>' } }],
      })
      const pushSpy = vi.spyOn(testRouter, 'push')

      const wrapper = mount(TaskIndicator, {
        global: {
          plugins: [i18n, testRouter],
        },
      })

      expect(wrapper.text()).toContain('个任务进行中')
      await wrapper.find('button').trigger('click')
      expect(pushSpy).toHaveBeenCalledWith('/tasks')
    })
  })
})

import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { mount, flushPromises } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createRouter, createMemoryHistory } from 'vue-router'
import { useTasksStore } from '../store'
import TasksPage from '../TasksPage.vue'
import TaskIndicator from '../TaskIndicator.vue'
import { useDialog } from '../../../ui/useDialog'
import { setupMockIpc } from '../../../ipc/mock'
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
    zh: {
      tasks: zhCnTasks,
      common: zhCnCommon,
    },
  },
})

describe('Tasks feature', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    setupMockIpc()
  })

  it('loads tasks and computes tree structure and aggregate speed', async () => {
    const store = useTasksStore()
    await store.fetchTasks()

    expect(store.tasks.length).toBeGreaterThan(0)
    expect(store.activeTasksCount).toBeGreaterThan(0)
    expect(store.aggregateSpeedBps).toBeGreaterThan(0)

    // Verify tree computation
    const roots = store.taskTree
    expect(roots.length).toBeGreaterThan(0)
    const groupTask = roots.find((r) => r.id === 'task-dl-group-1')
    expect(groupTask).toBeDefined()
    expect(groupTask!.children.length).toBe(3)
  })

  it('can pause, resume, and cancel tasks', async () => {
    const store = useTasksStore()
    await store.fetchTasks()

    await store.pause('task-dl-group-1')
    expect(store.tasks.find((t) => t.id === 'task-dl-group-1')?.state).toBe('paused')

    await store.resume('task-dl-group-1')
    expect(store.tasks.find((t) => t.id === 'task-dl-group-1')?.state).toBe('running')

    await store.cancel('task-dl-group-1')
    expect(store.tasks.find((t) => t.id === 'task-dl-group-1')?.state).toBe('canceled')
  })

  it('renders TasksPage with tasks list', async () => {
    const wrapper = mount(TasksPage, {
      global: {
        plugins: [i18n],
      },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('任务列表')
    expect(wrapper.text()).toContain('批量下载')
  })

  it('prompts confirmDanger when canceling a task via UI', async () => {
    const store = useTasksStore()
    await store.fetchTasks()
    const { activeDialog, handleConfirm } = useDialog()

    const wrapper = mount(TasksPage, {
      global: {
        plugins: [i18n],
      },
    })
    await flushPromises()

    const cancelBtn = wrapper.find('button[title="取消"]')
    expect(cancelBtn.exists()).toBe(true)

    await cancelBtn.trigger('click')
    await flushPromises()

    expect(activeDialog.value?.open).toBe(true)
    expect(activeDialog.value?.severity).toBe('danger')

    handleConfirm()
    await flushPromises()

    expect(store.tasks.find((t) => t.id === 'task-dl-group-1')?.state).toBe('canceled')
  })

  it('renders TaskIndicator and navigates to /logs?tab=tasks on click', async () => {
    const store = useTasksStore()
    await store.fetchTasks()

    const testRouter = createRouter({
      history: createMemoryHistory(),
      routes: [{ path: '/logs', component: { template: '<div>logs</div>' } }],
    })
    const pushSpy = vi.spyOn(testRouter, 'push')

    const wrapper = mount(TaskIndicator, {
      global: {
        plugins: [i18n, testRouter],
      },
    })

    expect(wrapper.text()).toContain('任务进行中')
    await wrapper.find('button').trigger('click')
    expect(pushSpy).toHaveBeenCalledWith('/logs?tab=tasks')
  })
})

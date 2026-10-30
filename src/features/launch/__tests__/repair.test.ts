import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createRouter, createWebHistory } from 'vue-router'
import { i18n } from '../../../i18n'
import { setupMockIpc } from '../../../ipc/mock'
import { ipcClient } from '../../../ipc/client'
import { useToast } from '../../../ui/useToast'
import { useInstancesStore } from '../../instances/store'
import { useLaunchStore } from '../store'
import type { VueWrapper } from '@vue/test-utils'
import { awaitTask, type TaskOutcome } from '../../tasks/awaitTask'
import LaunchHeroBanner from '../LaunchHeroBanner.vue'

vi.mock('../../tasks/awaitTask', () => ({ awaitTask: vi.fn() }))

const awaitTaskMock = vi.mocked(awaitTask)

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', component: { template: '<div>home</div>' } },
    { path: '/java', component: { template: '<div>java</div>' } },
    { path: '/instances', component: { template: '<div>instances</div>' } },
  ],
})

let pinia = createPinia()

async function mountBanner() {
  const instancesStore = useInstancesStore()
  await instancesStore.fetchInstances()
  useLaunchStore()

  const wrapper = mount(LaunchHeroBanner, {
    props: { isRepairing: false },
    global: {
      plugins: [pinia, i18n, router],
    },
  })
  await flushPromises()
  return wrapper
}

async function clickRepair(wrapper: VueWrapper) {
  const button = wrapper.findAll('button').find((b) => b.text().includes('检查与修复'))
  expect(button).toBeDefined()
  await button!.trigger('click')
  await flushPromises()
}

describe('LaunchHeroBanner repair', () => {
  beforeEach(() => {
    pinia = createPinia()
    setActivePinia(pinia)
    setupMockIpc()
    i18n.global.locale.value = 'zh-CN'
    useToast().toasts.value.splice(0)
    awaitTaskMock.mockReset()
  })

  it('reports success only after the repair task reaches its terminal state', async () => {
    const wrapper = await mountBanner()
    const repairSpy = vi.spyOn(ipcClient, 'installRepair').mockResolvedValue('task-repair-1')

    const { promise, resolve } = Promise.withResolvers<TaskOutcome>()
    awaitTaskMock.mockImplementation(() => promise)

    await clickRepair(wrapper)

    // Queued, not finished: no toast yet, and the button stays busy.
    expect(repairSpy).toHaveBeenCalledWith(useLaunchStore().selectedInstance?.id)
    expect(awaitTaskMock).toHaveBeenCalledWith('task-repair-1')
    expect(useToast().toasts.value).toHaveLength(0)
    expect(wrapper.emitted('update:isRepairing')?.[0]).toEqual([true])

    resolve({ state: 'success', error: null })
    await flushPromises()

    expect(useToast().toasts.value.at(-1)?.message).toBe('检查与修复完成')
    expect(wrapper.emitted('update:isRepairing')?.at(-1)).toEqual([false])
  })

  it('surfaces a failed repair instead of claiming success', async () => {
    const wrapper = await mountBanner()
    vi.spyOn(ipcClient, 'installRepair').mockResolvedValue('task-repair-2')
    awaitTaskMock.mockResolvedValue({ state: 'failed', error: null })

    await clickRepair(wrapper)

    const toast = useToast().toasts.value.at(-1)
    expect(toast?.type).toBe('danger')
    expect(toast?.message).toBe('检查与修复失败')
    expect(wrapper.emitted('update:isRepairing')?.at(-1)).toEqual([false])
  })
})

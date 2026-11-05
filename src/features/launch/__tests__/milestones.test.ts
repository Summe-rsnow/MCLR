import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { mount, flushPromises } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { useLaunchStore } from '../store'
import SponsorMilestoneDialog from '../components/SponsorMilestoneDialog.vue'
import zhCnLaunch from '../../../locales/zh-CN/launch.json'
import zhCnCommon from '../../../locales/zh-CN/common.json'
import { router } from '../../../router'

const i18n = createI18n({
  legacy: false,
  locale: 'zh-CN',
  messages: {
    'zh-CN': {
      launch: zhCnLaunch,
      common: zhCnCommon,
    },
  },
})

describe('SponsorMilestoneDialog', () => {
  beforeEach(() => {
    document.body.innerHTML = ''
    setActivePinia(createPinia())
  })

  afterEach(() => {
    document.body.innerHTML = ''
    vi.restoreAllMocks()
  })

  it('renders milestone dialog for milestone 10 with approved copy', async () => {
    const launchStore = useLaunchStore()
    launchStore.activeMilestone = 10

    mount(SponsorMilestoneDialog, {
      global: {
        plugins: [i18n, router],
      },
    })

    await flushPromises()
    expect(document.body.querySelector('[data-testid="sponsor-milestone-dialog"]')).not.toBeNull()
    expect(document.body.textContent).toContain('冒险渐入佳境！')
    expect(document.body.textContent).toContain('启程 10 次')
    expect(document.body.textContent).toContain('去赞助')
    expect(document.body.textContent).toContain('稍后再说')
    expect(document.body.textContent).toContain('#10')
  })

  it('renders milestone dialog for all supported milestones (20, 50, 100, 200, 500)', async () => {
    const launchStore = useLaunchStore()
    const milestones = [20, 50, 100, 200, 500]

    for (const m of milestones) {
      document.body.innerHTML = ''
      launchStore.activeMilestone = m
      mount(SponsorMilestoneDialog, {
        global: {
          plugins: [i18n, router],
        },
      })
      await flushPromises()
      expect(document.body.textContent).toContain(`#${m}`)
    }
  })

  it('clicking confirm triggers dismissMilestone(true) and navigates to sponsor page', async () => {
    const launchStore = useLaunchStore()
    launchStore.activeMilestone = 50
    const pushSpy = vi.spyOn(router, 'push').mockImplementation(async () => {})

    mount(SponsorMilestoneDialog, {
      global: {
        plugins: [i18n, router],
      },
    })
    await flushPromises()

    const confirmBtn = Array.from(document.body.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('犒劳作者'),
    )
    expect(confirmBtn).toBeDefined()
    confirmBtn!.click()
    await flushPromises()

    expect(launchStore.activeMilestone).toBeNull()
    expect(pushSpy).toHaveBeenCalledWith('/settings/sponsor')
  })

  it('clicking cancel dismisses milestone without navigation', async () => {
    const launchStore = useLaunchStore()
    launchStore.activeMilestone = 100
    const pushSpy = vi.spyOn(router, 'push').mockImplementation(async () => {})

    mount(SponsorMilestoneDialog, {
      global: {
        plugins: [i18n, router],
      },
    })
    await flushPromises()

    const cancelBtn = Array.from(document.body.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('默默陪伴'),
    )
    expect(cancelBtn).toBeDefined()
    cancelBtn!.click()
    await flushPromises()

    expect(launchStore.activeMilestone).toBeNull()
    expect(pushSpy).not.toHaveBeenCalled()
  })
})

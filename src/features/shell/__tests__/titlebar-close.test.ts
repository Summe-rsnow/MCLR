import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import type { VueWrapper } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { i18n } from '../../../i18n'
import { setupMockIpc } from '../../../ipc/mock'
import { ipcClient } from '../../../ipc/client'
import TitleBar from '../TitleBar.vue'

const closeSpy = vi.fn(async () => {})

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({
    close: closeSpy,
    isMaximized: async () => false,
    onResized: async () => () => {},
    minimize: async () => {},
    toggleMaximize: async () => {},
  }),
}))

let wrapper: VueWrapper | undefined

function mountTitleBar() {
  // The dialog teleports to `body`, so the component has to live in the document too.
  wrapper = mount(TitleBar, {
    attachTo: document.body,
    global: {
      plugins: [createPinia(), i18n],
    },
  })
  return wrapper
}

function click(el: Element | null | undefined) {
  expect(el).toBeTruthy()
  ;(el as HTMLElement).click()
}

async function clickClose() {
  const button = [...document.querySelectorAll('header button')].find(
    (b) => b.getAttribute('title') === '关闭',
  )
  click(button)
  await flushPromises()
}

function prompt() {
  return document.querySelector('[data-testid="close-prompt"]')
}

describe('TitleBar close behaviour', () => {
  beforeEach(() => {
    document.body.innerHTML = ''
    setActivePinia(createPinia())
    setupMockIpc()
    i18n.global.locale.value = 'zh-CN'
    closeSpy.mockClear()
  })

  afterEach(() => {
    wrapper?.unmount()
    wrapper = undefined
  })

  it('asks on the first close instead of closing straight away', async () => {
    mountTitleBar()
    await flushPromises()

    await clickClose()

    expect(prompt()).toBeTruthy()
    expect(prompt()!.textContent).toContain('隐藏到托盘，继续后台运行')
    expect(closeSpy).not.toHaveBeenCalled()
  })

  it('hides to tray and keeps asking when 不再提醒 is not ticked', async () => {
    mountTitleBar()
    await flushPromises()
    await clickClose()

    click(document.querySelector('[data-testid="close-prompt-hide"]'))
    await flushPromises()

    const saved = await ipcClient.settingsGet()
    expect(saved.system?.keepInBackground).toBe(true)
    expect(saved.system?.closePromptDismissed).toBe(false)
    expect(closeSpy).toHaveBeenCalledTimes(1)

    // Still undecided, so the next close asks again.
    closeSpy.mockClear()
    await clickClose()
    expect(prompt()).toBeTruthy()
    expect(closeSpy).not.toHaveBeenCalled()
  })

  it('remembers the answer and stops asking once 不再提醒 is ticked', async () => {
    mountTitleBar()
    await flushPromises()
    await clickClose()

    const checkbox = document.querySelector(
      '[data-testid="close-prompt"] input[type="checkbox"]',
    ) as HTMLInputElement
    checkbox.checked = true
    checkbox.dispatchEvent(new Event('change'))
    await flushPromises()

    click(document.querySelector('[data-testid="close-prompt-quit"]'))
    await flushPromises()

    const saved = await ipcClient.settingsGet()
    expect(saved.system?.keepInBackground).toBe(false)
    expect(saved.system?.closePromptDismissed).toBe(true)
    expect(closeSpy).toHaveBeenCalledTimes(1)

    // A fresh window honours the stored answer without prompting.
    wrapper?.unmount()
    document.body.innerHTML = ''
    closeSpy.mockClear()
    mountTitleBar()
    await flushPromises()
    await clickClose()
    expect(prompt()).toBeNull()
    expect(closeSpy).toHaveBeenCalledTimes(1)
  })

  it('does not close when the prompt is cancelled', async () => {
    mountTitleBar()
    await flushPromises()
    await clickClose()

    const cancel = [...document.querySelectorAll('button')].find((b) => b.textContent === '取消')
    click(cancel)
    await flushPromises()

    expect(closeSpy).not.toHaveBeenCalled()
    const saved = await ipcClient.settingsGet()
    expect(saved.system?.closePromptDismissed).toBe(false)
  })
})

import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import DevUiPage from '../DevUiPage.vue'
import zhCnDevUi from '../../../locales/zh-CN/devUi.json'
import zhCnCommon from '../../../locales/zh-CN/common.json'

// Mock tauri plugins
vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
  save: vi.fn(),
}))

vi.mock('@tauri-apps/plugin-opener', () => ({
  openUrl: vi.fn(),
}))

describe('DevUiPage', () => {
  it('renders all 8 demo-* section containers in exact order', () => {
    const pinia = createPinia()
    setActivePinia(pinia)

    const i18n = createI18n({
      legacy: false,
      locale: 'zh-CN',
      messages: {
        'zh-CN': {
          devUi: zhCnDevUi,
          common: zhCnCommon,
        },
      },
    })

    const wrapper = mount(DevUiPage, {
      global: {
        plugins: [pinia, i18n],
        stubs: {
          Teleport: true,
        },
      },
    })

    const expectedKeys = [
      'demo-theme',
      'demo-layout',
      'demo-actions',
      'demo-forms',
      'demo-navigation',
      'demo-data',
      'demo-feedback',
      'demo-overlays',
    ]

    for (const key of expectedKeys) {
      const el = wrapper.find(`[data-testid="${key}"]`)
      expect(el.exists(), `Expected [data-testid="${key}"] to exist`).toBe(true)
    }

    // Verify ordering
    const elements = expectedKeys.map((key) => wrapper.find(`[data-testid="${key}"]`).element)
    for (let i = 0; i < elements.length - 1; i++) {
      const compare = elements[i].compareDocumentPosition(elements[i + 1])
      expect(compare & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
    }
  })

  it('contains no mixed-language strings in zh-CN/devUi.json', () => {
    const jsonPath = resolve(process.cwd(), 'src/locales/zh-CN/devUi.json')
    const raw = readFileSync(jsonPath, 'utf8')
    const json = JSON.parse(raw)

    function flattenValues(obj: unknown): string[] {
      if (typeof obj === 'string') return [obj]
      if (Array.isArray(obj)) return obj.flatMap(flattenValues)
      if (obj && typeof obj === 'object') {
        return Object.values(obj).flatMap(flattenValues)
      }
      return []
    }
    const values = flattenValues(json)
    // Whitelist allowed brand/acronym words, sorted longest first so NeoForge matches before Forge
    const whitelist = [
      'NeoForge',
      'CurseForge',
      'Minecraft',
      'Modrinth',
      'Fabric',
      'Forge',
      'Markdown',
      'OKLCH',
      'MCLR',
      'Java',
      'JSON',
      'HTML',
      'URL',
      'CSS',
      'PCL',
      'PID',
      'UI',
      'ID',
    ]

    // Mixed language regex: Chinese + at least 2 English letters (either order)
    const mixedRegex = /[\u4e00-\u9fff][\s\S]*[A-Za-z]{2,}|[A-Za-z]{2,}[\s\S]*[\u4e00-\u9fff]/

    const violations: { original: string; stripped: string }[] = []

    for (const val of values) {
      // Strip placeholders e.g. {count}
      let stripped = val.replace(/\{[^}]+\}/g, '')
      // Strip whitelisted brand words (case-insensitive or exact)
      for (const word of whitelist) {
        stripped = stripped.split(new RegExp(word, 'gi')).join('')
      }

      if (mixedRegex.test(stripped)) {
        violations.push({ original: val, stripped })
      }
    }

    expect(violations).toEqual([])
  })

  it('asserts presence and click interactions of MOverlay, MDrawer, and confirmDanger demo buttons', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)

    const i18n = createI18n({
      legacy: false,
      locale: 'zh-CN',
      messages: {
        'zh-CN': {
          devUi: zhCnDevUi,
          common: zhCnCommon,
        },
      },
    })

    const wrapper = mount(DevUiPage, {
      global: {
        plugins: [pinia, i18n],
        stubs: {
          teleport: true,
        },
      },
    })

    // 1. MOverlay custom button & interaction
    const overlayBtn = wrapper.find('[data-testid="open-custom-overlay-btn"]')
    expect(overlayBtn.exists()).toBe(true)
    await overlayBtn.trigger('click')

    // 2. MDrawer buttons & interaction
    const drawerRightBtn = wrapper.find('[data-testid="open-drawer-right-btn"]')
    expect(drawerRightBtn.exists()).toBe(true)
    await drawerRightBtn.trigger('click')

    const drawerLeftBtn = wrapper.find('[data-testid="open-drawer-left-btn"]')
    expect(drawerLeftBtn.exists()).toBe(true)
    await drawerLeftBtn.trigger('click')

    const drawerBottomBtn = wrapper.find('[data-testid="open-drawer-bottom-btn"]')
    expect(drawerBottomBtn.exists()).toBe(true)
    await drawerBottomBtn.trigger('click')

    // 3. confirmDanger button & interaction
    const confirmDangerBtn = wrapper.find('[data-testid="trigger-confirm-danger-btn"]')
    expect(confirmDangerBtn.exists()).toBe(true)
    await confirmDangerBtn.trigger('click')

    // 4. MSearchBar, MStatusBadge exist
    expect(wrapper.findComponent({ name: 'MSearchBar' }).exists()).toBe(true)
    expect(wrapper.findComponent({ name: 'MStatusBadge' }).exists()).toBe(true)
  })
})

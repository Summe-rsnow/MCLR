import { describe, it, expect, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { mount, flushPromises } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createRouter, createWebHistory, type Router } from 'vue-router'
import { useCrashStore } from '../store'
import CrashDialog from '../CrashDialog.vue'
import { resetMockCrash, lastExportedCrashReportDest } from '../mock'
import { setupMockIpc } from '../../../ipc/mock'

import zhCnCrash from '../../../locales/zh-CN/crash.json'
import zhCnCommon from '../../../locales/zh-CN/common.json'
import zhCnError from '../../../locales/zh-CN/error.json'

const i18n = createI18n({
  legacy: false,
  locale: 'zh-CN',
  messages: {
    'zh-CN': {
      crash: zhCnCrash,
      common: zhCnCommon,
      error: zhCnError,
    },
  },
})

describe('Crash Analysis Feature', () => {
  let router: Router

  beforeEach(async () => {
    setActivePinia(createPinia())
    setupMockIpc()
    resetMockCrash()

    router = createRouter({
      history: createWebHistory(),
      routes: [
        {
          path: '/logs',
          name: 'logs',
          component: { template: '<div>Logs Page</div>' },
        },
        {
          path: '/crash',
          name: 'crash',
          component: { template: '<div>Crash Page</div>' },
        },
      ],
    })
    await router.push('/')
    await router.isReady()
  })

  it('contains all 45 crash cause titles and advice in crash.json', () => {
    const requiredCauses = [
      'extractedModFile',
      'missingMixinBootstrap',
      'outOfMemory',
      'usingJdk',
      'unsupportedOpenGl',
      'usingOpenJ9',
      'javaTooNew',
      'javaIncompatible',
      'invalidModFileName',
      'pixelFormatNotSupported',
      'veryShortOutput',
      'intelDriverAccessViolation',
      'amdDriverAccessViolation',
      'nvidiaDriverAccessViolation',
      'manualDebugCrash',
      'openGl1282',
      'fileOrContentValidationFailed',
      'confirmedModCrash',
      'suspectedModCrash',
      'modConfigCrash',
      'modMixinFailed',
      'modLoaderError',
      'modInitializationFailed',
      'stackKeywordFound',
      'stackModNameFound',
      'optiFineWorldLoadCrash',
      'specificBlockCrash',
      'specificEntityCrash',
      'resourcePackTooLarge',
      'noAnalyzableFile',
      'x86JavaMemoryLimit',
      'duplicateMods',
      'incompatibleMods',
      'optiFineForgeIncompatible',
      'fabricError',
      'fabricSolutionProvided',
      'forgeError',
      'oldForgeNewJavaIncompatible',
      'multipleForgeInInstanceJson',
      'tooManyModsIdLimit',
      'nightConfigBug',
      'shadersModWithOptiFine',
      'incompleteForgeInstallation',
      'modRequiresJava11',
      'missingDependencyOrWrongMcVersion',
    ]

    expect(requiredCauses.length).toBe(45)
    for (const code of requiredCauses) {
      const causeObj = (zhCnCrash.cause as Record<string, { title?: string; advice?: string }>)[code]
      expect(causeObj, `Missing cause key: ${code}`).toBeDefined()
      expect(causeObj.title, `Missing title for ${code}`).toBeTruthy()
      expect(causeObj.advice, `Missing advice for ${code}`).toBeTruthy()
    }
  })

  it('useCrashStore analyzes instance and opens dialog', async () => {
    const store = useCrashStore()
    expect(store.isOpen).toBe(false)
    expect(store.currentResult).toBeNull()

    const result = await store.analyzeInstance('1.20.1-vanilla', null, true)
    expect(result).not.toBeNull()
    expect(store.isOpen).toBe(true)
    expect(store.currentResult?.causes.length).toBe(2)
    expect(store.currentResult?.causes[0].code).toBe('outOfMemory')
  })

  it('renders CrashDialog with cause details, evidence toggle and export action', async () => {
    const store = useCrashStore()
    await store.analyzeInstance('1.20.1-vanilla', null, true)

    mount(CrashDialog, {
      global: {
        plugins: [i18n, router],
      },
    })
    await flushPromises()

    // Dialog title in document.body
    expect(document.body.innerHTML).toContain('Minecraft 崩溃分析')
    expect(document.body.innerHTML).toContain('Minecraft 内存不足')
    expect(document.body.innerHTML).toContain('高置信度')
    expect(document.body.innerHTML).toContain('疑似模组引发游戏崩溃')

    // Test evidence toggle
    const evidenceToggle = Array.from(document.body.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('行日志证据'),
    )
    expect(evidenceToggle).toBeDefined()
    if (evidenceToggle) {
      evidenceToggle.click()
      await flushPromises()
      expect(document.body.innerHTML).toContain('OutOfMemoryError')
    }

    // Test export report action
    const exportBtn = Array.from(document.body.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('导出崩溃报告'),
    )
    expect(exportBtn).toBeDefined()
    if (exportBtn) {
      exportBtn.click()
      await flushPromises()
      expect(lastExportedCrashReportDest).not.toBeNull()
    }
    const closeBtn = Array.from(document.body.querySelectorAll('button')).find(
      (b) => b.textContent?.trim() === '关闭',
    )
    expect(closeBtn).toBeDefined()
    if (closeBtn) {
      closeBtn.click()
      expect(store.isOpen).toBe(false)
    }
  })
})

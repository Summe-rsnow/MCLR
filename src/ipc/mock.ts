import { mockIPC } from '@tauri-apps/api/mocks'
import { isEmptyScenarioActive, mockLatencyMs } from './scenario'

type MockHandlers = Record<string, (args: Record<string, unknown>) => unknown>

const featureMocks = import.meta.glob<{ default: MockHandlers }>('../features/*/mock.ts', {
  eager: true,
})

export function setupMockIpc(): void {
  const mockSettings = {
    schemaVersion: 1,
    ui: {
      seed: '#3b82f6',
      mode: 'system',
      material: 'flat',
      scale: 1.0,
      backgroundMode: 'none',
      backgroundPath: '',
      backgroundBlur: 0.0,
      backgroundOpacity: 0.3,
    },
    language: {
      locale: 'zh-CN',
      formatCulture: 'zh-CN',
    },
    system: {
      logLevel: 'info',
      logRetentionDays: 7,
      disableGpu: false,
      keepInBackground: false,
      closePromptDismissed: false,
    },
    launch: {
      ramMode: 'auto',
      ramCustomMb: 4096,
      jvmArgs:
        '-XX:+UseG1GC -XX:+UseAdaptiveSizePolicy -XX:-OmitStackTraceInFastThrow ' +
        '-Djdk.lang.Process.allowAmbiguousCommands=true -Dfml.ignoreInvalidMinecraftCertificates=True ' +
        '-Dfml.ignorePatchDiscrepancies=True -Dlog4j2.formatMsgNoLookups=true',
      gameArgs: '',
      windowTitle: '',
      fullscreen: false,
      width: 854,
      height: 480,
      preferredIpStack: 'auto',
      processPriority: 'normal',
      closeLauncherOnStart: 'keep',
      crashAnalysis: true,
      renderer: 'default',
      preLaunchCommand: '',
      useJavaExe: true,
      requireGpuHighPerformance: true,
      lockMemory: true,
      legacyFix: true,
      lwjglUnsafeAgent: true,
    },
    java: {
      custom: [],
      select: 'auto',
      preferJdk: false,
    },
    game: {
      folders: [],
      currentFolder: '',
      defaultIsolation: 'on',
      sortBy: 'name',
    },
    net: {
      proxyMode: 'none',
      proxyUrl: '',
      doh: false,
      sourcePriority: 'mirrorFirst',
      connectionsPerFile: 8,
      maxConcurrentFiles: 16,
    },
    update: {
      channel: 'stable',
      autoCheck: true,
      announcementUrl: '',
    },
    states: {
      dismissedAnnouncements: [],
    },
    comp: {
      favorites: [],
      storeDir: '',
      useHardlink: false,
      filenameFormat: 1,
      showZhName: true,
      localNameStyle: 0,
    },
  }
  const handlers: MockHandlers = {
    // Keep in sync with package.json on release; a stale value hides version bugs.
    app_info: () => ({ name: 'MCLR', version: '0.0.26' }),
    secrets_status: () => ({
      msClientId: false,
      curseforgeApiKey: false,
      updateEndpoint: false,
      updatePubkey: false,
    }),
    window_set_material: () => null,
    app_quit: () => null,
    renderer_agent_available: () => false,
    settings_get: () => ({ ...mockSettings }),
    settings_patch: (args) => {
      const patch = (args.patch ?? {}) as Record<string, unknown>
      if (patch && typeof patch === 'object') {
        for (const [key, val] of Object.entries(patch)) {
          if (val && typeof val === 'object' && !Array.isArray(val)) {
            ;(mockSettings as Record<string, unknown>)[key] = {
              ...((mockSettings as Record<string, unknown>)[key] as Record<string, unknown>),
              ...val,
            }
          } else {
            ;(mockSettings as Record<string, unknown>)[key] = val
          }
        }
      }
      return { ...mockSettings }
    },
    tasks_list: () => [],
    task_cancel: () => null,
    task_pause: () => null,
    task_resume: () => null,
  }

  for (const mod of Object.values(featureMocks)) {
    if (mod.default) {
      Object.assign(handlers, mod.default)
    }
  }

  mockIPC(
    async (cmd, args) => {
      const handler = handlers[cmd]
      if (!handler) {
        throw { code: 'unknown', params: { cmd }, detail: `Unhandled mock command: ${cmd}` }
      }
      const latency = mockLatencyMs()
      if (latency > 0) {
        const { promise, resolve } = Promise.withResolvers<void>()
        setTimeout(resolve, latency)
        await promise
      }
      const result = handler((args ?? {}) as Record<string, unknown>)
      return isEmptyScenarioActive() && /_(list|scan)$/.test(cmd) && Array.isArray(result)
        ? []
        : result
    },
    {
      // Backend-pushed events (`tasks://update`, `resources://patch`) can then be driven from a
      // test with `emit()` instead of being swallowed by the "unhandled command" fallback.
      shouldMockEvents: true,
    },
  )
}

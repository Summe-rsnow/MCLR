import { mockIPC } from '@tauri-apps/api/mocks'

type MockHandlers = Record<string, (args: Record<string, unknown>) => unknown>

const featureMocks = import.meta.glob<{ default: MockHandlers }>(
  '../features/*/mock.ts',
  { eager: true },
)

export function setupMockIpc(): void {
  const mockSettings = {
    schemaVersion: 1,
    ui: {
      seed: '#3b82f6',
      mode: 'system',
      material: 'flat',
      scale: 1.0,
      fontFamily: '',
      backgroundMode: 'none',
      backgroundPath: '',
      backgroundBlur: 0.0,
      backgroundOpacity: 0.3,
      musicEnabled: false,
      musicFolder: '',
      musicVolume: 50,
    },
    language: {
      locale: 'zh-CN',
      formatCulture: 'zh-CN',
    },
    system: {
      logLevel: 'info',
      logRetentionDays: 7,
      disableGpu: false,
    },
  }

  const handlers: MockHandlers = {
    app_info: () => ({ name: 'MCLR', version: '0.0.1' }),
    secrets_status: () => ({
      msClientId: false,
      curseforgeApiKey: false,
      updateEndpoint: false,
      updatePubkey: false,
    }),
    window_set_material: () => null,
    settings_get: () => ({ ...mockSettings }),
    settings_patch: (args) => {
      const patch = (args.patch ?? {}) as Record<string, unknown>
      if (patch && typeof patch === 'object') {
        for (const [key, val] of Object.entries(patch)) {
          if (val && typeof val === 'object' && !Array.isArray(val)) {
            (mockSettings as Record<string, unknown>)[key] = {
              ...((mockSettings as Record<string, unknown>)[key] as Record<string, unknown>),
              ...val,
            }
          } else {
            (mockSettings as Record<string, unknown>)[key] = val
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

  mockIPC((cmd, args) => {
    const handler = handlers[cmd]
    if (handler) {
      return handler((args ?? {}) as Record<string, unknown>)
    }
    throw { code: 'unknown', params: { cmd }, detail: `Unhandled mock command: ${cmd}` }
  })
}

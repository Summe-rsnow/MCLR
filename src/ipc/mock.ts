import { mockIPC } from '@tauri-apps/api/mocks'

type MockHandlers = Record<string, (args: Record<string, unknown>) => unknown>

const featureMocks = import.meta.glob<{ default: MockHandlers }>(
  '../features/*/mock.ts',
  { eager: true },
)

export function setupMockIpc(): void {
  const handlers: MockHandlers = {
    app_info: () => ({ name: 'MCLR', version: '0.0.1' }),
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

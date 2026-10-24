import type { LauncherLogFile, UpdateCheckResult } from '../../ipc/client'

export const mockUpdateResult: UpdateCheckResult = {
  available: true,
  currentVersion: '0.0.11',
  version: '0.0.12',
  date: '2026-10-25',
  body: '## 0.0.12 Release Notes\n- Support self-update\n- Remaining settings system pages\n- i18n testing improvements',
  downloadUrl: 'https://example.com/mclr-update-0.0.12.zip',
  signature: 'mock-sig-1234567890',
}

export const mockLauncherLogs: LauncherLogFile[] = [
  {
    fileName: 'mclr-2026-10-25.log',
    path: 'C:/Users/User/AppData/Roaming/MCLR/logs/mclr-2026-10-25.log',
    size: 2048,
    modifiedAt: '2026-10-25T10:00:00Z',
  },
  {
    fileName: 'mclr-2026-10-24.log',
    path: 'C:/Users/User/AppData/Roaming/MCLR/logs/mclr-2026-10-24.log',
    size: 10240,
    modifiedAt: '2026-10-24T18:30:00Z',
  },
]

export const mockLogContents: Record<string, string> = {
  'mclr-2026-10-25.log': '[2026-10-25T10:00:00Z INFO mclr_core] MCLR initialized version 0.0.11\n[2026-10-25T10:00:01Z INFO mclr_game] Found 1 installed instances\n',
  'mclr-2026-10-24.log': '[2026-10-24T18:30:00Z INFO mclr_core] System check ok\n',
}

let mockUpdateAvailable = true
let mockUpdateNotConfigured = false

export function setMockUpdateState(opts: { available?: boolean; notConfigured?: boolean }): void {
  if (opts.available !== undefined) mockUpdateAvailable = opts.available
  if (opts.notConfigured !== undefined) mockUpdateNotConfigured = opts.notConfigured
}

export function resetSettingsSystemMock(): void {
  mockUpdateAvailable = true
  mockUpdateNotConfigured = false
}

const handlers = {
  update_check: (_args: Record<string, unknown>): UpdateCheckResult => {
    if (mockUpdateNotConfigured) {
      throw {
        code: 'notConfigured',
        params: { what: 'updater' },
        detail: 'Update endpoint or pubkey not configured',
      }
    }
    if (!mockUpdateAvailable) {
      return {
        available: false,
        currentVersion: '0.0.11',
        version: null,
        date: null,
        body: null,
        downloadUrl: null,
        signature: null,
      }
    }
    return { ...mockUpdateResult }
  },
  update_install: (_args: Record<string, unknown>): string => {
    if (mockUpdateNotConfigured) {
      throw {
        code: 'notConfigured',
        params: { what: 'updater' },
        detail: 'Update endpoint or pubkey not configured',
      }
    }
    return 'task-update-install-001'
  },
  launcher_logs_list: (): LauncherLogFile[] => {
    return [...mockLauncherLogs]
  },
  launcher_log_read: (args: Record<string, unknown>): string => {
    const fileName = String(args.fileName ?? '')
    if (fileName in mockLogContents) {
      return mockLogContents[fileName]
    }
    return `[INFO] Mock log content for ${fileName}`
  },
}

export default handlers

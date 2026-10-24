import type { CrashResult } from '../../ipc/client'

export const mockCrashResult: CrashResult = {
  instanceId: '1.20.1-vanilla',
  analyzedAt: '2026-10-24T12:30:00.000Z',
  causes: [
    {
      code: 'outOfMemory',
      params: { maxMemory: '2048' },
      confidence: 'high',
      evidence: [
        'java.lang.OutOfMemoryError: Java heap space',
        'at net.minecraft.client.renderer.texture.TextureAtlas.load(TextureAtlas.java:120)',
        'Stack: Minecraft run into out of memory exception',
      ],
    },
    {
      code: 'suspectedModCrash',
      params: { modName: 'JEI' },
      confidence: 'medium',
      evidence: [
        'Caused by: mezz.jei.api.gui.builder.IIngredientAcceptorException: crash while loading recipe',
      ],
    },
  ],
  summaryLogLines: [
    '[12:29:55] [main/INFO]: Loading Minecraft 1.20.1',
    '[12:30:00] [Render thread/FATAL]: Reported exception thrown!',
    'java.lang.OutOfMemoryError: Java heap space',
  ],
  reportFiles: ['crash-reports/crash-2026-10-24_12.30.00-client.txt', 'logs/latest.log'],
}

let crashResultState: CrashResult = JSON.parse(JSON.stringify(mockCrashResult))
export let lastExportedCrashReportDest: string | null = null

export function setMockCrashResult(res: CrashResult): void {
  crashResultState = JSON.parse(JSON.stringify(res))
}

export function resetMockCrash(): void {
  crashResultState = JSON.parse(JSON.stringify(mockCrashResult))
  lastExportedCrashReportDest = null
}

const handlers = {
  crash_analyze: (_args: Record<string, unknown>): CrashResult => {
    return JSON.parse(JSON.stringify(crashResultState))
  },
  crash_export_report: (args: Record<string, unknown>): string => {
    lastExportedCrashReportDest = String(args.destZipPath ?? '')
    return 'crash-report-exported.zip'
  },
}

export default handlers

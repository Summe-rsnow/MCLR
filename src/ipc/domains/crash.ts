import { invokeCommand } from '../client'

export type CrashConfidence = 'high' | 'medium' | 'low'

export interface CrashCauseFinding {
  code: string
  params: Record<string, string>
  confidence: CrashConfidence
  evidence: string[]
}

export interface CrashResult {
  instanceId: string
  analyzedAt: string
  causes: CrashCauseFinding[]
  summaryLogLines: string[]
  reportFiles: string[]
}

export const crashIpc = {
  crashAnalyze: (instanceId: string, customLogPath?: string | null): Promise<CrashResult> =>
    invokeCommand<CrashResult>('crash_analyze', {
      instanceId,
      customLogPath: customLogPath ?? null,
    }),
  crashExportReport: (instanceId: string, destZipPath: string): Promise<string> =>
    invokeCommand<string>('crash_export_report', { instanceId, destZipPath }),
}

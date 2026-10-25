import { invokeCommand } from '../client'
import type { JavaInstallation } from '../bindings'

export interface ScreenshotMeta {
  fileName: string
  path: string
  size: number
  createdMs: number
  modifiedAt: string
}

export interface ScreenshotsPagedResult {
  items: ScreenshotMeta[]
  total: number
}

export const mediaIpc = {
  screenshotsListPaged: (
    instanceId: string,
    offset: number,
    limit: number,
  ): Promise<ScreenshotsPagedResult> =>
    invokeCommand<ScreenshotsPagedResult>('screenshots_list_paged', {
      instanceId,
      offset,
      limit,
    }),

  screenshotThumb: (path: string): Promise<string> =>
    invokeCommand<string>('screenshot_thumb', { path }),

  screenshotDelete: (path: string): Promise<void> =>
    invokeCommand<void>('screenshot_delete', { path }),

  screenshotReveal: (path: string): Promise<void> =>
    invokeCommand<void>('screenshot_reveal', { path }),

  screenshotOpen: (path: string): Promise<void> =>
    invokeCommand<void>('screenshot_open', { path }),

  screenshotCopy: (path: string): Promise<void> =>
    invokeCommand<void>('screenshot_copy', { path }),

  javaScan: (forceRefresh?: boolean): Promise<JavaInstallation[]> =>
    invokeCommand<JavaInstallation[]>('java_scan', { forceRefresh: forceRefresh ?? null }),

  javaList: (): Promise<JavaInstallation[]> =>
    invokeCommand<JavaInstallation[]>('java_list'),
}

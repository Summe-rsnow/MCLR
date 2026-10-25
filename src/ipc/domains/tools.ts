import { invokeCommand } from '../client'

export interface SkinFetchResult {
  username: string
  uuid: string
  skinUrl: string
  skinDataUrl: string
  headDataUrl: string
  slim: boolean
}

export interface CacheSizeInfo {
  cacheBytes: number
  logsBytes: number
  skinCacheBytes: number
  totalBytes: number
}

export interface DailyLuckResult {
  date: string
  username: string
  score: number
  levelKey: string
}

export const toolsIpc = {
  toolsFetchSkin: (username: string): Promise<SkinFetchResult> =>
    invokeCommand<SkinFetchResult>('tools_fetch_skin', { username }),
  toolsSaveSkin: (path: string, dataUrl: string): Promise<void> =>
    invokeCommand<void>('tools_save_skin', { path, dataUrl }),
  toolsCacheInfo: (): Promise<CacheSizeInfo> => invokeCommand<CacheSizeInfo>('tools_cache_info'),
  toolsCacheClear: (
    clearCache: boolean,
    clearLogs: boolean,
    clearSkins: boolean,
  ): Promise<CacheSizeInfo> =>
    invokeCommand<CacheSizeInfo>('tools_cache_clear', {
      clearCache,
      clearLogs,
      clearSkins,
    }),
  toolsCustomDownload: (url: string, destPath: string): Promise<string> =>
    invokeCommand<string>('tools_custom_download', { url, destPath }),
  toolsCreateShortcut: (instanceId: string, shortcutName?: string | null): Promise<string> =>
    invokeCommand<string>('tools_create_shortcut', {
      instanceId,
      shortcutName: shortcutName ?? null,
    }),
  toolsDailyLuck: (username: string): Promise<DailyLuckResult> =>
    invokeCommand<DailyLuckResult>('tools_daily_luck', { username }),
  toolsListMediaFiles: (folderPath: string, kind: string): Promise<string[]> =>
    invokeCommand<string[]>('tools_list_media_files', { folderPath, kind }),
  assetsAllow: (path: string): Promise<void> => invokeCommand<void>('assets_allow', { path }),
  pathOpen: (path: string): Promise<void> => invokeCommand<void>('path_open', { path }),
}

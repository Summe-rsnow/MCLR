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

export const toolsIpc = {
  toolsFetchSkin: (username: string): Promise<SkinFetchResult> =>
    invokeCommand<SkinFetchResult>('tools_fetch_skin', { username }),
  toolsSaveSkin: (path: string, dataUrl: string): Promise<void> =>
    invokeCommand<void>('tools_save_skin', { path, dataUrl }),
  toolsGenerateSkinAvatar: (skinPath: string, size: number): Promise<string> =>
    invokeCommand<string>('tools_generate_skin_avatar', { skinPath, size }),
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
  assetsAllow: (path: string): Promise<void> => invokeCommand<void>('assets_allow', { path }),
  pathOpen: (path: string): Promise<void> => invokeCommand<void>('path_open', { path }),
}

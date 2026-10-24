import type { CacheSizeInfo, DailyLuckResult, SkinFetchResult } from '../../ipc/client'

export const mockSkinResult: SkinFetchResult = {
  username: 'Notch',
  uuid: '069a79f444e94726a5bef2a2fce1b29d',
  skinUrl: 'http://textures.minecraft.net/texture/292009a4925b58f02c77ad806f9b99f40e3d526ea4a44b36e600d8317d221173',
  skinDataUrl: 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAEAAAABACAYAAACqaXHeAAAF',
  headDataUrl: 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAgAAAAICAIAAABLbSncAAAA',
  slim: false,
}

export const mockCacheInfo: CacheSizeInfo = {
  cacheBytes: 154200000,
  logsBytes: 12500000,
  skinCacheBytes: 3200000,
  totalBytes: 166700000,
}

let currentCacheState: CacheSizeInfo = { ...mockCacheInfo }

export const mockDailyLuckResult: DailyLuckResult = {
  date: '2026-10-24',
  username: 'Steve',
  score: 88,
  levelKey: 'blessing',
}

export let lastCreatedShortcut: { instanceId: string; shortcutName?: string | null } | null = null
export let lastCustomDownload: { url: string; destPath: string } | null = null

export function resetToolsMock(): void {
  currentCacheState = { ...mockCacheInfo }
  lastCreatedShortcut = null
  lastCustomDownload = null
}

const handlers = {
  tools_fetch_skin: (args: Record<string, unknown>): SkinFetchResult => {
    const username = String(args.username ?? 'Steve')
    return {
      ...mockSkinResult,
      username,
    }
  },
  tools_cache_info: (): CacheSizeInfo => {
    return { ...currentCacheState }
  },
  tools_cache_clear: (args: Record<string, unknown>): CacheSizeInfo => {
    const clearCache = Boolean(args.clearCache)
    const clearLogs = Boolean(args.clearLogs)
    const clearSkins = Boolean(args.clearSkins)

    if (clearCache) {
      currentCacheState.cacheBytes = 0
      currentCacheState.skinCacheBytes = 0
    } else if (clearSkins) {
      currentCacheState.skinCacheBytes = 0
    }

    if (clearLogs) {
      currentCacheState.logsBytes = 0
    }

    currentCacheState.totalBytes = currentCacheState.cacheBytes + currentCacheState.logsBytes
    return { ...currentCacheState }
  },
  tools_custom_download: (args: Record<string, unknown>): string => {
    lastCustomDownload = {
      url: String(args.url ?? ''),
      destPath: String(args.destPath ?? ''),
    }
    return 'task-custom-download-123'
  },
  tools_create_shortcut: (args: Record<string, unknown>): string => {
    lastCreatedShortcut = {
      instanceId: String(args.instanceId ?? ''),
      shortcutName: args.shortcutName ? String(args.shortcutName) : null,
    }
    return 'C:/Users/User/Desktop/Minecraft.lnk'
  },
  tools_daily_luck: (args: Record<string, unknown>): DailyLuckResult => {
    const username = String(args.username ?? 'Steve')
    return {
      ...mockDailyLuckResult,
      username,
    }
  },
  tools_list_media_files: (_args: Record<string, unknown>): string[] => {
    return ['/path/to/media1.mp3', '/path/to/media2.mp3']
  },
}

export default handlers

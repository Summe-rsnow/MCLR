import type {
  SaveInfo,
  DatapackInfo,
  ScreenshotInfo,
  InstanceFileItem,
  ServerEntry,
  ServerPingResult,
  ExportCandidateEntry,
  ModpackExportOptions,
} from '../../ipc/client'

export const mockSaves: Record<string, SaveInfo[]> = {
  '1.20.1-vanilla': [
    {
      folderName: 'NewWorld',
      path: 'C:\\Users\\User\\AppData\\Roaming\\MCLR\\.minecraft\\versions\\1.20.1-vanilla\\saves\\NewWorld',
      name: 'New World',
      seed: '123456789012345',
      spawn: { x: 100, y: 64, z: -200 },
      gameMode: 0,
      difficulty: 2,
      playTime: 3600000,
      lastPlayed: 1718000000000,
      dataVersion: 3465,
      iconDataUrl: null,
    },
    {
      folderName: 'SurvivalTest',
      path: 'C:\\Users\\User\\AppData\\Roaming\\MCLR\\.minecraft\\versions\\1.20.1-vanilla\\saves\\SurvivalTest',
      name: 'Survival Test',
      seed: '-987654321',
      spawn: { x: 0, y: 70, z: 0 },
      gameMode: 1,
      difficulty: 1,
      playTime: 7200000,
      lastPlayed: 1717000000000,
      dataVersion: 3465,
      iconDataUrl: null,
    },
  ],
}

export const mockDatapacks: Record<string, DatapackInfo[]> = {
  NewWorld: [
    {
      id: 'vanilla',
      fileName: 'vanilla',
      description: 'Vanilla data pack',
      packFormat: 15,
      enabled: true,
      builtIn: true,
    },
    {
      id: 'custom_pack.zip',
      fileName: 'custom_pack.zip',
      description: 'A custom adventure datapack',
      packFormat: 15,
      enabled: true,
      builtIn: false,
    },
    {
      id: 'structures',
      fileName: 'structures',
      description: 'Extra structures and dungeons',
      packFormat: 15,
      enabled: false,
      builtIn: false,
    },
  ],
}

export const mockScreenshots: Record<string, ScreenshotInfo[]> = {
  '1.20.1-vanilla': [
    {
      fileName: '2024-05-01_12.00.00.png',
      path: 'C:\\Users\\User\\AppData\\Roaming\\MCLR\\.minecraft\\versions\\1.20.1-vanilla\\screenshots\\2024-05-01_12.00.00.png',
      size: 1048576,
      modifiedAt: '2024-05-01T12:00:00Z',
      dataUrl: 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==',
    },
    {
      fileName: '2024-05-02_15.30.22.png',
      path: 'C:\\Users\\User\\AppData\\Roaming\\MCLR\\.minecraft\\versions\\1.20.1-vanilla\\screenshots\\2024-05-02_15.30.22.png',
      size: 2097152,
      modifiedAt: '2024-05-02T15:30:22Z',
      dataUrl: null,
    },
  ],
}

export const mockInstanceFiles: Record<string, Record<string, InstanceFileItem[]>> = {
  '1.20.1-vanilla': {
    resourcepacks: [
      {
        fileName: 'Faithful_32x.zip',
        path: 'C:\\path\\Faithful_32x.zip',
        enabled: true,
        size: 15000000,
        modifiedAt: '2024-04-10T10:00:00Z',
      },
      {
        fileName: 'BareBones.zip.disabled',
        path: 'C:\\path\\BareBones.zip.disabled',
        enabled: false,
        size: 8000000,
        modifiedAt: '2024-04-11T10:00:00Z',
      },
    ],
    shaderpacks: [
      {
        fileName: 'ComplementaryReimagined.zip',
        path: 'C:\\path\\ComplementaryReimagined.zip',
        enabled: true,
        size: 3500000,
        modifiedAt: '2024-04-12T10:00:00Z',
      },
    ],
    schematics: [
      {
        fileName: 'medieval_castle.schem',
        path: 'C:\\path\\medieval_castle.schem',
        enabled: true,
        size: 45000,
        modifiedAt: '2024-04-13T10:00:00Z',
      },
    ],
  },
}

export const mockServers: Record<string, ServerEntry[]> = {
  '1.20.1-vanilla': [
    {
      index: 0,
      name: 'Hypixel Network',
      ip: 'mc.hypixel.net',
      icon: null,
      acceptTextures: 1,
    },
    {
      index: 1,
      name: 'Local Test Server',
      ip: '127.0.0.1:25565',
      icon: null,
      acceptTextures: null,
    },
  ],
}

let savesState = JSON.parse(JSON.stringify(mockSaves)) as Record<string, SaveInfo[]>
let datapacksState = JSON.parse(JSON.stringify(mockDatapacks)) as Record<string, DatapackInfo[]>
let screenshotsState = JSON.parse(JSON.stringify(mockScreenshots)) as Record<string, ScreenshotInfo[]>
let instanceFilesState = JSON.parse(JSON.stringify(mockInstanceFiles)) as Record<
  string,
  Record<string, InstanceFileItem[]>
>
let serversState = JSON.parse(JSON.stringify(mockServers)) as Record<string, ServerEntry[]>

export const mockExportCandidates: ExportCandidateEntry[] = [
  { relPath: 'mods', isDir: true, size: 10485760, defaultSelected: true },
  { relPath: 'config', isDir: true, size: 20480, defaultSelected: true },
  { relPath: 'resourcepacks', isDir: true, size: 5242880, defaultSelected: true },
  { relPath: 'shaderpacks', isDir: true, size: 8388608, defaultSelected: true },
  { relPath: 'options.txt', isDir: false, size: 1536, defaultSelected: true },
  { relPath: 'saves', isDir: true, size: 31457280, defaultSelected: false },
  { relPath: 'logs', isDir: true, size: 512000, defaultSelected: false },
  { relPath: 'crash-reports', isDir: true, size: 102400, defaultSelected: false },
]

export let lastExportedModpackOptions: ModpackExportOptions | null = null

export function resetMockInstanceDetail(): void {
  savesState = JSON.parse(JSON.stringify(mockSaves))
  datapacksState = JSON.parse(JSON.stringify(mockDatapacks))
  screenshotsState = JSON.parse(JSON.stringify(mockScreenshots))
  instanceFilesState = JSON.parse(JSON.stringify(mockInstanceFiles))
  serversState = JSON.parse(JSON.stringify(mockServers))
  lastExportedModpackOptions = null
}

const handlers = {
  saves_list: (args: Record<string, unknown>): SaveInfo[] => {
    const instId = String(args.instanceId)
    return savesState[instId] || []
  },
  save_delete: (args: Record<string, unknown>): void => {
    const instId = String(args.instanceId)
    const folder = String(args.folderName)
    if (savesState[instId]) {
      savesState[instId] = savesState[instId].filter((s) => s.folderName !== folder)
    }
  },
  save_backup: (args: Record<string, unknown>): string => {
    const folder = String(args.folderName)
    return `C:\\backups\\${folder}_backup.zip`
  },
  save_export: (_args: Record<string, unknown>): void => {},
  save_import: (args: Record<string, unknown>): SaveInfo => {
    const instId = String(args.instanceId)
    const newSave: SaveInfo = {
      folderName: 'ImportedWorld',
      path: `C:\\minecraft\\versions\\${instId}\\saves\\ImportedWorld`,
      name: 'Imported World',
      seed: '999999999',
      spawn: { x: 50, y: 65, z: 50 },
      gameMode: 0,
      difficulty: 1,
      playTime: 0,
      lastPlayed: Date.now(),
      dataVersion: 3465,
      iconDataUrl: null,
    }
    if (!savesState[instId]) {
      savesState[instId] = []
    }
    savesState[instId].push(newSave)
    return newSave
  },
  save_datapacks_list: (args: Record<string, unknown>): DatapackInfo[] => {
    const folder = String(args.folderName)
    return datapacksState[folder] || []
  },
  save_datapack_toggle: (args: Record<string, unknown>): DatapackInfo[] => {
    const folder = String(args.folderName)
    const packId = String(args.packId)
    const enabled = Boolean(args.enabled)
    const list = datapacksState[folder] || []
    const target = list.find((p) => p.id === packId)
    if (target) {
      target.enabled = enabled
    }
    return [...list]
  },
  screenshots_list: (args: Record<string, unknown>): ScreenshotInfo[] => {
    const instId = String(args.instanceId)
    return screenshotsState[instId] || []
  },
  screenshot_delete: (args: Record<string, unknown>): void => {
    const instId = String(args.instanceId)
    const file = String(args.fileName)
    if (screenshotsState[instId]) {
      screenshotsState[instId] = screenshotsState[instId].filter((s) => s.fileName !== file)
    }
  },
  instance_files_list: (args: Record<string, unknown>): InstanceFileItem[] => {
    const instId = String(args.instanceId)
    const folderKind = String(args.folderKind)
    return instanceFilesState[instId]?.[folderKind] || []
  },
  instance_file_toggle: (args: Record<string, unknown>): InstanceFileItem => {
    const instId = String(args.instanceId)
    const folderKind = String(args.folderKind)
    const file = String(args.fileName)
    const enabled = Boolean(args.enabled)
    const list = instanceFilesState[instId]?.[folderKind] || []
    const item = list.find((i) => i.fileName === file)
    if (!item) {
      throw { code: 'saves.invalidPath', params: { file } }
    }
    item.enabled = enabled
    if (enabled && item.fileName.endsWith('.disabled')) {
      item.fileName = item.fileName.replace(/\.disabled$/, '')
    } else if (!enabled && !item.fileName.endsWith('.disabled')) {
      item.fileName = `${item.fileName}.disabled`
    }
    return { ...item }
  },
  instance_file_delete: (args: Record<string, unknown>): void => {
    const instId = String(args.instanceId)
    const folderKind = String(args.folderKind)
    const file = String(args.fileName)
    if (instanceFilesState[instId]?.[folderKind]) {
      instanceFilesState[instId][folderKind] = instanceFilesState[instId][folderKind].filter(
        (i) => i.fileName !== file,
      )
    }
  },
  servers_list: (args: Record<string, unknown>): ServerEntry[] => {
    const instId = String(args.instanceId)
    return serversState[instId] || []
  },
  server_add: (args: Record<string, unknown>): ServerEntry[] => {
    const instId = String(args.instanceId)
    const name = String(args.name)
    const ip = String(args.ip)
    const acceptTextures = args.acceptTextures !== undefined ? (args.acceptTextures as number | null) : null
    if (!serversState[instId]) {
      serversState[instId] = []
    }
    const newEntry: ServerEntry = {
      index: serversState[instId].length,
      name,
      ip,
      icon: null,
      acceptTextures,
    }
    serversState[instId].push(newEntry)
    return [...serversState[instId]]
  },
  server_update: (args: Record<string, unknown>): ServerEntry[] => {
    const instId = String(args.instanceId)
    const index = Number(args.index)
    const name = String(args.name)
    const ip = String(args.ip)
    const acceptTextures = args.acceptTextures !== undefined ? (args.acceptTextures as number | null) : null
    const list = serversState[instId] || []
    if (list[index]) {
      list[index].name = name
      list[index].ip = ip
      list[index].acceptTextures = acceptTextures
    }
    return [...list]
  },
  server_remove: (args: Record<string, unknown>): ServerEntry[] => {
    const instId = String(args.instanceId)
    const index = Number(args.index)
    if (serversState[instId]) {
      serversState[instId].splice(index, 1)
      serversState[instId].forEach((s, idx) => {
        s.index = idx
      })
    }
    return [...(serversState[instId] || [])]
  },
  server_ping: (args: Record<string, unknown>): ServerPingResult => {
    const ip = String(args.ip)
    return {
      ip,
      resolvedAddr: '192.168.1.100:25565',
      online: true,
      motd: 'A Minecraft Server Welcome!',
      versionName: '1.20.1',
      protocol: 763,
      playersOnline: 42,
      playersMax: 100,
      favicon: null,
      latencyMs: 35,
    }
  },
  export_scan_instance: (_args: Record<string, unknown>): ExportCandidateEntry[] => {
    return [...mockExportCandidates]
  },
  export_modpack: (args: Record<string, unknown>): string => {
    const options = (args.options ?? args) as ModpackExportOptions
    lastExportedModpackOptions = options
    return 'task-export-1'
  },
}

export default handlers

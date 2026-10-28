import { invokeCommand } from '../client'

export interface SaveSpawn {
  x: number
  y: number
  z: number
}

export interface SaveInfo {
  folderName: string
  path: string
  name: string
  seed: string
  spawn: SaveSpawn
  gameMode: number
  difficulty: number
  playTime: number
  lastPlayed: number
  dataVersion?: number | null
  iconPath?: string | null
}

export interface DatapackInfo {
  id: string
  fileName: string
  description: string
  packFormat: number
  enabled: boolean
  builtIn: boolean
}

export interface ScreenshotInfo {
  fileName: string
  path: string
  size: number
  modifiedAt: string
  dataUrl?: string | null
}

export interface InstanceFileItem {
  fileName: string
  path: string
  enabled: boolean
  size: number
  modifiedAt: string
}

export const savesIpc = {
  savesList: (instanceId: string): Promise<SaveInfo[]> =>
    invokeCommand<SaveInfo[]>('saves_list', { instanceId }),
  saveDelete: (instanceId: string, folderName: string): Promise<void> =>
    invokeCommand<void>('save_delete', { instanceId, folderName }),
  saveBackup: (instanceId: string, folderName: string): Promise<string> =>
    invokeCommand<string>('save_backup', { instanceId, folderName }),
  saveExport: (instanceId: string, folderName: string, destZipPath: string): Promise<void> =>
    invokeCommand<void>('save_export', { instanceId, folderName, destZipPath }),
  saveImport: (instanceId: string, zipPath: string): Promise<SaveInfo> =>
    invokeCommand<SaveInfo>('save_import', { instanceId, zipPath }),
  saveDatapacksList: (instanceId: string, folderName: string): Promise<DatapackInfo[]> =>
    invokeCommand<DatapackInfo[]>('save_datapacks_list', { instanceId, folderName }),
  saveDatapackToggle: (
    instanceId: string,
    folderName: string,
    packId: string,
    enabled: boolean,
  ): Promise<DatapackInfo[]> =>
    invokeCommand<DatapackInfo[]>('save_datapack_toggle', {
      instanceId,
      folderName,
      packId,
      enabled,
    }),
}

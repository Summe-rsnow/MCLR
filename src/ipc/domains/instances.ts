import { invokeCommand } from '../client'

export type InstanceKind =
  | 'vanilla'
  | 'snapshot'
  | 'oldBeta'
  | 'oldAlpha'
  | 'fabric'
  | 'legacyFabric'
  | 'forge'
  | 'neoforge'
  | 'cleanroom'
  | 'optiFine'
  | 'liteLoader'
  | 'labyMod'
  | 'quilt'
  | 'moddedMultiple'

export type InstanceState = 'ok' | 'broken' | 'incomplete'

export type CardKind = 'star' | 'api' | 'regular' | 'rubbish' | 'fool' | 'error' | 'hidden'

export interface InstanceConfig {
  icon: string
  category: string
  pinned: boolean
  starred?: boolean
  cardType?: CardKind | null
  isolation: string
  java?: string | null
  memory?: number | null
  jvmArgs?: string | null
  gameArgs?: string | null
  windowTitle?: string | null
  preLaunchCommand?: string | null
  loginRequirement: string
  serverQuickPlay?: string | null
  lastPlayedAt?: string | null
}

export interface InstanceInfo {
  id: string
  name: string
  folder: string
  path: string
  mcVersion: string
  kind: InstanceKind
  loaders: string[]
  state: InstanceState
  config: InstanceConfig
  modable: boolean
  card: CardKind
  releaseTime?: string | null
  loaderVersions: Record<string, string>
}

export const instancesIpc = {
  instancesList: (): Promise<InstanceInfo[]> => invokeCommand<InstanceInfo[]>('instances_list'),
  instanceRename: (id: string, newName: string): Promise<InstanceInfo> =>
    invokeCommand<InstanceInfo>('instance_rename', { id, newName }),
  instanceDuplicate: (id: string, newName: string): Promise<InstanceInfo> =>
    invokeCommand<InstanceInfo>('instance_duplicate', { id, newName }),
  instanceDelete: (id: string, permanent = false): Promise<void> =>
    invokeCommand<void>('instance_delete', { id, permanent }),
  instanceUpdateConfig: (id: string, config: InstanceConfig): Promise<InstanceInfo> =>
    invokeCommand<InstanceInfo>('instance_update_config', { id, config }),
  instanceOpenPath: (id: string, sub = ''): Promise<void> =>
    invokeCommand<void>('instance_open_path', { id, sub }),
}

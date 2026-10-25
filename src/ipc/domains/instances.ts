import { invokeCommand } from '../client'
import type { InstanceConfig, InstanceInfo, CardKind } from '../client'

export type { CardKind }

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

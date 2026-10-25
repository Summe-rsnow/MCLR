import { invokeCommand } from '../client'

export interface ServerEntry {
  index: number
  name: string
  ip: string
  icon?: string | null
  acceptTextures?: number | null
}

export interface ServerPingResult {
  ip: string
  resolvedAddr: string
  online: boolean
  motd: string
  versionName: string
  protocol: number
  playersOnline: number
  playersMax: number
  favicon?: string | null
  latencyMs: number
}

export const serversIpc = {
  serversList: (instanceId: string): Promise<ServerEntry[]> =>
    invokeCommand<ServerEntry[]>('servers_list', { instanceId }),
  serverAdd: (
    instanceId: string,
    name: string,
    ip: string,
    acceptTextures?: number | null,
  ): Promise<ServerEntry[]> =>
    invokeCommand<ServerEntry[]>('server_add', {
      instanceId,
      name,
      ip,
      acceptTextures: acceptTextures ?? null,
    }),
  serverUpdate: (
    instanceId: string,
    index: number,
    name: string,
    ip: string,
    acceptTextures?: number | null,
  ): Promise<ServerEntry[]> =>
    invokeCommand<ServerEntry[]>('server_update', {
      instanceId,
      index,
      name,
      ip,
      acceptTextures: acceptTextures ?? null,
    }),
  serverRemove: (instanceId: string, index: number): Promise<ServerEntry[]> =>
    invokeCommand<ServerEntry[]>('server_remove', { instanceId, index }),
  serverPing: (ip: string): Promise<ServerPingResult> =>
    invokeCommand<ServerPingResult>('server_ping', { ip }),
}

import { invokeCommand } from '../client'

export type AccountKind = 'offline' | 'microsoft' | 'authlib' | 'yggdrasilConnect'

export interface CapeView {
  id: string
  alias: string
  url: string
  active: boolean
}

export interface AccountView {
  id: string
  kind: AccountKind
  username: string
  uuid: string
  selected: boolean
  server?: string | null
  serverName?: string | null
  skinHeadDataUrl?: string | null
  capes: CapeView[]
  defaultSkinModel: string
}

export interface DeviceCodeEvent {
  userCode: string
  verificationUri: string
  expiresIn: number
}

export interface AuthlibProfileView {
  id: string
  name: string
}

export interface AuthlibLoginResponse {
  account?: AccountView | null
  pendingSessionId?: string | null
  profiles: AuthlibProfileView[]
}

export const accountsIpc = {
  accountsList: (): Promise<AccountView[]> => invokeCommand<AccountView[]>('accounts_list'),
  accountsSelect: (id: string): Promise<AccountView[]> =>
    invokeCommand<AccountView[]>('accounts_select', { id }),
  accountsRemove: (id: string): Promise<AccountView[]> =>
    invokeCommand<AccountView[]>('accounts_remove', { id }),
  accountsAddOffline: (
    username: string,
    uuidMode: string,
    customUuid?: string,
    skinModel?: 'classic' | 'slim',
  ): Promise<AccountView> =>
    invokeCommand<AccountView>('accounts_add_offline', {
      username,
      uuidMode,
      customUuid,
      skinModel: skinModel ?? null,
    }),
  accountsRenameOffline: (id: string, username: string): Promise<AccountView> =>
    invokeCommand<AccountView>('accounts_rename_offline', {
      id,
      username,
    }),
  accountsAddMicrosoft: (): Promise<string> => invokeCommand<string>('accounts_add_microsoft'),
  accountsAddAuthlib: (
    server: string,
    username: string,
    password: string,
  ): Promise<AuthlibLoginResponse> =>
    invokeCommand<AuthlibLoginResponse>('accounts_add_authlib', {
      server,
      username,
      password,
    }),
  accountsSelectProfile: (pendingSessionId: string, profileId: string): Promise<AccountView> =>
    invokeCommand<AccountView>('accounts_select_profile', {
      pendingSessionId,
      profileId,
    }),
  accountsRefresh: (id: string): Promise<AccountView> =>
    invokeCommand<AccountView>('accounts_refresh', { id }),
  accountsSkinUpload: (id: string, variant: string, pngPath: string): Promise<AccountView> =>
    invokeCommand<AccountView>('accounts_skin_upload', {
      id,
      variant,
      pngPath,
    }),
  accountsCapeSet: (id: string, capeId: string | null): Promise<AccountView> =>
    invokeCommand<AccountView>('accounts_cape_set', { id, capeId }),
}

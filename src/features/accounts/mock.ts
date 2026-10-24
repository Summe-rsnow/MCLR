import type { AccountView, CapeView, AuthlibLoginResponse } from '../../ipc/client'

export const mockCapes: CapeView[] = [
  {
    id: 'cape-migrator',
    alias: 'Migrator Cape',
    url: 'https://textures.minecraft.net/texture/2340c0e03dd66c3ea9d86d3cac630801da23c8a920e71426cf2779d663535920',
    active: true,
  },
  {
    id: 'cape-vanilla',
    alias: 'Vanilla Cape',
    url: 'https://textures.minecraft.net/texture/4176378e55fc112529994f2e9d394142f36f6d84877da4ff75cb579ee7bb05ec',
    active: false,
  },
]

export const mockAccounts: AccountView[] = [
  {
    id: 'acc-ms-1',
    kind: 'microsoft',
    username: 'StevePro',
    uuid: '069a79f4-44e9-4726-a5be-fca90e38aaf5',
    selected: true,
    server: null,
    serverName: null,
    skinHeadDataUrl:
      "data:image/svg+xml;utf8,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 8 8' shape-rendering='crispEdges'%3E%3Crect width='8' height='8' fill='%23b58463'/%3E%3Cpath d='M0 0h8v2H0zM0 2h1v1H0zM7 2h1v1H7z' fill='%233d2817'/%3E%3Crect x='1' y='4' width='1' height='1' fill='%23fff'/%3E%3Crect x='2' y='4' width='1' height='1' fill='%23493b7f'/%3E%3Crect x='5' y='4' width='1' height='1' fill='%23493b7f'/%3E%3Crect x='6' y='4' width='1' height='1' fill='%23fff'/%3E%3Crect x='3' y='5' width='2' height='1' fill='%239c6347'/%3E%3Cpath d='M2 6h4v1H2z' fill='%23633924'/%3E%3C/svg%3E",
    capes: mockCapes,
    defaultSkinModel: 'classic',
  },
  {
    id: 'acc-off-1',
    kind: 'offline',
    username: 'AlexBuilder',
    uuid: 'e7136261-262c-3580-bfae-4cb8c962b800',
    selected: false,
    server: null,
    serverName: null,
    skinHeadDataUrl: null,
    capes: [],
    defaultSkinModel: 'slim',
  },
  {
    id: 'acc-authlib-1',
    kind: 'authlib',
    username: 'SkinUser01',
    uuid: '8535a74e-5c4d-44a6-ba0a-70678f7e2a9b',
    selected: false,
    server: 'https://littleskin.cn/api/yggdrasil',
    serverName: 'LittleSkin',
    skinHeadDataUrl: null,
    capes: [],
    defaultSkinModel: 'classic',
  },
]

let accountsData: AccountView[] = JSON.parse(JSON.stringify(mockAccounts))

export function resetMockAccounts(): void {
  accountsData = JSON.parse(JSON.stringify(mockAccounts))
}

const handlers = {
  accounts_list: () => [...accountsData],

  accounts_select: (args: Record<string, unknown>) => {
    const id = String(args.id)
    accountsData = accountsData.map((a) => ({
      ...a,
      selected: a.id === id,
    }))
    return [...accountsData]
  },

  accounts_remove: (args: Record<string, unknown>) => {
    const id = String(args.id)
    accountsData = accountsData.filter((a) => a.id !== id)
    if (accountsData.length > 0 && !accountsData.some((a) => a.selected)) {
      accountsData[0].selected = true
    }
    return [...accountsData]
  },

  accounts_add_offline: (args: Record<string, unknown>) => {
    const username = String(args.username || 'Player')
    const uuidMode = String(args.uuidMode || 'standard')
    const customUuid = args.customUuid ? String(args.customUuid) : undefined
    const uuid =
      uuidMode === 'custom' && customUuid
        ? customUuid
        : '00000000-0000-0000-0000-' + Math.random().toString(16).substring(2, 14)
    const newAcc: AccountView = {
      id: `acc-off-${Date.now()}`,
      kind: 'offline',
      username,
      uuid,
      selected: accountsData.length === 0,
      server: null,
      serverName: null,
      skinHeadDataUrl: null,
      capes: [],
      defaultSkinModel: 'classic',
    }
    accountsData.push(newAcc)
    return { ...newAcc }
  },

  accounts_add_microsoft: () => {
    return 'task-auth-microsoft-1'
  },

  accounts_add_authlib: (args: Record<string, unknown>): AuthlibLoginResponse => {
    const server = String(args.server || 'https://littleskin.cn/api/yggdrasil')
    const username = String(args.username || '')
    void args.password

    if (username.includes('multiprofile')) {
      return {
        account: null,
        pendingSessionId: 'sess-123456',
        profiles: [
          { id: 'prof-1', name: 'ProfileAlpha' },
          { id: 'prof-2', name: 'ProfileBeta' },
        ],
      }
    }

    const newAcc: AccountView = {
      id: `acc-auth-${Date.now()}`,
      kind: 'authlib',
      username: username || 'ThirdPartyUser',
      uuid: '11112222-3333-4444-5555-666677778888',
      selected: accountsData.length === 0,
      server,
      serverName: server.replace(/^https?:\/\//, '').split('/')[0],
      skinHeadDataUrl: null,
      capes: [],
      defaultSkinModel: 'classic',
    }
    accountsData.push(newAcc)
    return {
      account: { ...newAcc },
      pendingSessionId: null,
      profiles: [],
    }
  },

  accounts_select_profile: (args: Record<string, unknown>) => {
    const profileId = String(args.profileId)
    const newAcc: AccountView = {
      id: `acc-auth-${Date.now()}`,
      kind: 'authlib',
      username: profileId === 'prof-2' ? 'ProfileBeta' : 'ProfileAlpha',
      uuid: '99998888-7777-6666-5555-444433332222',
      selected: accountsData.length === 0,
      server: 'https://littleskin.cn/api/yggdrasil',
      serverName: 'littleskin.cn',
      skinHeadDataUrl: null,
      capes: [],
      defaultSkinModel: 'classic',
    }
    accountsData.push(newAcc)
    return { ...newAcc }
  },

  accounts_refresh: (args: Record<string, unknown>) => {
    const id = String(args.id)
    const acc = accountsData.find((a) => a.id === id)
    if (!acc) throw { code: 'auth.notFound', params: { id } }
    return { ...acc }
  },

  accounts_skin_upload: (args: Record<string, unknown>) => {
    const id = String(args.id)
    const variant = String(args.variant || 'classic')
    const acc = accountsData.find((a) => a.id === id)
    if (!acc) throw { code: 'auth.notFound', params: { id } }
    acc.defaultSkinModel = variant
    acc.skinHeadDataUrl =
      'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAgAAAAICAYAAADED76LAAAAJElEQVQoU2P8z8BQDwAEgAF/QualIQAAAABJRU5ErkJggg=='
    return { ...acc }
  },

  accounts_cape_set: (args: Record<string, unknown>) => {
    const id = String(args.id)
    const capeId = args.capeId ? String(args.capeId) : null
    const acc = accountsData.find((a) => a.id === id)
    if (!acc) throw { code: 'auth.notFound', params: { id } }
    acc.capes = acc.capes.map((c) => ({
      ...c,
      active: c.id === capeId,
    }))
    return { ...acc }
  },
}

export default handlers

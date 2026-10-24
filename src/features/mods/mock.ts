import type {
  LocalModInfo,
  JarInJarNode,
  ResolvedDependencyItem,
  ModpackInspectResult,
  CompFile,
} from '../../ipc/client'
const initialModsList: LocalModInfo[] = [
  {
    fileName: 'fabric-api-0.92.0+1.20.1.jar',
    path: 'mods/fabric-api-0.92.0+1.20.1.jar',
    enabled: true,
    modId: 'fabric-api',
    name: 'Fabric API',
    version: '0.92.0+1.20.1',
    description: 'Core API library for Fabric mods.',
    authors: ['FabricMC'],
    loader: 'fabric',
    sha1: 'a1b2c3d4e5f60123456789abcdef0123456789ab',
    murmur2: 12345678,
    size: 2048576,
    projectId: 'P7dR8mSH',
    updateFile: null,
  },
  {
    fileName: 'sodium-fabric-0.5.8+mc1.20.1.jar',
    path: 'mods/sodium-fabric-0.5.8+mc1.20.1.jar',
    enabled: true,
    modId: 'sodium',
    name: 'Sodium',
    version: '0.5.8+mc1.20.1',
    description: 'Modern rendering engine for Minecraft.',
    authors: ['jellysquid3'],
    loader: 'fabric',
    sha1: 'c3d4e5f60123456789abcdef0123456789aba1b2',
    murmur2: 23456789,
    size: 1536000,
    projectId: 'AANobbMI',
    updateFile: null,
  },
  {
    fileName: 'iris-fabric-1.6.17+1.20.1.jar.disabled',
    path: 'mods/iris-fabric-1.6.17+1.20.1.jar.disabled',
    enabled: false,
    modId: 'iris',
    name: 'Iris Shaders',
    version: '1.6.17+1.20.1',
    description: 'A modern shaders mod for Minecraft.',
    authors: ['coderbot'],
    loader: 'fabric',
    sha1: 'd4e5f60123456789abcdef0123456789aba1b2c3',
    murmur2: 34567890,
    size: 3145728,
    projectId: 'YL57xq9U',
    updateFile: null,
  },
]

let mockModsData: Record<string, LocalModInfo[]> = {}

export function resetMockMods(): void {
  mockModsData = {
    default: JSON.parse(JSON.stringify(initialModsList)),
  }
}

resetMockMods()

const mockJarInJarData: Record<string, JarInJarNode[]> = {
  'fabric-api-0.92.0+1.20.1.jar': [
    {
      id: 'fabric-api-base',
      name: 'Fabric API Base',
      version: '0.4.31+ef105b4977',
      path: 'META-INF/jars/fabric-api-base-0.4.31+ef105b4977.jar',
      size: 32768,
      children: [],
    },
    {
      id: 'fabric-resource-loader-v0',
      name: 'Fabric Resource Loader (v0)',
      version: '0.11.10+ef105b4977',
      path: 'META-INF/jars/fabric-resource-loader-v0-0.11.10+ef105b4977.jar',
      size: 65536,
      children: [
        {
          id: 'fabric-resource-conditions-api-v1',
          name: 'Fabric Resource Conditions API (v1)',
          version: '2.3.8+ef105b4977',
          path: 'META-INF/jars/fabric-resource-conditions-api-v1-2.3.8+ef105b4977.jar',
          size: 16384,
          children: [],
        },
      ],
    },
  ],
}

export default {
  local_mods_list: (args: Record<string, unknown>): LocalModInfo[] => {
    const instId = (args.instanceId as string) || 'default'
    if (!mockModsData[instId]) {
      mockModsData[instId] = JSON.parse(JSON.stringify(mockModsData.default))
    }
    return JSON.parse(JSON.stringify(mockModsData[instId]))
  },

  local_mod_toggle: (args: Record<string, unknown>): LocalModInfo => {
    const instId = (args.instanceId as string) || 'default'
    const fileName = args.fileName as string
    const enabled = args.enabled as boolean
    const list = mockModsData[instId] || mockModsData.default
    const mod = list.find((m) => m.fileName === fileName)
    if (!mod) {
      throw { code: 'game.io', params: { file: fileName }, detail: 'Mod not found' }
    }
    mod.enabled = enabled
    if (enabled && mod.fileName.endsWith('.disabled')) {
      mod.fileName = mod.fileName.replace(/\.disabled$/, '')
      mod.path = mod.path.replace(/\.disabled$/, '')
    } else if (!enabled && !mod.fileName.endsWith('.disabled')) {
      mod.fileName = mod.fileName + '.disabled'
      mod.path = mod.path + '.disabled'
    }
    return mod
  },

  local_mod_delete: (args: Record<string, unknown>): null => {
    const instId = (args.instanceId as string) || 'default'
    const fileName = args.fileName as string
    const list = mockModsData[instId] || mockModsData.default
    const idx = list.findIndex((m) => m.fileName === fileName)
    if (idx !== -1) {
      list.splice(idx, 1)
    }
    return null
  },

  local_mods_check_updates: (args: Record<string, unknown>): LocalModInfo[] => {
    const instId = (args.instanceId as string) || 'default'
    const list = mockModsData[instId] || mockModsData.default
    const sodiumMod = list.find((m) => m.modId === 'sodium')
    if (sodiumMod) {
      sodiumMod.updateFile = {
        id: 'new-sodium-file-id',
        projectId: 'AANobbMI',
        platform: 'modrinth',
        name: 'sodium-fabric-0.5.11+mc1.20.1.jar',
        fileName: 'sodium-fabric-0.5.11+mc1.20.1.jar',
        mcVersions: ['1.20.1'],
        loaders: ['fabric'],
        channel: 'release',
        releaseDate: '2024-07-01T12:00:00Z',
        size: 1600000,
        hashes: { sha1: 'newhash123' },
        downloadUrls: ['https://cdn.modrinth.com/data/AANobbMI/versions/new.jar'],
        distributionDisabled: false,
        dependencies: [],
      }
    }
    return [...list]
  },

  local_mod_update: (args: Record<string, unknown>): string => {
    const instId = (args.instanceId as string) || 'default'
    const currentFileName = args.currentFileName as string
    const newFile = args.newFile as CompFile
    const list = mockModsData[instId] || mockModsData.default
    const mod = list.find((m) => m.fileName === currentFileName)
    if (mod && newFile) {
      mod.fileName = newFile.name
      mod.version = '0.5.11+mc1.20.1'
      mod.updateFile = null
    }
    return 'task-mod-update-' + Math.random().toString(36).substring(2, 8)
  },

  local_mod_jar_in_jar: (args: Record<string, unknown>): JarInJarNode[] => {
    const fileName = (args.fileName as string) || ''
    const cleanName = fileName.replace(/\.disabled$/, '')
    return mockJarInJarData[cleanName] || []
  },

  comp_resolve_dependencies: (args: Record<string, unknown>): ResolvedDependencyItem[] => {
    const file = args.file as CompFile
    if (!file) return []
    return [
      {
        project: {
          platform: 'modrinth',
          id: 'P7dR8mSH',
          slug: 'fabric-api',
          title: 'Fabric API',
          summary: 'Essential hooks and features for Fabric mods.',
          author: 'FabricMC',
          downloads: 100000000,
          follows: 200000,
          categories: ['library'],
          mcVersions: ['1.20.1'],
          loaders: ['fabric'],
          compType: 'mod',
          updatedAt: '2024-06-01T12:00:00Z',
        },
        file: {
          id: 'fapi-ver-1',
          projectId: 'P7dR8mSH',
          platform: 'modrinth',
          name: 'fabric-api-0.92.0+1.20.1.jar',
          fileName: 'fabric-api-0.92.0+1.20.1.jar',
          mcVersions: ['1.20.1'],
          loaders: ['fabric'],
          channel: 'release',
          releaseDate: '2024-05-01T12:00:00Z',
          size: 2048576,
          hashes: { sha1: 'a1b2c3' },
          downloadUrls: ['https://cdn.modrinth.com/data/P7dR8mSH/versions/fapi.jar'],
          distributionDisabled: false,
          dependencies: [],
        },
        installed: true,
      },
      {
        project: {
          platform: 'modrinth',
          id: 'mOgUt4GM',
          slug: 'modmenu',
          title: 'Mod Menu',
          summary: 'Adds a mod list to Minecraft.',
          author: 'Prospector',
          downloads: 30000000,
          follows: 80000,
          categories: ['utility'],
          mcVersions: ['1.20.1'],
          loaders: ['fabric'],
          compType: 'mod',
          updatedAt: '2024-05-15T12:00:00Z',
        },
        file: {
          id: 'modmenu-ver-1',
          projectId: 'mOgUt4GM',
          platform: 'modrinth',
          name: 'modmenu-7.2.2.jar',
          fileName: 'modmenu-7.2.2.jar',
          mcVersions: ['1.20.1'],
          loaders: ['fabric'],
          channel: 'release',
          releaseDate: '2024-05-10T12:00:00Z',
          size: 512000,
          hashes: { sha1: 'b2c3d4' },
          downloadUrls: ['https://cdn.modrinth.com/data/mOgUt4GM/versions/modmenu.jar'],
          distributionDisabled: false,
          dependencies: [],
        },
        installed: false,
      },
    ]
  },

  modpack_inspect: (args: Record<string, unknown>): ModpackInspectResult => {
    const archivePath = (args.archivePath as string) || ''
    if (archivePath.toLowerCase().includes('quilt')) {
      throw {
        code: 'modpack.quiltUnsupported',
        params: {},
        detail: 'Quilt modpacks are not supported',
      }
    }
    return {
      format: archivePath.endsWith('.mrpack') ? 'modrinth' : 'curseforge',
      name: 'Fabulously Optimized',
      version: '5.8.0',
      author: 'RobotKoer',
      mcVersion: '1.20.1',
      loader: 'Fabric',
      loaderVersion: '0.15.11',
      fileCount: 42,
    }
  },

  modpack_import_file: (_args: Record<string, unknown>): string => {
    return 'task-modpack-import-' + Math.random().toString(36).substring(2, 8)
  },

  modpack_install_remote: (_args: Record<string, unknown>): string => {
    return 'task-modpack-install-' + Math.random().toString(36).substring(2, 8)
  },
}

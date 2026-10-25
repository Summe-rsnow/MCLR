import type {
  VersionManifestList,
  LoaderKind,
  LoaderVersionEntry,
} from '../../ipc/client'
import { validateInstanceNameLocal } from './compat'

export const mockVersionManifest: VersionManifestList = {
  latestRelease: '1.20.4',
  latestSnapshot: '24w14a',
  versions: [
    {
      id: '24w14a',
      versionType: 'snapshot',
      url: 'https://example.com/24w14a.json',
      releaseTime: '2024-04-03T14:00:00+00:00',
    },
    {
      id: '1.20.4',
      versionType: 'release',
      url: 'https://example.com/1.20.4.json',
      releaseTime: '2023-12-07T13:00:00+00:00',
    },
    {
      id: '1.20.1',
      versionType: 'release',
      url: 'https://example.com/1.20.1.json',
      releaseTime: '2023-06-12T10:00:00+00:00',
    },
    {
      id: '1.19.4',
      versionType: 'release',
      url: 'https://example.com/1.19.4.json',
      releaseTime: '2023-03-14T11:00:00+00:00',
    },
    {
      id: '1.14.4',
      versionType: 'release',
      url: 'https://example.com/1.14.4.json',
      releaseTime: '2019-07-19T10:00:00+00:00',
    },
    {
      id: '1.12.2',
      versionType: 'release',
      url: 'https://example.com/1.12.2.json',
      releaseTime: '2017-09-18T08:00:00+00:00',
    },
    {
      id: '1.8.9',
      versionType: 'release',
      url: 'https://example.com/1.8.9.json',
      releaseTime: '2015-12-03T09:00:00+00:00',
    },
    {
      id: 'b1.7.3',
      versionType: 'old_beta',
      url: 'https://example.com/b1.7.3.json',
      releaseTime: '2011-07-08T00:00:00+00:00',
    },
  ],
}

let manifestData: VersionManifestList = JSON.parse(
  JSON.stringify(mockVersionManifest),
)

export function resetMockDownload(): void {
  manifestData = JSON.parse(JSON.stringify(mockVersionManifest))
}

export function getMockLoaderVersions(
  loader: LoaderKind,
  mcVersion: string,
): LoaderVersionEntry[] {
  switch (loader) {
    case 'fabric':
      return [
        {
          loader: 'fabric',
          mcVersion,
          version: '0.15.11',
          stable: true,
          releaseTime: '2024-04-01T00:00:00Z',
        },
        {
          loader: 'fabric',
          mcVersion,
          version: '0.15.10',
          stable: true,
          releaseTime: '2024-03-20T00:00:00Z',
        },
      ]
    case 'forge':
      return [
        {
          loader: 'forge',
          mcVersion,
          version: '47.2.0',
          stable: true,
          releaseTime: '2023-10-01T00:00:00Z',
        },
        {
          loader: 'forge',
          mcVersion,
          version: '47.1.0',
          stable: false,
          releaseTime: '2023-08-01T00:00:00Z',
        },
      ]
    case 'neoForge':
      return [
        {
          loader: 'neoForge',
          mcVersion,
          version: '20.4.80-beta',
          stable: true,
          releaseTime: '2024-01-01T00:00:00Z',
        },
      ]
    case 'optiFine':
      return [
        {
          loader: 'optiFine',
          mcVersion,
          version: 'HD_U_I7',
          stable: true,
          releaseTime: '2023-12-01T00:00:00Z',
        },
        {
          loader: 'optiFine',
          mcVersion,
          version: 'HD_U_I6',
          stable: true,
          releaseTime: '2023-11-01T00:00:00Z',
        },
      ]
    case 'legacyFabric':
      return [
        {
          loader: 'legacyFabric',
          mcVersion,
          version: '0.14.21',
          stable: true,
          releaseTime: '2023-09-01T00:00:00Z',
        },
      ]
    case 'liteLoader':
      return [
        {
          loader: 'liteLoader',
          mcVersion,
          version: '1.12.2-SNAPSHOT',
          stable: true,
          releaseTime: '2018-01-01T00:00:00Z',
        },
      ]
    case 'cleanroom':
      return [
        {
          loader: 'cleanroom',
          mcVersion,
          version: '0.2.4-alpha',
          stable: true,
          releaseTime: '2023-11-01T00:00:00Z',
        },
      ]
    default:
      return []
  }
}

const handlers = {
  install_manifest: () => {
    if (typeof window !== 'undefined' && new URLSearchParams(window.location.search).get('many') === '1') {
      const extra = Array.from({ length: 900 }, (_, i) => ({
        id: `23w${String((i % 50) + 1).padStart(2, '0')}a-${i}`,
        versionType: 'snapshot',
        url: `https://piston-meta.mojang.com/v1/packages/${i}/snap.json`,
        releaseTime: new Date(Date.UTC(2023, 0, 1 + (i % 300))).toISOString(),
      }))
      return { ...manifestData, versions: [...manifestData.versions, ...extra] }
    }
    return { ...manifestData }
  },
  install_create: (args: Record<string, unknown>) => {
    const req = args.req as { instanceName?: string } | undefined
    const name = String(req?.instanceName || 'instance')
    return `task-install-${name}`
  },
  install_extras: () => {
    return {
      fabricApi: [
        {
          id: 'mock-fabric-api-1',
          projectId: 'fabric-api',
          platform: 'modrinth',
          name: 'Fabric API 0.92.0+1.20.1',
          fileName: 'fabric-api-0.92.0+1.20.1.jar',
          mcVersions: ['1.20.1'],
          loaders: ['fabric'],
          channel: 'release',
          releaseDate: '2024-04-01T12:00:00Z',
          size: 2000000,
          hashes: { sha1: 'abc', sha512: null, murmur2: null },
          downloadUrls: ['https://example.com/fabric-api.jar'],
          distributionDisabled: false,
          dependencies: [],
        },
      ],
      legacyFabricApi: [],
      optiFabric: [
        {
          id: 'mock-optifabric-1',
          projectId: 'optifabric',
          platform: 'modrinth',
          name: 'OptiFabric 1.14.3',
          fileName: 'optifabric-1.14.3.jar',
          mcVersions: ['1.20.1'],
          loaders: ['fabric'],
          channel: 'release',
          releaseDate: '2024-04-01T12:00:00Z',
          size: 1000000,
          hashes: { sha1: 'def', sha512: null, murmur2: null },
          downloadUrls: ['https://example.com/optifabric.jar'],
          distributionDisabled: false,
          dependencies: [],
        },
      ],
    }
  },
  install_current_info: () => {
    return {
      loaders: ['Fabric 0.15.11'],
      extras: {
        fabricApi: 'fabric-api-0.92.0+1.20.1.jar',
        legacyFabricApi: null,
        optifabric: null,
      },
    }
  },
  install_validate_name: (args: Record<string, unknown>) => {
    const name = String(args.name || '')
    const err = validateInstanceNameLocal(name)
    if (err) {
      throw { code: err }
    }
    return null
  },
  install_vanilla: (args: Record<string, unknown>) => {
    const version = String(args.version || '')
    const name = String(args.name || '')
    return `task-install-vanilla-${name || version}`
  },
  install_repair: (args: Record<string, unknown>) => {
    const instanceId = String(args.instanceId || '')
    return `task-repair-${instanceId}`
  },
  loaders_versions: (args: Record<string, unknown>) => {
    const loader = args.loader as LoaderKind
    const mcVersion = String(args.mcVersion || '')
    return getMockLoaderVersions(loader, mcVersion)
  },
  loaders_install: (args: Record<string, unknown>) => {
    const name = String(args.instanceName || '')
    return `task-install-loader-${name}`
  },
}
export default handlers

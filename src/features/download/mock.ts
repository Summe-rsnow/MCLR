import type {
  VersionManifestList,
  LoaderKind,
  LoaderVersionEntry,
} from '../../ipc/client'

export const mockVersionManifest: VersionManifestList = {
  latestRelease: '1.20.4',
  latestSnapshot: '24w14a',
  versions: [
    {
      id: '1.20.4',
      versionType: 'release',
      url: 'https://piston-meta.mojang.com/v1/packages/1.20.4.json',
      releaseTime: '2023-12-07T12:00:00+00:00',
    },
    {
      id: '1.20.1',
      versionType: 'release',
      url: 'https://piston-meta.mojang.com/v1/packages/1.20.1.json',
      releaseTime: '2023-06-12T12:00:00+00:00',
    },
    {
      id: '1.16.5',
      versionType: 'release',
      url: 'https://piston-meta.mojang.com/v1/packages/1.16.5.json',
      releaseTime: '2021-01-15T12:00:00+00:00',
    },
    {
      id: '24w14a',
      versionType: 'snapshot',
      url: 'https://piston-meta.mojang.com/v1/packages/24w14a.json',
      releaseTime: '2024-04-03T12:00:00+00:00',
    },
    {
      id: '1.20.5-pre1',
      versionType: 'snapshot',
      url: 'https://piston-meta.mojang.com/v1/packages/1.20.5-pre1.json',
      releaseTime: '2024-04-10T12:00:00+00:00',
    },
    {
      id: 'b1.7.3',
      versionType: 'old_beta',
      url: 'https://piston-meta.mojang.com/v1/packages/b1.7.3.json',
      releaseTime: '2011-07-08T12:00:00+00:00',
    },
    {
      id: 'a1.2.6',
      versionType: 'old_alpha',
      url: 'https://piston-meta.mojang.com/v1/packages/a1.2.6.json',
      releaseTime: '2010-12-03T12:00:00+00:00',
    },
  ],
}

let manifestData: VersionManifestList = JSON.parse(JSON.stringify(mockVersionManifest))

export function resetMockDownload(): void {
  manifestData = JSON.parse(JSON.stringify(mockVersionManifest))
}

export function getMockLoaderVersions(loader: LoaderKind, mcVersion: string): LoaderVersionEntry[] {
  switch (loader) {
    case 'fabric':
      return [
        { loader: 'fabric', mcVersion, version: '0.15.11', stable: true, releaseTime: '2024-04-10T12:00:00Z' },
        { loader: 'fabric', mcVersion, version: '0.15.10', stable: true, releaseTime: '2024-04-01T12:00:00Z' },
        { loader: 'fabric', mcVersion, version: '0.16.0-beta.1', stable: false, releaseTime: '2024-05-01T12:00:00Z' },
      ]
    case 'forge':
      if (mcVersion === '1.20.1') {
        return [
          { loader: 'forge', mcVersion, version: '47.2.20', stable: true, releaseTime: '2023-11-20T10:00:00Z' },
          { loader: 'forge', mcVersion, version: '47.2.0', stable: true, releaseTime: '2023-09-10T10:00:00Z' },
        ]
      }
      if (mcVersion === '1.16.5') {
        return [
          { loader: 'forge', mcVersion, version: '36.2.39', stable: true, releaseTime: '2022-01-10T10:00:00Z' },
        ]
      }
      return [
        { loader: 'forge', mcVersion, version: `${mcVersion}-recommended`, stable: true, releaseTime: '2023-01-01T00:00:00Z' },
      ]
    case 'neoForge':
      return [
        { loader: 'neoForge', mcVersion, version: '20.4.80-beta', stable: true, releaseTime: '2024-03-15T10:00:00Z' },
        { loader: 'neoForge', mcVersion, version: '20.4.70-beta', stable: false, releaseTime: '2024-03-01T10:00:00Z' },
      ]
    case 'optiFine':
      return [
        { loader: 'optiFine', mcVersion, version: `HD_U_I7`, stable: true, releaseTime: '2023-10-01T00:00:00Z' },
        { loader: 'optiFine', mcVersion, version: `HD_U_I6`, stable: true, releaseTime: '2023-08-01T00:00:00Z' },
      ]
    case 'legacyFabric':
      return [
        { loader: 'legacyFabric', mcVersion, version: '0.14.21+1.8.9', stable: true, releaseTime: '2023-05-01T00:00:00Z' },
      ]
    case 'liteLoader':
      return [
        { loader: 'liteLoader', mcVersion, version: '1.12.2-SNAPSHOT', stable: true, releaseTime: '2019-01-01T00:00:00Z' },
      ]
    case 'labyMod':
      return [
        { loader: 'labyMod', mcVersion, version: '4.1.18', stable: true, releaseTime: '2024-02-01T00:00:00Z' },
      ]
    case 'cleanroom':
      return [
        { loader: 'cleanroom', mcVersion, version: '0.2.4-alpha', stable: true, releaseTime: '2024-01-15T00:00:00Z' },
      ]
    default:
      return []
  }
}

const handlers = {
  install_manifest: () => {
    return { ...manifestData }
  },
  install_vanilla: (args: Record<string, unknown>) => {
    const version = String(args.version || '')
    const name = String(args.name || '')
    return `task-install-vanilla-${name || version}`
  },
  install_repair: (args: Record<string, unknown>) => {
    const instanceId = String(args.instanceId || '')
    return `task-install-repair-${instanceId}`
  },
  loaders_versions: (args: Record<string, unknown>) => {
    const loader = String(args.loader || '') as LoaderKind
    const mcVersion = String(args.mcVersion || '')
    return getMockLoaderVersions(loader, mcVersion)
  },
  loaders_install: (args: Record<string, unknown>) => {
    const name = String(args.instanceName || '')
    const loader = String(args.loader || '')
    const loaderVersion = String(args.loaderVersion || '')
    return `task-install-loader-${name || loader}-${loaderVersion}`
  },
}
export default handlers

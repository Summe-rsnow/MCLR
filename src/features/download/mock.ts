import type { VersionManifestList } from '../../ipc/client'

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
}

export default handlers

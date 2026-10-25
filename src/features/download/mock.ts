import type { VersionManifestList, LoaderKind, LoaderVersionEntry } from '../../ipc/client'
import { validateInstanceNameLocal, isAprilFools, loaderVisibility, getMcFoolName } from './compat'

function generateMockVersions() {
  const versions: VersionManifestList['versions'] = []

  // 1. April Fools: exactly 12
  const aprilFoolsData = [
    { id: '26w14a', releaseTime: '2026-04-01T12:00:00+00:00' },
    { id: '25w14craftmine', releaseTime: '2025-04-01T12:00:00+00:00' },
    { id: '24w14potato', releaseTime: '2024-04-01T12:00:00+00:00' },
    { id: '23w13a_or_b', releaseTime: '2023-04-01T12:00:00+00:00' },
    { id: '22w13oneblockatatime', releaseTime: '2022-04-01T12:00:00+00:00' },
    { id: '20w14∞', releaseTime: '2020-04-01T12:00:00+00:00' },
    { id: '3D Shareware v1.34', releaseTime: '2019-04-01T12:00:00+00:00' },
    { id: '1.rv-pre1', releaseTime: '2016-04-01T12:00:00+00:00' },
    { id: '15w14a', releaseTime: '2015-04-01T12:00:00+00:00' },
    { id: '2.0_purple', releaseTime: '2013-04-01T12:00:00+00:00' },
    { id: '2.0_red', releaseTime: '2013-04-01T12:00:00+00:00' },
    { id: '2.0_blue', releaseTime: '2013-04-01T12:00:00+00:00' },
  ]
  for (const af of aprilFoolsData) {
    versions.push({
      id: af.id,
      versionType: 'snapshot',
      type: 'special',
      url: `https://example.com/${af.id}.json`,
      releaseTime: af.releaseTime,
    })
  }

  // 2. Releases: exactly 112
  const realReleases = [
    '1.21.4',
    '1.21.3',
    '1.21.2',
    '1.21.1',
    '1.21',
    '1.20.6',
    '1.20.5',
    '1.20.4',
    '1.20.3',
    '1.20.2',
    '1.20.1',
    '1.20',
    '1.19.4',
    '1.19.3',
    '1.19.2',
    '1.19.1',
    '1.19',
    '1.18.2',
    '1.18.1',
    '1.18',
    '1.17.1',
    '1.17',
    '1.16.5',
    '1.16.4',
    '1.16.3',
    '1.16.2',
    '1.16.1',
    '1.16',
    '1.15.2',
    '1.15.1',
    '1.15',
    '1.14.4',
    '1.14.3',
    '1.14.2',
    '1.14.1',
    '1.14',
    '1.13.2',
    '1.13.1',
    '1.13',
    '1.12.2',
    '1.12.1',
    '1.12',
    '1.11.2',
    '1.11.1',
    '1.11',
    '1.10.2',
    '1.10.1',
    '1.10',
    '1.9.4',
    '1.9.3',
    '1.9.2',
    '1.9.1',
    '1.9',
    '1.8.9',
    '1.8.8',
    '1.8.7',
    '1.8.6',
    '1.8.5',
    '1.8.4',
    '1.8.3',
    '1.8.2',
    '1.8.1',
    '1.8',
    '1.7.10',
    '1.7.9',
    '1.7.8',
    '1.7.7',
    '1.7.6',
    '1.7.5',
    '1.7.4',
    '1.7.2',
    '1.6.4',
    '1.6.2',
    '1.6.1',
    '1.5.2',
    '1.5.1',
    '1.4.7',
    '1.4.6',
    '1.4.5',
    '1.4.4',
    '1.4.2',
    '1.3.2',
    '1.3.1',
    '1.2.5',
    '1.2.4',
    '1.2.3',
    '1.2.2',
    '1.2.1',
    '1.1',
    '1.0',
  ]
  const releasesSet = new Set(realReleases)
  const additionalReleases: string[] = []
  let addIdx = 1
  while (realReleases.length + additionalReleases.length < 112) {
    const candidate = `1.${Math.floor(addIdx / 4) + 1}.${(addIdx % 4) + 5}`
    if (!releasesSet.has(candidate) && !additionalReleases.includes(candidate)) {
      additionalReleases.push(candidate)
    }
    addIdx++
  }
  const allReleases = [...realReleases, ...additionalReleases]
  allReleases.forEach((id, i) => {
    const d = new Date(Date.UTC(2024, 11, 1 - i * 35))
    if (d.getUTCMonth() === 3 && d.getUTCDate() === 1) d.setUTCDate(2)
    const releaseTime =
      id === '1.20.4'
        ? '2023-12-07T13:00:00+00:00'
        : id === '1.20.1'
          ? '2023-06-12T10:00:00+00:00'
          : id === '1.19.4'
            ? '2023-03-14T11:00:00+00:00'
            : id === '1.14.4'
              ? '2019-07-19T10:00:00+00:00'
              : id === '1.12.2'
                ? '2017-09-18T08:00:00+00:00'
                : id === '1.8.9'
                  ? '2015-12-03T09:00:00+00:00'
                  : d.toISOString()
    versions.push({
      id,
      versionType: 'release',
      type: 'release',
      url: `https://example.com/${id}.json`,
      releaseTime,
    })
  })

  // 3. Old: exactly 165
  const knownBetas = [
    'b1.8.1',
    'b1.8',
    'b1.7.3',
    'b1.7.2',
    'b1.7',
    'b1.6.6',
    'b1.6.5',
    'b1.6.4',
    'b1.6.3',
    'b1.6.2',
    'b1.6.1',
    'b1.6',
    'b1.5_02',
    'b1.5_01',
    'b1.5',
    'b1.4_01',
    'b1.4',
    'b1.3_01',
    'b1.3',
    'b1.2_02',
    'b1.2_01',
    'b1.2',
    'b1.1_02',
    'b1.1_01',
    'b1.1',
    'b1.0_01',
    'b1.0',
  ]
  const knownAlphas = [
    'a1.2.6',
    'a1.2.5',
    'a1.2.4_01',
    'a1.2.3_04',
    'a1.2.3_02',
    'a1.2.3_01',
    'a1.2.3',
    'a1.2.2b',
    'a1.2.2a',
    'a1.2.1_01',
    'a1.2.0_02',
    'a1.2.0_01',
    'a1.1.2_01',
    'a1.1.1',
    'a1.1.0',
    'a1.0.17_04',
    'a1.0.17_02',
    'a1.0.16',
    'a1.0.15',
    'a1.0.14',
    'a1.0.5_01',
    'a1.0.4',
  ]
  const knownClassics = [
    'c0.30',
    'c0.0.13a_03',
    'c0.0.13a',
    'rd-132211',
    'rd-20090515',
    'inf-20100618',
  ]
  const oldList = [...knownBetas, ...knownAlphas, ...knownClassics]
  let oldPad = 1
  while (oldList.length < 165) {
    oldList.push(`c0.0.${oldPad}`)
    oldPad++
  }
  oldList.forEach((id, i) => {
    const isBeta = id.startsWith('b')
    const d = new Date(Date.UTC(2011, 8, 1, 10, 0, 0))
    d.setUTCDate(d.getUTCDate() - i)
    if (d.getUTCMonth() === 3 && d.getUTCDate() === 1) d.setUTCDate(2)
    const releaseTime = id === 'b1.7.3' ? '2011-07-08T00:00:00+00:00' : d.toISOString()
    versions.push({
      id,
      versionType: isBeta ? 'old_beta' : 'old_alpha',
      type: isBeta ? 'old_beta' : 'old_alpha',
      url: `https://example.com/${id}.json`,
      releaseTime,
    })
  })

  // 4. Snapshots: exactly 825
  const snapshots: string[] = ['24w14a']
  let year = 24
  let week = 13
  let letterIdx = 0
  const letters = ['a', 'b', 'c', 'd']
  while (snapshots.length < 825) {
    const sId = `${String(year).padStart(2, '0')}w${String(week).padStart(2, '0')}${letters[letterIdx]}`
    if (
      !sId.includes('15w14a') &&
      !sId.includes('26w14a') &&
      !sId.startsWith('2.0') &&
      !sId.startsWith('20w14inf')
    ) {
      snapshots.push(sId)
    }
    letterIdx++
    if (letterIdx >= 3) {
      letterIdx = 0
      week--
      if (week < 1) {
        week = 50
        year--
        if (year < 11) {
          year = 24
        }
      }
    }
  }

  snapshots.forEach((id, i) => {
    const d = new Date(Date.UTC(2024, 3, 10, 15, 0, 0))
    d.setUTCDate(d.getUTCDate() - i * 2)
    if (d.getUTCMonth() === 3 && d.getUTCDate() === 1) d.setUTCDate(2)
    const releaseTime = id === '24w14a' ? '2024-04-03T14:00:00+00:00' : d.toISOString()
    versions.push({
      id,
      versionType: 'snapshot',
      type: 'snapshot',
      url: `https://example.com/${id}.json`,
      releaseTime,
    })
  })

  // Sort descending by releaseTime
  versions.sort((a, b) => new Date(b.releaseTime).getTime() - new Date(a.releaseTime).getTime())

  return versions
}

export const mockVersionManifest: VersionManifestList = {
  latestRelease: '1.20.4',
  latestSnapshot: '24w14a',
  versions: generateMockVersions(),
}

let manifestData: VersionManifestList = JSON.parse(JSON.stringify(mockVersionManifest))

export function resetMockDownload(): void {
  manifestData = JSON.parse(JSON.stringify(mockVersionManifest))
}

export function getMockLoaderVersions(loader: LoaderKind, mcVersion: string): LoaderVersionEntry[] {
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
    case 'labyMod':
      return [
        {
          loader: 'labyMod',
          mcVersion,
          version: '48',
          stable: true,
          releaseTime: '2024-04-01T00:00:00Z',
        },
      ]
    default:
      return []
  }
}

const handlers = {
  install_manifest: () => {
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
  install_version_compat: (args: Record<string, unknown>) => {
    const mcVersion = String(args.mcVersion || '')
    const releaseTime = String(args.releaseTime || '')
    const fool = isAprilFools(mcVersion, releaseTime)
    const vis = loaderVisibility(mcVersion)
    const supportedLoaders: string[] = []
    if (vis.fabric) supportedLoaders.push('fabric')
    if (vis.forge) supportedLoaders.push('forge')
    if (vis.neoforge) supportedLoaders.push('neoforge')
    if (vis.cleanroom) supportedLoaders.push('cleanroom')
    if (vis.legacyFabric) supportedLoaders.push('legacyfabric')
    if (vis.liteloader) supportedLoaders.push('liteloader')
    if (vis.labymod) supportedLoaders.push('labymod')
    return {
      isAprilFools: fool,
      foolCode: fool ? getMcFoolName(mcVersion) : null,
      supportedLoaders,
      incompatiblePairs: [],
      recommendedJava: 17,
    }
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

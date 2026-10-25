import { describe, expect, it } from 'vitest'
import {
  classifyVersion,
  defaultName,
  getMcFoolName,
  isAprilFools,
  loaderIncompatibilities,
  loaderVisibility,
  validateInstanceNameLocal,
} from '../compat'
import { mockVersionManifest } from '../mock'

describe('compat loaderVisibility', () => {
  it('Forge hidden for 1.5.1, shown for 1.5.2', () => {
    expect(loaderVisibility('1.5.1').forge).toBe(false)
    expect(loaderVisibility('1.5.2').forge).toBe(true)
  })

  it('NeoForge hidden for 1.19.4, shown for 1.20.1', () => {
    expect(loaderVisibility('1.19.4').neoforge).toBe(false)
    expect(loaderVisibility('1.20.1').neoforge).toBe(true)
  })

  it('Cleanroom only for 1.12.2', () => {
    expect(loaderVisibility('1.12.2').cleanroom).toBe(true)
    expect(loaderVisibility('1.12.1').cleanroom).toBe(false)
    expect(loaderVisibility('1.20.1').cleanroom).toBe(false)
  })

  it('Fabric shown for 1.14, LegacyFabric for 1.8.9', () => {
    expect(loaderVisibility('1.14').fabric).toBe(true)
    expect(loaderVisibility('1.8.9').fabric).toBe(false)
    expect(loaderVisibility('1.8.9').legacyFabric).toBe(true)
  })
})

describe('compat loaderIncompatibilities', () => {
  it('OptiFine disabled with Forge on 1.13.2 and 1.14.3, enabled on 1.14.4', () => {
    expect(
      loaderIncompatibilities('1.13.2', { primaryLoader: 'forge' }).optifineDisabledReason,
    ).toBe('download.compat.forgeOptifineVersion')
    expect(
      loaderIncompatibilities('1.14.3', { primaryLoader: 'forge' }).optifineDisabledReason,
    ).toBe('download.compat.forgeOptifineVersion')
    expect(
      loaderIncompatibilities('1.14.4', { primaryLoader: 'forge' }).optifineDisabledReason,
    ).toBeUndefined()
  })

  it('OptiFine disabled with Fabric on >= 1.20.5, enabled on 1.20.4', () => {
    expect(
      loaderIncompatibilities('1.20.5', { primaryLoader: 'fabric' }).optifineDisabledReason,
    ).toBe('download.compat.fabricOptifineVersion')
    expect(
      loaderIncompatibilities('1.20.4', { primaryLoader: 'fabric' }).optifineDisabledReason,
    ).toBeUndefined()
  })

  it('NeoForge selection disables OptiFine and LiteLoader', () => {
    const incomp = loaderIncompatibilities('1.20.1', { primaryLoader: 'neoforge' })
    expect(incomp.optifineDisabledReason).toBe('download.compat.neoforgeOptifine')
    expect(incomp.liteloaderDisabledReason).toBe('download.compat.neoforgeLiteloader')
  })
})

describe('compat defaultName', () => {
  it('produces the exact spec examples', () => {
    expect(
      defaultName({
        mcVersion: '1.20.1',
        fabricVersion: '0.15.11+build.1',
      }),
    ).toBe('1.20.1-Fabric_0.15.11')

    expect(
      defaultName({
        mcVersion: '1.20.1',
        forgeVersion: '47.2.0',
        optifineDisplay: '1.20.1 HD_U_I7',
      }),
    ).toBe('1.20.1-Forge_47.2.0-OptiFine_HD_U_I7')

    expect(
      defaultName({
        mcVersion: '1.12.2',
        forgeVersion: '14.23.5.2859',
        liteloader: true,
      }),
    ).toBe('1.12.2-Forge_14.23.5.2859-LiteLoader')

    expect(
      defaultName({
        mcVersion: '24w14a',
      }),
    ).toBe('24w14a')
  })
})

describe('compat validateInstanceNameLocal', () => {
  it('validates local rules and duplicate differing only by case', () => {
    expect(validateInstanceNameLocal('')).toBe('instance.name.empty')
    expect(validateInstanceNameLocal('a'.repeat(71))).toBe('instance.name.tooLong')
    expect(validateInstanceNameLocal(' leadingSpace')).toBe('instance.name.invalidChar')
    expect(validateInstanceNameLocal('trailingDot.')).toBe('instance.name.trailingDot')
    expect(validateInstanceNameLocal('CON')).toBe('instance.name.reserved')
    expect(validateInstanceNameLocal('aux.txt')).toBe('instance.name.reserved')

    expect(validateInstanceNameLocal('1.20.1-Fabric', ['1.20.1-fabric'])).toBe(
      'instance.name.duplicate',
    )
    expect(validateInstanceNameLocal('1.20.1-Fabric', ['1.20.1-forge'])).toBeNull()
  })
})

describe('compat isAprilFools', () => {
  it('detects 2.0 and known joke snapshot IDs', () => {
    expect(isAprilFools('2.0-purple')).toBe(true)
    expect(isAprilFools('2.0_blue')).toBe(true)
    expect(isAprilFools('2point0_red')).toBe(true)
    expect(isAprilFools('15w14a')).toBe(true)
    expect(isAprilFools('1.rv-pre1')).toBe(true)
    expect(isAprilFools('3d shareware v1.34')).toBe(true)
    expect(isAprilFools('3D Shareware v1.34')).toBe(true)
    expect(isAprilFools('20w14infinite')).toBe(true)
    expect(isAprilFools('20w14∞')).toBe(true)
    expect(isAprilFools('22w13oneblockatatime')).toBe(true)
    expect(isAprilFools('23w13a_or_b')).toBe(true)
    expect(isAprilFools('24w14potato')).toBe(true)
    expect(isAprilFools('25w14craftmine')).toBe(true)
    expect(isAprilFools('26w14a')).toBe(true)
    expect(isAprilFools('1.20.1')).toBe(false)
    expect(isAprilFools('24w14a')).toBe(false)
  })

  it('detects April 1st release dates', () => {
    expect(isAprilFools('custom-joke', '2021-04-01T12:00:00Z')).toBe(true)
    expect(isAprilFools('normal-snap', '2024-04-03T14:00:00Z')).toBe(false)
  })
})

describe('compat classifyVersion', () => {
  it('classifies release, snapshot, old, and april fools correctly', () => {
    expect(classifyVersion({ id: '1.20.4', versionType: 'release' })).toBe('release')
    expect(classifyVersion({ id: '1.20.1', versionType: 'release' })).toBe('release')
    expect(classifyVersion({ id: '24w14a', versionType: 'snapshot' })).toBe('snapshot')
    expect(classifyVersion({ id: '24w14potato', versionType: 'snapshot' })).toBe('aprilFools')
    expect(classifyVersion({ id: '20w14∞', versionType: 'snapshot' })).toBe('aprilFools')
    expect(classifyVersion({ id: 'b1.7.3', versionType: 'old_beta' })).toBe('old')
    expect(classifyVersion({ id: 'c0.30', versionType: 'old_alpha' })).toBe('old')
  })

  it('classifies 1.0 as release regardless of snapshot typing (PCL parity)', () => {
    expect(classifyVersion({ id: '1.0', versionType: 'snapshot' })).toBe('release')
    expect(classifyVersion({ id: '1.0', type: 'snapshot' })).toBe('release')
    expect(classifyVersion('1.0')).toBe('release')
  })

  it('preserves official release category even for April 1st release dates', () => {
    expect(
      classifyVersion({
        id: '26.1.1',
        versionType: 'release',
        releaseTime: '2026-04-01T09:06:36+00:00',
      }),
    ).toBe('release')
  })

  it('mock manifest produces exact PCL counts (112 release, 825 snapshot, 165 old, 12 april fools, 1114 total)', () => {
    const counts = { release: 0, snapshot: 0, old: 0, aprilFools: 0 }
    for (const v of mockVersionManifest.versions) {
      const cat = classifyVersion(v)
      counts[cat]++
    }
    expect(counts.release).toBe(112)
    expect(counts.snapshot).toBe(825)
    expect(counts.old).toBe(165)
    expect(counts.aprilFools).toBe(12)
    expect(mockVersionManifest.versions.length).toBe(1114)
    expect(mockVersionManifest.versions.some((v) => v.id === '1.0')).toBe(true)
  })
})

describe('compat getMcFoolName', () => {
  it('returns appropriate Chinese lore names for joke versions', () => {
    expect(getMcFoolName('24w14potato')).toBe('毒马铃薯更新')
    expect(getMcFoolName('20w14∞')).toBe('终极内容更新')
    expect(getMcFoolName('20w14infinite')).toBe('终极内容更新')
    expect(getMcFoolName('15w14a')).toBe('爱与拥抱更新')
    expect(getMcFoolName('2.0_blue')).toBe('2.0 蓝色版')
    expect(getMcFoolName('1.20.4')).toBe('')
  })
})

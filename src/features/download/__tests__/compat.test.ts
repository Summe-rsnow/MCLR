import { describe, expect, it } from 'vitest'
import {
  defaultName,
  isAprilFools,
  loaderIncompatibilities,
  loaderVisibility,
  validateInstanceNameLocal,
} from '../compat'

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
      loaderIncompatibilities('1.13.2', { primaryLoader: 'forge' })
        .optifineDisabledReason,
    ).toBe('download.compat.forgeOptifineVersion')
    expect(
      loaderIncompatibilities('1.14.3', { primaryLoader: 'forge' })
        .optifineDisabledReason,
    ).toBe('download.compat.forgeOptifineVersion')
    expect(
      loaderIncompatibilities('1.14.4', { primaryLoader: 'forge' })
        .optifineDisabledReason,
    ).toBeUndefined()
  })

  it('OptiFine disabled with Fabric on >= 1.20.5, enabled on 1.20.4', () => {
    expect(
      loaderIncompatibilities('1.20.5', { primaryLoader: 'fabric' })
        .optifineDisabledReason,
    ).toBe('download.compat.fabricOptifineVersion')
    expect(
      loaderIncompatibilities('1.20.4', { primaryLoader: 'fabric' })
        .optifineDisabledReason,
    ).toBeUndefined()
  })

  it('NeoForge selection disables OptiFine and LiteLoader', () => {
    const incomp = loaderIncompatibilities('1.20.1', { primaryLoader: 'neoforge' })
    expect(incomp.optifineDisabledReason).toBe('download.compat.neoforgeOptifine')
    expect(incomp.liteloaderDisabledReason).toBe(
      'download.compat.neoforgeLiteloader',
    )
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
    expect(validateInstanceNameLocal(' leadingSpace')).toBe(
      'instance.name.invalidChar',
    )
    expect(validateInstanceNameLocal('trailingDot.')).toBe(
      'instance.name.trailingDot',
    )
    expect(validateInstanceNameLocal('CON')).toBe('instance.name.reserved')
    expect(validateInstanceNameLocal('aux.txt')).toBe('instance.name.reserved')

    expect(
      validateInstanceNameLocal('1.20.1-Fabric', ['1.20.1-fabric']),
    ).toBe('instance.name.duplicate')
    expect(
      validateInstanceNameLocal('1.20.1-Fabric', ['1.20.1-forge']),
    ).toBeNull()
  })
})

describe('compat isAprilFools', () => {
  it('detects 2.0 and known joke snapshot IDs', () => {
    expect(isAprilFools('2.0-purple')).toBe(true)
    expect(isAprilFools('15w14a')).toBe(true)
    expect(isAprilFools('24w14potato')).toBe(true)
    expect(isAprilFools('1.20.1')).toBe(false)
  })
})

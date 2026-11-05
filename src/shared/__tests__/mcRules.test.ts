import { describe, expect, it } from 'vitest'
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, resolve } from 'node:path'
import { compareVersions, dropOf, isRelease, majorKey } from '../mcver'
import { expandModLoaders, incompatiblePairs, parseLoader, supportedLoaders } from '../loader'
import { localEntryNames, modDisplayName, modSubtitle } from '../modName'
import { formatBytes } from '../format'

const __dirname = dirname(fileURLToPath(import.meta.url))
const fixturePath = resolve(__dirname, '../../../crates/mclr-core/tests/fixtures/mc_rules.json')
const fixture = JSON.parse(readFileSync(fixturePath, 'utf-8'))

describe('mc_rules golden fixture parity', () => {
  it('compares versions identically to Rust', () => {
    for (const [a, b, expected] of fixture.compare) {
      expect(compareVersions(a, b)).toBe(expected)
    }
  })

  it('determines isRelease identically to Rust', () => {
    for (const [id, expected] of fixture.isRelease) {
      expect(isRelease(id)).toBe(expected)
    }
  })

  it('computes dropOf identically to Rust', () => {
    for (const [id, expected] of fixture.dropOf) {
      expect(dropOf(id)).toBe(expected)
    }
  })

  it('extracts majorKey identically to Rust', () => {
    for (const [id, expected] of fixture.majorKey) {
      expect(majorKey(id)).toBe(expected)
    }
  })

  it('parses loader labels identically to Rust', () => {
    for (const [label, expected] of fixture.parseLoader) {
      expect(parseLoader(label)).toBe(expected)
    }
  })

  it('expands mod loaders identically to Rust', () => {
    for (const [labels, mc, expected] of fixture.expand) {
      expect(expandModLoaders(labels, mc)).toEqual(expected)
    }
  })

  it('determines supported loaders identically to Rust in enum order', () => {
    for (const [mc, expected] of fixture.supported) {
      expect(supportedLoaders(mc)).toEqual(expected)
    }
  })

  it('determines incompatible loader pairs identically to Rust', () => {
    for (const [mc, expected] of fixture.incompatible) {
      expect(incompatiblePairs(mc)).toEqual(expected)
    }
  })
})

describe('modName helpers', () => {
  it('modDisplayName selects the appropriate display name', () => {
    const mod = { zhName: '高亮显示', title: 'Jade', slug: 'jade' }
    expect(modDisplayName(mod, true)).toBe('高亮显示')
    expect(modDisplayName(mod, false)).toBe('Jade')

    const noZh = { zhName: null, title: 'JEI', slug: 'jei' }
    expect(modDisplayName(noZh, true)).toBe('JEI')

    const slugOnly = { slug: 'sodium' }
    expect(modDisplayName(slugOnly, false)).toBe('sodium')
  })

  it('modSubtitle shows title only when zhName !== title and showZh is true', () => {
    const mod = { zhName: '高亮显示', title: 'Jade' }
    expect(modSubtitle(mod, true)).toBe('Jade')
    expect(modSubtitle(mod, false)).toBeNull()

    const same = { zhName: 'JEI', title: 'JEI' }
    expect(modSubtitle(same, true)).toBeNull()

    const noZh = { zhName: null, title: 'Fabric API' }
    expect(modSubtitle(noZh, true)).toBeNull()
  })

  it('localEntryNames handles style swap and non-mod kinds', () => {
    const entry = { zhName: '机械动力', fileName: 'create-1.20.1.jar' }
    // style 0: zhName leads
    expect(localEntryNames(entry, 'mod', true, 0)).toEqual({
      primary: '机械动力',
      secondary: 'create-1.20.1.jar',
    })

    // style 1: fileName leads
    expect(localEntryNames(entry, 'mod', true, 1)).toEqual({
      primary: 'create-1.20.1.jar',
      secondary: '机械动力',
    })

    // showZh = false: fileName only, no secondary
    expect(localEntryNames(entry, 'mod', false, 0)).toEqual({
      primary: 'create-1.20.1.jar',
      secondary: null,
    })

    // Non-mod kind: no secondary name even with zhName
    expect(localEntryNames(entry, 'resourcepack', true, 0)).toEqual({
      primary: 'create-1.20.1.jar',
      secondary: null,
    })
  })
})

describe('formatBytes helper', () => {
  it('formats boundary sizes correctly', () => {
    expect(formatBytes(0)).toBe('0 B')
    expect(formatBytes(1023)).toBe('1023 B')
    expect(formatBytes(1536)).toBe('1.5 KB')
    expect(formatBytes(150 * 1024 * 1024)).toBe('150 MB')
    expect(formatBytes(1.5 * 1024 * 1024 * 1024)).toBe('1.5 GB')
  })
})

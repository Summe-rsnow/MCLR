import { describe, it, expect } from 'vitest'
import type { ResourceEntry } from '../../../ipc/client'

describe('Resources feature unit tests', () => {
  const dummyEntries: ResourceEntry[] = [
    {
      path: '/path/to/mods/fabric-api.jar',
      relPath: 'fabric-api.jar',
      fileName: 'fabric-api.jar',
      isDir: false,
      size: 2048000,
      createdMs: 1000,
      modifiedMs: 2000,
      enabled: true,
      state: 'ok',
      title: 'Fabric API',
      description: 'Core API for Fabric',
      version: '0.92.0',
      icon: null,
      tags: ['Fabric'],
      modId: 'fabric-api',
      projectId: 'fabric-api',
      latestFile: null,
      updateAvailable: false,
      dependents: [],
    },
    {
      path: '/path/to/mods/sodium.jar',
      relPath: 'sodium.jar',
      fileName: 'sodium.jar',
      isDir: false,
      size: 1024000,
      createdMs: 2000,
      modifiedMs: 3000,
      enabled: true,
      state: 'ok',
      title: 'Sodium',
      description: 'Rendering engine',
      version: '0.5.8',
      icon: null,
      tags: ['Fabric'],
      modId: 'sodium',
      projectId: 'sodium',
      latestFile: {
        id: 'file-2',
        platform: 'modrinth',
        projectId: 'sodium',
        name: 'sodium-0.5.11.jar',
        fileName: 'sodium-0.5.11.jar',
        channel: 'release',
        releaseDate: '2024-05-01',
        size: 1050000,
        mcVersions: ['1.20.1'],
        loaders: ['fabric'],
        downloadUrls: ['https://example.com/sodium.jar'],
        hashes: { sha1: '123', sha512: null, murmur2: null },
        dependencies: [],
        distributionDisabled: false,
      },
      updateAvailable: true,
      dependents: [],
    },
    {
      path: '/path/to/mods/iris.jar.disabled',
      relPath: 'iris.jar.disabled',
      fileName: 'iris.jar.disabled',
      isDir: false,
      size: 3000000,
      createdMs: 500,
      modifiedMs: 1500,
      enabled: false,
      state: 'ok',
      title: 'Iris Shaders',
      description: 'Shaderpack support',
      version: '1.6.17',
      icon: null,
      tags: ['Fabric'],
      modId: 'iris',
      projectId: 'iris',
      latestFile: null,
      updateAvailable: false,
      dependents: [],
    },
  ]

  it('correctly filters enabled and disabled resources', () => {
    const enabled = dummyEntries.filter((e) => e.enabled)
    const disabled = dummyEntries.filter((e) => !e.enabled)

    expect(enabled.length).toBe(2)
    expect(disabled.length).toBe(1)
    expect(disabled[0].title).toBe('Iris Shaders')
  })

  it('correctly identifies updatable resources', () => {
    const updatable = dummyEntries.filter((e) => e.updateAvailable)
    expect(updatable.length).toBe(1)
    expect(updatable[0].title).toBe('Sodium')
    expect(updatable[0].latestFile?.name).toBe('sodium-0.5.11.jar')
  })

  it('computes weighted search score accurately', () => {
    function computeScore(entry: ResourceEntry, query: string): number {
      if (!query) return 1
      const q = query.toLowerCase()
      let score = 0
      if (entry.title?.toLowerCase().includes(q)) score += 1.0
      if (entry.fileName.toLowerCase().includes(q)) score += 1.0
      if (entry.description?.toLowerCase().includes(q)) score += 0.4
      if (entry.version?.toLowerCase().includes(q)) score += 0.2
      for (const tag of entry.tags) {
        if (tag.toLowerCase().includes(q)) {
          score += 0.2
          break
        }
      }
      return score
    }

    const sodium = dummyEntries[1]
    expect(computeScore(sodium, 'sodium')).toBeGreaterThanOrEqual(1.0)
    expect(computeScore(sodium, 'rendering')).toBeCloseTo(0.4)
    expect(computeScore(sodium, 'fabric')).toBeCloseTo(0.2)
    expect(computeScore(sodium, 'nonexistent')).toBe(0)
  })

  it('sorts entries by size, creation time, and name', () => {
    const sortedBySize = [...dummyEntries].sort((a, b) => b.size - a.size)
    expect(sortedBySize[0].title).toBe('Iris Shaders')
    expect(sortedBySize[2].title).toBe('Sodium')

    const sortedByName = [...dummyEntries].sort((a, b) =>
      (a.title || a.fileName).localeCompare(b.title || b.fileName),
    )
    expect(sortedByName[0].title).toBe('Fabric API')
    expect(sortedByName[1].title).toBe('Iris Shaders')
    expect(sortedByName[2].title).toBe('Sodium')
  })

  it('supports batch toggle logic for all entries', () => {
    // Simulate enable all
    const allEnabled = dummyEntries.map((e) => ({ ...e, enabled: true }))
    expect(allEnabled.every((e) => e.enabled)).toBe(true)

    // Simulate disable all
    const allDisabled = dummyEntries.map((e) => ({ ...e, enabled: false }))
    expect(allDisabled.every((e) => !e.enabled)).toBe(true)

    // Simulate batch toggle on selection
    const selected = new Set([dummyEntries[0].path, dummyEntries[2].path])
    const batchDisabled = dummyEntries.map((e) =>
      selected.has(e.path) ? { ...e, enabled: false } : e,
    )
    expect(batchDisabled.find((e) => e.path === dummyEntries[0].path)?.enabled).toBe(false)
    expect(batchDisabled.find((e) => e.path === dummyEntries[1].path)?.enabled).toBe(true)
  })
})

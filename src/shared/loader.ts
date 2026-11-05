import type { LoaderKind as BindingsLoaderKind } from '@/ipc/bindings'
import { compareVersions, dropOf } from './mcver'

export type LoaderKindBinding =
  | 'fabric'
  | 'legacyFabric'
  | 'liteLoader'
  | 'labyMod'
  | 'optiFine'
  | 'forge'
  | 'neoForge'
  | 'cleanroom'

export type LoaderId =
  | 'fabric'
  | 'quilt'
  | 'legacyfabric'
  | 'forge'
  | 'neoforge'
  | 'cleanroom'
  | 'liteloader'
  | 'optifine'
  | 'labymod'

/**
 * Parses a loader label into a canonical `LoaderId`.
 *
 * Drops whitespace, `_`, `-`, converts to lowercase.
 * Matches in order: `legacyfabric`, `neoforge`, `quilt`, `fabric`, `cleanroom`, `forge`, `liteloader`, `optifine`, `labymod`.
 */
export function parseLoader(label: string | null | undefined): LoaderId | null {
  if (!label) return null
  const clean = label.toLowerCase().replace(/[\s_-]/g, '')
  if (!clean) return null

  if (clean.includes('legacyfabric')) return 'legacyfabric'
  if (clean.includes('neoforge')) return 'neoforge'
  if (clean.includes('quilt')) return 'quilt'
  if (clean.includes('fabric')) return 'fabric'
  if (clean.includes('cleanroom')) return 'cleanroom'
  if (clean.includes('forge')) return 'forge'
  if (clean.includes('liteloader')) return 'liteloader'
  if (clean.includes('optifine')) return 'optifine'
  if (clean.includes('labymod')) return 'labymod'

  return null
}

/**
 * Whether this loader can run/install mods. False only for `optifine` and `labymod`.
 */
export function isModableLoader(id: LoaderId): boolean {
  return id !== 'optifine' && id !== 'labymod'
}

/**
 * Priority rank for sorting / primary loader selection (lower number = higher priority).
 */
export function loaderPriority(id: LoaderId): number {
  switch (id) {
    case 'fabric':
      return 1
    case 'quilt':
      return 2
    case 'legacyfabric':
      return 3
    case 'neoforge':
      return 4
    case 'forge':
      return 5
    case 'cleanroom':
      return 6
    case 'labymod':
      return 7
    case 'optifine':
      return 8
    case 'liteloader':
      return 9
  }
}

/**
 * Determines the primary loader from candidates by minimum priority.
 */
export function primaryLoader(candidates: (string | undefined | null)[]): LoaderId | null {
  let best: LoaderId | null = null
  let bestScore = 99

  for (const candidate of candidates) {
    const parsed = parseLoader(candidate)
    if (parsed) {
      const score = loaderPriority(parsed)
      if (score < bestScore) {
        bestScore = score
        best = parsed
      }
    }
  }

  return best
}

/**
 * Maps a loader id into a search facet id for community platforms (`legacyfabric -> 'fabric'`).
 */
export function searchLoaderId(id: LoaderId | null | undefined): string | null {
  if (!id) return null
  return id === 'legacyfabric' ? 'fabric' : id
}

/**
 * Expands loader labels into mod targets (order-preserving, deduplicated).
 */
export function expandModLoaders(labels: string[], mc: string): string[] {
  const cleanMc = mc.trim()
  const out: string[] = []

  const push = (id: string) => {
    const lower = id.trim().toLowerCase()
    if (lower && !out.includes(lower)) {
      out.push(lower)
    }
  }

  for (const label of labels) {
    const parsed = parseLoader(label)
    if (parsed === 'neoforge') {
      push('neoforge')
      if (cleanMc === '1.20.1') {
        push('forge')
      }
    } else if (parsed === 'quilt') {
      push('quilt')
      push('fabric')
    } else if (parsed === 'cleanroom') {
      push('forge')
    } else if (parsed === 'legacyfabric') {
      push('fabric')
    } else if (parsed) {
      push(parsed)
    } else {
      push(label)
    }
  }

  return out
}

function isStandardRelease(mc: string): boolean {
  return mc.startsWith('1.') || /^\d{2}/.test(mc)
}

/**
 * Returns supported loaders for the given Minecraft version in enum declaration order.
 */
export function supportedLoaders(mc: string): LoaderId[] {
  const clean = mc.trim()
  const standard = isStandardRelease(clean)
  const drop = dropOf(clean)
  const supported: LoaderId[] = []

  if (drop >= 130) supported.push('fabric')
  if (drop >= 130) supported.push('quilt')
  if (drop >= 30 && drop <= 130) supported.push('legacyfabric')
  if (standard && compareVersions(clean, '1.1') >= 0) supported.push('forge')
  if (standard && compareVersions(clean, '1.20.1') >= 0 && clean !== '1.20' && clean !== '1.20.0') {
    supported.push('neoforge')
  }
  if (clean === '1.12.2') supported.push('cleanroom')
  if (standard && compareVersions(clean, '1.5.2') >= 0 && compareVersions(clean, '1.12.2') <= 0) {
    supported.push('liteloader')
  }
  if (compareVersions(clean, '1.7.2') >= 0) supported.push('optifine')
  if (standard && compareVersions(clean, '1.8.9') >= 0) supported.push('labymod')

  return supported
}

/**
 * Incompatible loader pairs for the given Minecraft version.
 */
export function incompatiblePairs(mc: string): [LoaderId, LoaderId][] {
  const clean = mc.trim()
  const pairs: [LoaderId, LoaderId][] = [
    ['fabric', 'forge'],
    ['fabric', 'neoforge'],
    ['forge', 'neoforge'],
    ['cleanroom', 'optifine'],
    ['cleanroom', 'liteloader'],
    ['neoforge', 'liteloader'],
    ['labymod', 'optifine'],
    ['labymod', 'liteloader'],
    ['legacyfabric', 'liteloader'],
  ]

  if (compareVersions(clean, '1.20.5') >= 0) {
    pairs.push(['fabric', 'optifine'])
  }
  if (compareVersions(clean, '1.20.2') >= 0) {
    pairs.push(['neoforge', 'optifine'])
  }
  if (compareVersions(clean, '1.13') >= 0 && compareVersions(clean, '1.14.3') <= 0) {
    pairs.push(['forge', 'optifine'])
  }

  return pairs
}

/**
 * Returns true if the loader is supported for the given Minecraft version.
 */
export function isLoaderSupported(id: LoaderId, mc: string): boolean {
  return supportedLoaders(mc).includes(id)
}

/**
 * Maps a canonical `LoaderId` to the Tauri `LoaderKind` binding.
 */
export function toLoaderKind(id: LoaderId): LoaderKindBinding {
  switch (id) {
    case 'neoforge':
      return 'neoForge'
    case 'legacyfabric':
      return 'legacyFabric'
    case 'liteloader':
      return 'liteLoader'
    case 'optifine':
      return 'optiFine'
    case 'labymod':
      return 'labyMod'
    case 'fabric':
      return 'fabric'
    case 'forge':
      return 'forge'
    case 'cleanroom':
      return 'cleanroom'
    case 'quilt':
      // Backend installer treats quilt as fabric
      return 'fabric'
  }
}

/**
 * Maps a Tauri `LoaderKind` binding back to canonical `LoaderId`.
 */
export function fromLoaderKind(kind: LoaderKindBinding | BindingsLoaderKind): LoaderId {
  return parseLoader(kind) ?? 'fabric'
}

/**
 * Compares two loaders by `loaderPriority`, then lexicographically for unknown loaders.
 */
export function compareLoaders(a: string, b: string): number {
  const parsedA = parseLoader(a)
  const parsedB = parseLoader(b)
  const prioA = parsedA ? loaderPriority(parsedA) : 50
  const prioB = parsedB ? loaderPriority(parsedB) : 50

  if (prioA !== prioB) {
    return prioA - prioB
  }
  return a.localeCompare(b)
}

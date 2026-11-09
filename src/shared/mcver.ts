const SEPARATOR_REGEX = /[.\-+_]/
const NUMERIC_REGEX = /^\d+$/

function numeric(segment: string): string | null {
  if (!NUMERIC_REGEX.test(segment)) {
    return null
  }
  const trimmed = segment.replace(/^0+/, '')
  return trimmed === '' ? '0' : trimmed
}

function compareNumeric(a: string, b: string): number {
  if (a.length !== b.length) {
    return a.length - b.length < 0 ? -1 : 1
  }
  if (a < b) return -1
  if (a > b) return 1
  return 0
}

function compareSegment(a: string, b: string): number {
  const numA = numeric(a)
  const numB = numeric(b)

  if (numA !== null && numB !== null) {
    return compareNumeric(numA, numB)
  }
  if (numA !== null && numB === null) {
    return 1
  }
  if (numA === null && numB !== null) {
    return -1
  }
  if (a < b) return -1
  if (a > b) return 1
  return 0
}

/**
 * Total order comparison between two version strings.
 *
 * Segments split on `.` `-` `+` `_`, empties dropped, missing segment = `"0"`.
 * Numeric segment > non-numeric segment, two numeric segments compare numerically
 * without integer overflow, two non-numeric segments compare lexicographically.
 */
export function compareVersions(a: string, b: string): number {
  const partsA = a
    .trim()
    .split(SEPARATOR_REGEX)
    .map((s) => s.trim())
    .filter((s) => s.length > 0)

  const partsB = b
    .trim()
    .split(SEPARATOR_REGEX)
    .map((s) => s.trim())
    .filter((s) => s.length > 0)

  const len = Math.max(partsA.length, partsB.length)
  for (let i = 0; i < len; i++) {
    const segA = partsA[i] ?? '0'
    const segB = partsB[i] ?? '0'
    const cmp = compareSegment(segA, segB)
    if (cmp !== 0) {
      return cmp
    }
  }
  return 0
}

export const compareVersionsDesc = (a: string, b: string): number => compareVersions(b, a)

/**
 * Normalised game version, or `null` when the instance does not report a real one.
 * Mirrors `mclr_core::mcver::known`: blank and `unknown` (any case) mean "not detected".
 */
export function knownVersion(raw?: string | null): string | null {
  const trimmed = (raw ?? '').trim()
  if (!trimmed || trimmed.toLowerCase() === 'unknown') return null
  return trimmed
}

const RELEASE_REGEX = /^\d+\.\d+(\.\d+)?$/

/**
 * `true` for standard release identifiers (e.g. `1.20.1`, `1.7`, `26.1`, `26.1.2`),
 * `false` for snapshots, pre-releases, and unversioned names.
 */
export function isRelease(id: string): boolean {
  return RELEASE_REGEX.test(id.trim())
}

/**
 * Calculates drop score for Minecraft versions (e.g. 1.20.1 -> 200, 1.14 -> 140, 26.1 -> 260).
 */
export function dropOf(id: string): number {
  const clean = id.trim()
  if (clean.startsWith('1.')) {
    const rest = clean.slice(2)
    const minorStr = rest.split('.')[0]
    const minor = parseInt(minorStr, 10)
    return isNaN(minor) ? 0 : minor * 10
  }

  const dotIndex = clean.indexOf('.')
  if (dotIndex !== -1) {
    const first = parseInt(clean.slice(0, dotIndex), 10)
    if (!isNaN(first) && first >= 26) {
      return first * 10
    }
  }

  if (clean.length >= 3 && /^\d{2}/.test(clean) && clean.includes('w')) {
    const yr = parseInt(clean.slice(0, 2), 10)
    if (yr >= 24) return 210
    if (yr === 23) return 200
    if (yr === 22) return 190
    if (yr === 21) return 180
    if (yr === 20) return 160
    if (yr === 19) return 150
    if (yr === 18) return 130
    if (yr === 17) return 120
    if (yr === 16) return 110
    if (yr === 15) return 90
    return 0
  }

  return 0
}

/**
 * Major version key for release versions (e.g. `"1.20.1"` -> `"1.20"`, `"26.1.2"` -> `"26.1"`).
 * Returns `null` for non-release versions.
 */
export function majorKey(id: string): string | null {
  if (!isRelease(id)) {
    return null
  }
  const clean = id.trim()
  const firstDot = clean.indexOf('.')
  if (firstDot === -1) {
    return clean
  }
  const secondDot = clean.indexOf('.', firstDot + 1)
  if (secondDot === -1) {
    return clean
  }
  return clean.slice(0, secondDot)
}

export const MAJOR_SNAPSHOT = '__snapshot__'
export const MAJOR_OTHER = '__other__'

export function groupKeyOf(id: string): string {
  return majorKey(id) ?? (id.trim() ? MAJOR_SNAPSHOT : MAJOR_OTHER)
}

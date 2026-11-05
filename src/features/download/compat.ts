import { i18n } from '@/i18n'
import { compareVersions } from '@/shared/mcver'
import { parseLoader } from '@/shared/loader'

/**
 * PCL-CE parity rules for Minecraft version detection, compatibility, add-ons, and instance naming.
 * Owned by slice B1.
 */

const FORBIDDEN_CHARS = ['\\', '/', ':', '*', '?', '"', '<', '>', '|', '!', ';']

const RESERVED_DOS_NAMES: Record<string, true> = {
  CON: true,
  PRN: true,
  AUX: true,
  NUL: true,
  COM1: true,
  COM2: true,
  COM3: true,
  COM4: true,
  COM5: true,
  COM6: true,
  COM7: true,
  COM8: true,
  COM9: true,
  LPT1: true,
  LPT2: true,
  LPT3: true,
  LPT4: true,
  LPT5: true,
  LPT6: true,
  LPT7: true,
  LPT8: true,
  LPT9: true,
}

const APRIL_FOOLS_IDS: Record<string, true> = {
  '15w14a': true,
  '1.rv-pre1': true,
  '3d shareware v1.34': true,
  '20w14∞': true,
  '20w14infinite': true,
  '22w13oneblockatatime': true,
  '23w13a_or_b': true,
  '24w14potato': true,
  '25w14craftmine': true,
  '26w14a': true,
}

export function isAprilFools(id: string, releaseTime?: string): boolean {
  const lower = id.toLowerCase()
  if (
    lower.startsWith('2.0') ||
    lower.startsWith('2point0') ||
    lower.startsWith('20w14inf') ||
    APRIL_FOOLS_IDS[lower]
  ) {
    return true
  }
  if (releaseTime) {
    const date = new Date(releaseTime)
    if (!isNaN(date.getTime())) {
      const shifted = new Date(date.getTime() + 2 * 3600 * 1000)
      if (shifted.getUTCMonth() === 3 && shifted.getUTCDate() === 1) {
        return true
      }
    }
  }
  return false
}

export function getMcFoolName(name: string): string {
  const lower = name.toLowerCase()
  let code = ''
  if (lower.startsWith('2.0') || lower.startsWith('2point0')) {
    if (lower.includes('red')) code = '2_0_red'
    else if (lower.includes('blue')) code = '2_0_blue'
    else if (lower.includes('purple')) code = '2_0_purple'
    else code = '2_0'
  } else if (lower === '15w14a') {
    code = '15w14a'
  } else if (lower === '1.rv-pre1' || lower === '1_rv-pre1') {
    code = '1_rv-pre1'
  } else if (lower === '3d shareware v1.34' || lower === '3d_shareware') {
    code = '3d_shareware'
  } else if (lower.startsWith('20w14inf') || lower === '20w14∞' || lower === '20w14infinite') {
    code = '20w14infinite'
  } else if (lower === '22w13oneblockatatime') {
    code = '22w13oneblockatatime'
  } else if (lower === '23w13a_or_b') {
    code = '23w13a_or_b'
  } else if (lower === '24w14potato') {
    code = '24w14potato'
  } else if (lower === '25w14craftmine') {
    code = '25w14craftmine'
  } else if (lower === '26w14a') {
    code = '26w14a'
  }
  if (!code) return ''
  const key = `download.foolNames.${code}`
  const translated = i18n.global.t(key)
  return translated === key ? '' : translated
}

export type McVersionCategory = 'release' | 'snapshot' | 'old' | 'aprilFools'

export interface VersionLike {
  id: string
  versionType?: string
  type?: string
  releaseTime?: string
}

export function classifyVersion(
  versionOrId: VersionLike | string,
  versionType?: string,
  releaseTime?: string,
): McVersionCategory {
  let id = ''
  let rawType = versionType ?? ''
  let time = releaseTime
  if (typeof versionOrId === 'object' && versionOrId !== null) {
    id = versionOrId.id
    rawType = versionOrId.versionType ?? versionOrId.type ?? rawType
    time = versionOrId.releaseTime ?? time
  } else {
    id = String(versionOrId)
  }

  const lowerType = rawType.toLowerCase()
  const lowerId = id.toLowerCase()

  // 1. Release: official release or 1.0 (Mojang manifest mislabeled 1.0 as snapshot)
  if (lowerType === 'release' || lowerId === '1.0') {
    return 'release'
  }

  // 2. April Fools: special type or joke version IDs / April 1st release dates
  if (lowerType === 'special' || isAprilFools(id, time)) {
    return 'aprilFools'
  }

  // 3. Snapshots / Pending
  if (lowerType === 'snapshot' || lowerType === 'pending') {
    if (
      lowerId.startsWith('1.') &&
      !lowerId.includes('combat') &&
      !lowerId.includes('rc') &&
      !lowerId.includes('experimental') &&
      lowerId !== '1.2' &&
      !lowerId.includes('pre')
    ) {
      return 'release'
    }
    return 'snapshot'
  }

  // 4. Old versions (alpha, beta, indev, infdev, classic)
  return 'old'
}

export interface LoaderIncompatibilities {
  optifineDisabledReason?: string
  liteloaderDisabledReason?: string
}

export function loaderIncompatibilities(
  mcVersion: string,
  selected: {
    primaryLoader?:
      | 'forge'
      | 'neoforge'
      | 'cleanroom'
      | 'fabric'
      | 'legacyFabric'
      | 'labymod'
      | null
    optifine?: boolean
    liteloader?: boolean
  },
): LoaderIncompatibilities {
  const result: LoaderIncompatibilities = {}
  const { primaryLoader } = selected

  if (
    primaryLoader === 'neoforge' ||
    primaryLoader === 'cleanroom' ||
    primaryLoader === 'labymod'
  ) {
    result.optifineDisabledReason = `download.compat.${primaryLoader}Optifine`
    result.liteloaderDisabledReason = `download.compat.${primaryLoader}Liteloader`
  }

  if (primaryLoader === 'legacyFabric') {
    result.liteloaderDisabledReason = 'download.compat.legacyFabricLiteloader'
  }

  if (primaryLoader === 'forge') {
    if (compareVersions(mcVersion, '1.13') >= 0 && compareVersions(mcVersion, '1.14.3') <= 0) {
      result.optifineDisabledReason = 'download.compat.forgeOptifineVersion'
    }
  }

  if (primaryLoader === 'fabric') {
    if (compareVersions(mcVersion, '1.20.5') >= 0) {
      result.optifineDisabledReason = 'download.compat.fabricOptifineVersion'
    }
  }

  return result
}

/** Display names for loader ids, used in summaries and picker labels. */
export const LOADER_DISPLAY_NAME: Record<string, string> = {
  none: 'Vanilla',
  vanilla: 'Vanilla',
  fabric: 'Fabric',
  legacyfabric: 'Legacy Fabric',
  legacyFabric: 'Legacy Fabric',
  forge: 'Forge',
  neoforge: 'NeoForge',
  cleanroom: 'Cleanroom',
  labymod: 'LabyMod',
  quilt: 'Quilt',
  liteloader: 'LiteLoader',
  optifine: 'OptiFine',
}

export function loaderDisplayName(id?: string | null): string {
  if (!id) return LOADER_DISPLAY_NAME.none
  const parsed = parseLoader(id)
  if (parsed && LOADER_DISPLAY_NAME[parsed]) {
    return LOADER_DISPLAY_NAME[parsed]
  }
  return LOADER_DISPLAY_NAME[id] ?? LOADER_DISPLAY_NAME[id.toLowerCase()] ?? id
}

export interface DefaultNameInput {
  mcVersion: string
  fabricVersion?: string | null
  legacyFabricVersion?: string | null
  labymodVersion?: string | null
  labymodChannel?: string | null
  forgeVersion?: string | null
  neoforgeVersion?: string | null
  cleanroomVersion?: string | null
  liteloader?: boolean
  optifineDisplay?: string | null
}

export function defaultName(input: DefaultNameInput): string {
  let name = input.mcVersion

  if (input.fabricVersion) {
    const cleanVer = input.fabricVersion.split('+build')[0].split('+')[0]
    name += `-Fabric_${cleanVer}`
  }
  if (input.legacyFabricVersion) {
    name += `-LegacyFabric_${input.legacyFabricVersion}`
  }
  if (input.labymodVersion) {
    const snapshot = input.labymodChannel?.toLowerCase() === 'snapshot'
    name += `-LabyMod_${input.labymodVersion}${snapshot ? '_Snapshot' : ''}`
  }
  if (input.forgeVersion) {
    name += `-Forge_${input.forgeVersion}`
  }
  if (input.neoforgeVersion) {
    name += `-NeoForge_${input.neoforgeVersion}`
  }
  if (input.cleanroomVersion) {
    name += `-Cleanroom_${input.cleanroomVersion}`
  }
  if (input.liteloader) {
    name += '-LiteLoader'
  }
  if (input.optifineDisplay) {
    const withoutMc = input.optifineDisplay.replace(new RegExp(`^${input.mcVersion}\\s*`), '')
    const clean = withoutMc.replace(/\s+/g, '_')
    name += `-OptiFine_${clean}`
  }

  return name
}

export function validateInstanceNameLocal(
  name: string,
  existingNames: string[] = [],
): string | null {
  if (!name || name.length === 0) {
    return 'instance.name.empty'
  }
  if (name.length > 70) {
    return 'instance.name.tooLong'
  }
  if (name.startsWith(' ') || name.endsWith(' ')) {
    return 'instance.name.invalidChar'
  }
  for (const char of FORBIDDEN_CHARS) {
    if (name.includes(char)) {
      return 'instance.name.invalidChar'
    }
  }
  if (name.endsWith('.')) {
    return 'instance.name.trailingDot'
  }

  const upper = name.toUpperCase()
  const base = upper.split('.')[0]
  if (RESERVED_DOS_NAMES[upper] || RESERVED_DOS_NAMES[base]) {
    return 'instance.name.reserved'
  }

  const lowerName = name.toLowerCase()
  if (existingNames.some((ex) => ex.toLowerCase() === lowerName)) {
    return 'instance.name.duplicate'
  }

  return null
}

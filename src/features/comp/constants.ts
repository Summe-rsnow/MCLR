import type { CompPlatform, CompSort, CompType } from '../../ipc/client'

export type CompPlatformFilter = CompPlatform | 'all'

/**
 * One category as it appears in the toolbar dropdown.
 *
 * `value` is the launcher-side key and the only thing the store keeps; the platform ids are
 * what each search backend actually needs. CurseForge addresses categories by a numeric id that
 * is scoped to a class id, Modrinth by a slug, so a category may exist on one platform only —
 * `mrSlug`/`cfId` being absent is exactly what disables the option there.
 */
export interface CompCategoryDef {
  value: string
  labelKey: string
  cfId?: number
  mrSlug?: string
}

export interface CompLoaderDef {
  value: string
  /** Brand names are not translated; only the "vanilla" fallback needs a label key. */
  label?: string
  labelKey?: string
  icon?: string
  image?: string
  platforms: CompPlatform[]
}

/** Official loader artwork, shared with `MLoaderIcon` via the same public assets. */
const LOADER_LOGO = (kind: string) => `/assets/icons/loaders/${kind}.png`

const MOD_LOADERS: CompLoaderDef[] = [
  {
    value: 'fabric',
    label: 'Fabric',
    image: LOADER_LOGO('fabric'),
    platforms: ['modrinth', 'curseforge'],
  },
  {
    value: 'forge',
    label: 'Forge',
    image: LOADER_LOGO('forge'),
    platforms: ['modrinth', 'curseforge'],
  },
  {
    value: 'neoforge',
    label: 'NeoForge',
    image: LOADER_LOGO('neoforge'),
    platforms: ['modrinth', 'curseforge'],
  },
  {
    value: 'quilt',
    label: 'Quilt',
    image: LOADER_LOGO('quilt'),
    platforms: ['modrinth', 'curseforge'],
  },
]

/** Shaders are not mods: iris/optifine/canvas are Modrinth-only loader facets. */
const SHADER_LOADERS: CompLoaderDef[] = [
  {
    value: 'iris',
    label: 'Iris',
    icon: 'i-lucide-gem',
    platforms: ['modrinth'],
  },
  { value: 'optifine', label: 'OptiFine', image: LOADER_LOGO('optifine'), platforms: ['modrinth'] },
  {
    value: 'vanilla',
    labelKey: 'comp.search.loaders.vanilla',
    icon: 'i-lucide-box',
    platforms: ['modrinth'],
  },
  { value: 'canvas', label: 'Canvas', icon: 'i-lucide-palette', platforms: ['modrinth'] },
]

export function getLoaderDefsForType(compType: CompType): CompLoaderDef[] {
  switch (compType) {
    case 'mod':
    case 'modpack':
      return MOD_LOADERS
    case 'shader':
      return SHADER_LOADERS
    default:
      // Resource packs, datapacks and worlds are not loader-bound on either platform.
      return []
  }
}

export const COMP_CATEGORIES_BY_TYPE: Record<CompType, CompCategoryDef[]> = {
  mod: [
    {
      value: 'technology',
      labelKey: 'comp.search.categories.technology',
      cfId: 412,
      mrSlug: 'technology',
    },
    { value: 'magic', labelKey: 'comp.search.categories.magic', cfId: 419, mrSlug: 'magic' },
    {
      value: 'adventure',
      labelKey: 'comp.search.categories.adventure',
      cfId: 422,
      mrSlug: 'adventure',
    },
    {
      value: 'optimization',
      labelKey: 'comp.search.categories.optimization',
      cfId: 6814,
      mrSlug: 'optimization',
    },
    { value: 'utility', labelKey: 'comp.search.categories.utility', cfId: 5191, mrSlug: 'utility' },
    {
      value: 'decoration',
      labelKey: 'comp.search.categories.decoration',
      cfId: 424,
      mrSlug: 'decoration',
    },
    { value: 'storage', labelKey: 'comp.search.categories.storage', cfId: 420, mrSlug: 'storage' },
    {
      value: 'equipment',
      labelKey: 'comp.search.categories.equipment',
      cfId: 434,
      mrSlug: 'equipment',
    },
    { value: 'food', labelKey: 'comp.search.categories.food', cfId: 436, mrSlug: 'food' },
    {
      value: 'worldgen',
      labelKey: 'comp.search.categories.worldgen',
      cfId: 406,
      mrSlug: 'worldgen',
    },
    { value: 'library', labelKey: 'comp.search.categories.library', cfId: 421, mrSlug: 'library' },
    { value: 'mobs', labelKey: 'comp.search.categories.mobs', cfId: 411, mrSlug: 'mobs' },
    {
      value: 'transportation',
      labelKey: 'comp.search.categories.transportation',
      cfId: 414,
      mrSlug: 'transportation',
    },
    { value: 'social', labelKey: 'comp.search.categories.social', cfId: 435, mrSlug: 'social' },
    { value: 'biomes', labelKey: 'comp.search.categories.biomes', cfId: 407 },
    { value: 'ores', labelKey: 'comp.search.categories.ores', cfId: 408 },
    { value: 'structures', labelKey: 'comp.search.categories.structures', cfId: 409 },
    { value: 'dimensions', labelKey: 'comp.search.categories.dimensions', cfId: 410 },
    { value: 'pipes', labelKey: 'comp.search.categories.pipes', cfId: 415 },
    { value: 'farming', labelKey: 'comp.search.categories.farming', cfId: 416 },
    { value: 'energy', labelKey: 'comp.search.categories.energy', cfId: 417 },
    { value: 'display', labelKey: 'comp.search.categories.display', cfId: 423 },
    { value: 'redstone', labelKey: 'comp.search.categories.redstone', cfId: 4558 },
    { value: 'automation', labelKey: 'comp.search.categories.automation', cfId: 4843 },
    { value: 'creative', labelKey: 'comp.search.categories.creative', cfId: 9026 },
    { value: 'cursed', labelKey: 'comp.search.categories.cursed', mrSlug: 'cursed' },
    {
      value: 'game-mechanics',
      labelKey: 'comp.search.categories.gameMechanics',
      mrSlug: 'game-mechanics',
    },
  ],
  modpack: [
    {
      value: 'technology',
      labelKey: 'comp.search.categories.technology',
      cfId: 4472,
      mrSlug: 'technology',
    },
    { value: 'magic', labelKey: 'comp.search.categories.magic', cfId: 4473 },
    {
      value: 'adventure',
      labelKey: 'comp.search.categories.adventure',
      cfId: 4475,
      mrSlug: 'adventure',
    },
    { value: 'quests', labelKey: 'comp.search.categories.quests', cfId: 4478, mrSlug: 'quests' },
    {
      value: 'challenging',
      labelKey: 'comp.search.categories.challenging',
      cfId: 4479,
      mrSlug: 'challenging',
    },
    {
      value: 'lightweight',
      labelKey: 'comp.search.categories.lightweight',
      cfId: 4481,
      mrSlug: 'lightweight',
    },
    { value: 'combat', labelKey: 'comp.search.categories.combat', cfId: 4483, mrSlug: 'combat' },
    {
      value: 'multiplayer',
      labelKey: 'comp.search.categories.multiplayer',
      cfId: 4484,
      mrSlug: 'multiplayer',
    },
    {
      value: 'kitchen-sink',
      labelKey: 'comp.search.categories.kitchenSink',
      mrSlug: 'kitchen-sink',
    },
    { value: 'exploration', labelKey: 'comp.search.categories.exploration', cfId: 4476 },
    { value: 'minigame', labelKey: 'comp.search.categories.minigame', cfId: 4477 },
    { value: 'scifi', labelKey: 'comp.search.categories.scifi', cfId: 4471 },
    { value: 'skyblock', labelKey: 'comp.search.categories.skyblock', cfId: 4736 },
    { value: 'vanilla-plus', labelKey: 'comp.search.categories.vanillaPlus', cfId: 5128 },
    { value: 'ftb', labelKey: 'comp.search.categories.ftb', cfId: 4487 },
    { value: 'map-based', labelKey: 'comp.search.categories.mapBased', cfId: 4480 },
    { value: 'extra-large', labelKey: 'comp.search.categories.extraLarge', cfId: 4482 },
  ],
  resourcepack: [
    { value: '16x', labelKey: 'comp.search.categories.res16x', cfId: 393, mrSlug: '16x' },
    { value: '32x', labelKey: 'comp.search.categories.res32x', cfId: 394, mrSlug: '32x' },
    { value: '64x', labelKey: 'comp.search.categories.res64x', cfId: 395, mrSlug: '64x' },
    { value: '128x', labelKey: 'comp.search.categories.res128x', cfId: 396, mrSlug: '128x' },
    { value: '256x', labelKey: 'comp.search.categories.res256x', cfId: 397, mrSlug: '256x' },
    { value: '512x+', labelKey: 'comp.search.categories.res512xPlus', cfId: 398, mrSlug: '512x+' },
    {
      value: 'realistic',
      labelKey: 'comp.search.categories.realistic',
      cfId: 400,
      mrSlug: 'realistic',
    },
    {
      value: 'vanilla-like',
      labelKey: 'comp.search.categories.vanillaLike',
      cfId: 403,
      mrSlug: 'vanilla-like',
    },
    { value: 'fonts', labelKey: 'comp.search.categories.fonts', cfId: 5244, mrSlug: 'fonts' },
    { value: 'tweaks', labelKey: 'comp.search.categories.tweaks', mrSlug: 'tweaks' },
    { value: 'simplistic', labelKey: 'comp.search.categories.simplistic', mrSlug: 'simplistic' },
    { value: 'themed', labelKey: 'comp.search.categories.themed', mrSlug: 'themed' },
    { value: 'cursed', labelKey: 'comp.search.categories.cursed', mrSlug: 'cursed' },
    { value: 'entities', labelKey: 'comp.search.categories.entities', mrSlug: 'entities' },
    { value: 'audio', labelKey: 'comp.search.categories.audio', mrSlug: 'audio' },
    { value: 'models', labelKey: 'comp.search.categories.models', mrSlug: 'models' },
    { value: 'locale', labelKey: 'comp.search.categories.locale', mrSlug: 'locale' },
    { value: 'gui', labelKey: 'comp.search.categories.gui', mrSlug: 'gui' },
    {
      value: 'core-shaders',
      labelKey: 'comp.search.categories.coreShaders',
      mrSlug: 'core-shaders',
    },
    { value: 'steampunk', labelKey: 'comp.search.categories.steampunk', cfId: 399 },
    { value: 'modern', labelKey: 'comp.search.categories.modern', cfId: 401 },
    { value: 'medieval', labelKey: 'comp.search.categories.medieval', cfId: 402 },
    { value: 'animated', labelKey: 'comp.search.categories.animated', cfId: 404 },
    { value: 'modded', labelKey: 'comp.search.categories.modded', cfId: 4465 },
  ],
  shader: [
    {
      value: 'realistic',
      labelKey: 'comp.search.categories.realistic',
      cfId: 6553,
      mrSlug: 'realistic',
    },
    { value: 'fantasy', labelKey: 'comp.search.categories.fantasy', cfId: 6554, mrSlug: 'fantasy' },
    {
      value: 'vanilla-like',
      labelKey: 'comp.search.categories.vanillaLike',
      cfId: 6555,
      mrSlug: 'vanilla-like',
    },
    {
      value: 'semi-realistic',
      labelKey: 'comp.search.categories.semiRealistic',
      mrSlug: 'semi-realistic',
    },
    { value: 'cartoon', labelKey: 'comp.search.categories.cartoon', mrSlug: 'cartoon' },
    {
      value: 'colored-lighting',
      labelKey: 'comp.search.categories.coloredLighting',
      mrSlug: 'colored-lighting',
    },
    {
      value: 'path-tracing',
      labelKey: 'comp.search.categories.pathTracing',
      mrSlug: 'path-tracing',
    },
    { value: 'pbr', labelKey: 'comp.search.categories.pbr', mrSlug: 'pbr' },
    { value: 'reflections', labelKey: 'comp.search.categories.reflections', mrSlug: 'reflections' },
    { value: 'potato', labelKey: 'comp.search.categories.perfPotato', mrSlug: 'potato' },
    { value: 'low', labelKey: 'comp.search.categories.perfLow', mrSlug: 'low' },
    { value: 'medium', labelKey: 'comp.search.categories.perfMedium', mrSlug: 'medium' },
    { value: 'high', labelKey: 'comp.search.categories.perfHigh', mrSlug: 'high' },
  ],
  datapack: [
    {
      value: 'adventure',
      labelKey: 'comp.search.categories.adventure',
      cfId: 6948,
      mrSlug: 'adventure',
    },
    { value: 'library', labelKey: 'comp.search.categories.library', cfId: 6950, mrSlug: 'library' },
    {
      value: 'technology',
      labelKey: 'comp.search.categories.technology',
      cfId: 6951,
      mrSlug: 'technology',
    },
    { value: 'magic', labelKey: 'comp.search.categories.magic', cfId: 6952, mrSlug: 'magic' },
    { value: 'utility', labelKey: 'comp.search.categories.utility', cfId: 6953, mrSlug: 'utility' },
    { value: 'fantasy', labelKey: 'comp.search.categories.fantasy', cfId: 6949 },
    { value: 'mod-related', labelKey: 'comp.search.categories.modRelated', cfId: 6946 },
    { value: 'cursed', labelKey: 'comp.search.categories.cursed', mrSlug: 'cursed' },
    {
      value: 'decoration',
      labelKey: 'comp.search.categories.decoration',
      mrSlug: 'decoration',
    },
    { value: 'equipment', labelKey: 'comp.search.categories.equipment', mrSlug: 'equipment' },
    { value: 'food', labelKey: 'comp.search.categories.food', mrSlug: 'food' },
    {
      value: 'game-mechanics',
      labelKey: 'comp.search.categories.gameMechanics',
      mrSlug: 'game-mechanics',
    },
    { value: 'mobs', labelKey: 'comp.search.categories.mobs', mrSlug: 'mobs' },
    {
      value: 'optimization',
      labelKey: 'comp.search.categories.optimization',
      mrSlug: 'optimization',
    },
    { value: 'storage', labelKey: 'comp.search.categories.storage', mrSlug: 'storage' },
    {
      value: 'transportation',
      labelKey: 'comp.search.categories.transportation',
      mrSlug: 'transportation',
    },
    { value: 'worldgen', labelKey: 'comp.search.categories.worldgen', mrSlug: 'worldgen' },
  ],
  world: [
    { value: 'adventure', labelKey: 'comp.search.categories.adventure', cfId: 248 },
    { value: 'creative', labelKey: 'comp.search.categories.creative', cfId: 249 },
    { value: 'minigame', labelKey: 'comp.search.categories.minigame', cfId: 250 },
    { value: 'parkour', labelKey: 'comp.search.categories.parkour', cfId: 251 },
    { value: 'puzzle', labelKey: 'comp.search.categories.puzzle', cfId: 252 },
    { value: 'survival', labelKey: 'comp.search.categories.survival', cfId: 253 },
    { value: 'modded-world', labelKey: 'comp.search.categories.moddedWorld', cfId: 4464 },
  ],
}

/** Modrinth has no world project type, so worlds are CurseForge-only. */
export function isPlatformSupportedForType(compType: CompType, platform: CompPlatform): boolean {
  return !(compType === 'world' && platform === 'modrinth')
}

/** CurseForge has no follow-count sort. */
export function isSortSupportedOnPlatform(sort: CompSort, platform: CompPlatformFilter): boolean {
  return !(platform === 'curseforge' && sort === 'follows')
}

export function findCategoryDef(
  compType: CompType,
  categoryValue: string,
): CompCategoryDef | undefined {
  return COMP_CATEGORIES_BY_TYPE[compType].find((c) => c.value === categoryValue)
}

export function isCategorySupportedOnPlatform(
  compType: CompType,
  categoryValue: string,
  platform: CompPlatformFilter,
): boolean {
  const def = findCategoryDef(compType, categoryValue)
  if (!def) return false
  if (platform === 'modrinth') return def.mrSlug !== undefined
  if (platform === 'curseforge') return def.cfId !== undefined
  // "all" queries both backends and keeps whichever of them accepts the category.
  return true
}

/**
 * Translates the launcher-side category key into the id the target backend expects.
 * Unknown keys are passed through untouched so a hand-written query string still works.
 */
export function resolveCategoryForPlatform(
  compType: CompType,
  categoryValue: string | null,
  targetPlatform: CompPlatform,
): string | null {
  if (!categoryValue) return null
  const def = findCategoryDef(compType, categoryValue)
  if (!def) return categoryValue
  if (targetPlatform === 'modrinth') return def.mrSlug ?? null
  return def.cfId ? String(def.cfId) : null
}

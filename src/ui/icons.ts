export type LoaderKindKey =
  | 'vanilla'
  | 'fabric'
  | 'legacyfabric'
  | 'forge'
  | 'neoforge'
  | 'cleanroom'
  | 'optifine'
  | 'liteloader'
  | 'labymod'
  | 'quilt'
  | 'unknown'

export interface LoaderMetaItem {
  icon: string
  hue: number
  label: string
}

export const loaderMeta: Record<LoaderKindKey, LoaderMetaItem> = {
  vanilla: { icon: 'i-lucide-box', hue: 140, label: 'Vanilla' },
  fabric: { icon: 'i-lucide-waypoints', hue: 75, label: 'Fabric' },
  legacyfabric: { icon: 'i-lucide-waypoints', hue: 60, label: 'Legacy Fabric' },
  forge: { icon: 'i-lucide-anvil', hue: 40, label: 'Forge' },
  neoforge: { icon: 'i-lucide-flame', hue: 55, label: 'NeoForge' },
  cleanroom: { icon: 'i-lucide-sparkles', hue: 150, label: 'Cleanroom' },
  optifine: { icon: 'i-lucide-eye', hue: 25, label: 'OptiFine' },
  liteloader: { icon: 'i-lucide-feather', hue: 200, label: 'LiteLoader' },
  labymod: { icon: 'i-lucide-gem', hue: 260, label: 'LabyMod' },
  quilt: { icon: 'i-lucide-blocks', hue: 300, label: 'Quilt' },
  unknown: { icon: 'i-lucide-cpu', hue: 250, label: 'Unknown' },
}

export function getLoaderMeta(kind?: string | null): LoaderMetaItem {
  if (!kind) return loaderMeta.vanilla
  const norm = kind.toLowerCase().replace(/[\s_-]/g, '')
  if (norm in loaderMeta) {
    return loaderMeta[norm as LoaderKindKey]
  }
  return loaderMeta.unknown
}

export type BrandName =
  | 'microsoft'
  | 'modrinth'
  | 'curseforge'
  | 'github'
  | 'mojang'
  | 'minecraft'
  | 'xbox'

export const brandIconMap: Record<BrandName, string> = {
  microsoft: 'i-simple-icons-microsoft',
  modrinth: 'i-simple-icons-modrinth',
  curseforge: 'i-simple-icons-curseforge',
  github: 'i-simple-icons-github',
  mojang: 'i-simple-icons-mojangstudios',
  minecraft: 'i-simple-icons-minecraft',
  xbox: 'i-simple-icons-xbox',
}

export type AccountKindKey = 'microsoft' | 'offline' | 'authlib' | 'yggdrasilConnect'

export interface AccountKindMetaItem {
  brand?: BrandName
  icon: string
}

export const accountKindMeta: Record<AccountKindKey, AccountKindMetaItem> = {
  microsoft: { brand: 'microsoft', icon: 'i-simple-icons-microsoft' },
  offline: { icon: 'i-lucide-user-round' },
  authlib: { icon: 'i-lucide-shield-check' },
  yggdrasilConnect: { icon: 'i-lucide-key-round' },
}

export function getAccountKindMeta(kind?: string | null): AccountKindMetaItem {
  if (!kind) return accountKindMeta.offline
  const norm = kind.toLowerCase().replace(/[\s_-]/g, '')
  if (norm === 'microsoft' || norm === 'msa') return accountKindMeta.microsoft
  if (norm === 'authlib' || norm === 'authlibinjector' || norm === 'yggdrasil') {
    return accountKindMeta.authlib
  }
  if (norm === 'yggdrasilconnect' || norm === 'littleleskin') {
    return accountKindMeta.yggdrasilConnect
  }
  return accountKindMeta.offline
}

export type CompTypeKey = 'mod' | 'modpack' | 'resourcepack' | 'shader' | 'datapack' | 'world'

export const compTypeMeta: Record<CompTypeKey, { icon: string; hue: number }> = {
  mod: { icon: 'i-lucide-puzzle', hue: 250 },
  modpack: { icon: 'i-lucide-package', hue: 150 },
  resourcepack: { icon: 'i-lucide-palette', hue: 320 },
  shader: { icon: 'i-lucide-sparkles', hue: 55 },
  datapack: { icon: 'i-lucide-layers', hue: 190 },
  world: { icon: 'i-lucide-globe', hue: 130 },
}

export const platformMeta: Record<'modrinth' | 'curseforge', { brand: BrandName; icon: string; label: string }> = {
  modrinth: { brand: 'modrinth', icon: 'i-simple-icons-modrinth', label: 'Modrinth' },
  curseforge: { brand: 'curseforge', icon: 'i-simple-icons-curseforge', label: 'CurseForge' },
}

export const navIcons = {
  home: 'i-lucide-house',
  launch: 'i-lucide-play',
  instances: 'i-lucide-boxes',
  accounts: 'i-lucide-user-round',
  mods: 'i-lucide-puzzle',
  download: 'i-lucide-cloud-download',
  comp: 'i-lucide-store',
  tools: 'i-lucide-wrench',
  logs: 'i-lucide-terminal',
  tasks: 'i-lucide-list-checks',
  settings: 'i-lucide-settings',
  devUi: 'i-lucide-swatch-book',
} as const

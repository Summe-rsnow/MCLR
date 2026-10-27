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
  image?: string
}

export const loaderMeta: Record<LoaderKindKey, LoaderMetaItem> = {
  vanilla: {
    icon: 'i-lucide-box',
    hue: 140,
    label: 'Vanilla',
    image: '/assets/icons/blocks/grass_block.png',
  },
  fabric: {
    icon: 'i-lucide-waypoints',
    hue: 75,
    label: 'Fabric',
    image: '/assets/icons/loaders/fabric.png',
  },
  legacyfabric: {
    icon: 'i-lucide-waypoints',
    hue: 60,
    label: 'Legacy Fabric',
    image: '/assets/icons/loaders/fabric.png',
  },
  forge: {
    icon: 'i-lucide-anvil',
    hue: 40,
    label: 'Forge',
    image: '/assets/icons/loaders/forge.png',
  },
  neoforge: {
    icon: 'i-lucide-flame',
    hue: 55,
    label: 'NeoForge',
    image: '/assets/icons/loaders/neoforge.png',
  },
  cleanroom: {
    icon: 'i-lucide-sparkles',
    hue: 150,
    label: 'Cleanroom',
    image: '/assets/icons/loaders/cleanroom.png',
  },
  optifine: {
    icon: 'i-lucide-eye',
    hue: 25,
    label: 'OptiFine',
    image: '/assets/icons/loaders/optifine.png',
  },
  liteloader: { icon: 'i-lucide-feather', hue: 215, label: 'LiteLoader' },
  labymod: {
    icon: 'i-lucide-gem',
    hue: 260,
    label: 'LabyMod',
    image: '/assets/icons/loaders/labymod.png',
  },
  quilt: {
    icon: 'i-lucide-blocks',
    hue: 300,
    label: 'Quilt',
    image: '/assets/icons/loaders/quilt.png',
  },
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

export interface MinecraftIconMetaItem {
  key: string
  icon: string
  hue: number
  labelKey: string
  label: string
  image?: string
}

export const minecraftIconPresets: Record<string, MinecraftIconMetaItem> = {
  grass_block: {
    key: 'grass_block',
    icon: 'i-lucide-box',
    hue: 145,
    labelKey: 'instanceDetail.overview.iconGrass',
    label: '草方块',
    image: '/assets/icons/blocks/grass_block.png',
  },
  grass: {
    key: 'grass',
    icon: 'i-lucide-box',
    hue: 145,
    labelKey: 'instanceDetail.overview.iconGrass',
    label: '草方块',
    image: '/assets/icons/blocks/grass_block.png',
  },
  stone: {
    key: 'stone',
    icon: 'i-lucide-box',
    hue: 220,
    labelKey: 'instanceDetail.overview.iconStone',
    label: '石头',
    image: '/assets/icons/blocks/stone.png',
  },
  gold_block: {
    key: 'gold_block',
    icon: 'i-lucide-box',
    hue: 85,
    labelKey: 'instanceDetail.overview.iconGoldBlock',
    label: '金块',
    image: '/assets/icons/blocks/gold_block.png',
  },
  redstone_block: {
    key: 'redstone_block',
    icon: 'i-lucide-box',
    hue: 25,
    labelKey: 'instanceDetail.overview.iconRedstoneBlock',
    label: '红石块',
    image: '/assets/icons/blocks/redstone_block.png',
  },
  grass_path: {
    key: 'grass_path',
    icon: 'i-lucide-box',
    hue: 130,
    labelKey: 'instanceDetail.overview.iconGrassPath',
    label: '草径',
    image: '/assets/icons/blocks/grass_path.png',
  },
  egg: {
    key: 'egg',
    icon: 'i-lucide-egg',
    hue: 65,
    labelKey: 'instanceDetail.overview.iconEgg',
    label: '鸡蛋',
    image: '/assets/icons/blocks/egg.png',
  },
  redstone_lamp_on: {
    key: 'redstone_lamp_on',
    icon: 'i-lucide-lamp',
    hue: 80,
    labelKey: 'instanceDetail.overview.iconRedstoneLampOn',
    label: '红石灯（亮）',
    image: '/assets/icons/blocks/redstone_lamp_on.png',
  },
  redstone_lamp_off: {
    key: 'redstone_lamp_off',
    icon: 'i-lucide-lamp',
    hue: 45,
    labelKey: 'instanceDetail.overview.iconRedstoneLampOff',
    label: '红石灯（灭）',
    image: '/assets/icons/blocks/redstone_lamp_off.png',
  },
  crafting_table: {
    key: 'crafting_table',
    icon: 'i-lucide-hammer',
    hue: 75,
    labelKey: 'instanceDetail.overview.iconCraftingTable',
    label: '工作台',
  },
  chest: {
    key: 'chest',
    icon: 'i-lucide-archive',
    hue: 60,
    labelKey: 'instanceDetail.overview.iconChest',
    label: '箱子',
  },
  furnace: {
    key: 'furnace',
    icon: 'i-lucide-flame',
    hue: 40,
    labelKey: 'instanceDetail.overview.iconFurnace',
    label: '熔炉',
  },
  diamond: {
    key: 'diamond',
    icon: 'i-lucide-gem',
    hue: 200,
    labelKey: 'instanceDetail.overview.iconDiamond',
    label: '钻石',
  },
  sword: {
    key: 'sword',
    icon: 'i-lucide-sword',
    hue: 200,
    labelKey: 'instanceDetail.overview.iconSword',
    label: '剑',
  },
  pickaxe: {
    key: 'pickaxe',
    icon: 'i-lucide-pickaxe',
    hue: 240,
    labelKey: 'instanceDetail.overview.iconPickaxe',
    label: '镐',
  },
  book: {
    key: 'book',
    icon: 'i-lucide-book-open',
    hue: 50,
    labelKey: 'instanceDetail.overview.iconBook',
    label: '书架',
  },
  tnt: {
    key: 'tnt',
    icon: 'i-lucide-bomb',
    hue: 25,
    labelKey: 'instanceDetail.overview.iconTnt',
    label: 'TNT',
  },
  redstone: {
    key: 'redstone',
    icon: 'i-lucide-zap',
    hue: 25,
    labelKey: 'instanceDetail.overview.iconRedstone',
    label: '红石',
  },
  apple: {
    key: 'apple',
    icon: 'i-lucide-apple',
    hue: 90,
    labelKey: 'instanceDetail.overview.iconApple',
    label: '金苹果',
  },
  command_block: {
    key: 'command_block',
    icon: 'i-lucide-terminal',
    hue: 310,
    labelKey: 'instanceDetail.overview.iconCommandBlock',
    label: '命令方块',
    image: '/assets/icons/blocks/command_block.png',
  },
  ender_pearl: {
    key: 'ender_pearl',
    icon: 'i-lucide-circle-dot',
    hue: 180,
    labelKey: 'instanceDetail.overview.iconEnderPearl',
    label: '末影珍珠',
  },
}

export const minecraftIconList: MinecraftIconMetaItem[] = [
  minecraftIconPresets.grass_block,
  minecraftIconPresets.grass_path,
  minecraftIconPresets.stone,
  minecraftIconPresets.gold_block,
  minecraftIconPresets.redstone_block,
  minecraftIconPresets.command_block,
  minecraftIconPresets.egg,
  minecraftIconPresets.redstone_lamp_on,
  minecraftIconPresets.redstone_lamp_off,
]

export function getMinecraftIconMeta(icon?: string | null): MinecraftIconMetaItem | null {
  if (!icon) return null
  const norm = icon.toLowerCase().trim().replace(/[\s-]/g, '_')
  return minecraftIconPresets[norm] || minecraftIconPresets[icon.toLowerCase().trim()] || null
}

export function getInstanceIconMeta(icon?: string | null, kind?: string | null): LoaderMetaItem {
  if (icon) {
    const mc = getMinecraftIconMeta(icon)
    if (mc) {
      return {
        icon: mc.icon,
        hue: mc.hue,
        label: mc.label,
        image: mc.image,
      }
    }
    if (icon.startsWith('i-')) {
      return {
        icon,
        hue: 145,
        label: 'Custom Icon',
      }
    }
  }
  return getLoaderMeta(kind)
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

export const platformMeta: Record<
  'modrinth' | 'curseforge',
  { brand: BrandName; icon: string; label: string }
> = {
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

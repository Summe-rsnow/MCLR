import type {
  CompFile,
  CompProject,
  CompSearchResult,
  CompSearchQuery,
  CompPlatform,
} from '../../ipc/client'

const mockProjects: CompProject[] = [
  {
    platform: 'modrinth',
    id: 'AANobbMI',
    slug: 'sodium',
    title: 'Sodium',
    summary: 'A modern rendering engine for Minecraft that greatly improves frame rates and reduces stutter.',
    description: '# Sodium\n\n**Sodium** is a free and open-source rendering engine for Minecraft 1.16+ which greatly improves frame rates and reduces micro-stutter.\n\n### Features\n- Modern OpenGL rendering\n- Significantly reduced CPU overhead\n- Compatible with the Fabric ecosystem',
    iconUrl: 'https://cdn.modrinth.com/data/AANobbMI/icon.png',
    author: 'jellysquid3',
    downloads: 45000000,
    follows: 120000,
    categories: ['optimization'],
    mcVersions: ['1.20.1', '1.20.2', '1.20.4', '1.21'],
    loaders: ['fabric', 'neoforge'],
    compType: 'mod',
    updatedAt: '2024-06-15T12:00:00Z',
    websiteUrl: 'https://modrinth.com/mod/sodium',
  },
  {
    platform: 'modrinth',
    id: 'YL57xq9U',
    slug: 'iris',
    title: 'Iris Shaders',
    summary: 'A modern shaders mod for Minecraft compatible with existing Shaderspacks.',
    description: '# Iris Shaders\n\nIris is a modern shader mod for Minecraft written to be compatible with Sodium.',
    iconUrl: 'https://cdn.modrinth.com/data/YL57xq9U/icon.png',
    author: 'coderbot',
    downloads: 30000000,
    follows: 95000,
    categories: ['shaders', 'optimization'],
    mcVersions: ['1.20.1', '1.20.4', '1.21'],
    loaders: ['fabric', 'neoforge'],
    compType: 'mod',
    updatedAt: '2024-06-10T10:00:00Z',
    websiteUrl: 'https://modrinth.com/mod/iris',
  },
  {
    platform: 'curseforge',
    id: '238222',
    slug: 'jei',
    title: 'Just Enough Items (JEI)',
    summary: 'JEI is an item and recipe viewing mod for Minecraft.',
    description: '# Just Enough Items (JEI)\n\nJEI is an item and recipe viewing mod for Minecraft, built from the ground up for stability and performance.',
    iconUrl: 'https://media.forgecdn.net/avatars/28/62/635838954801524319.png',
    author: 'mezz',
    downloads: 250000000,
    follows: 350000,
    categories: ['utility', 'mc-addons'],
    mcVersions: ['1.20.1', '1.20.2', '1.20.4', '1.21'],
    loaders: ['forge', 'neoforge', 'fabric'],
    compType: 'mod',
    updatedAt: '2024-06-01T08:00:00Z',
    websiteUrl: 'https://www.curseforge.com/minecraft/mc-mods/jei',
  },
  {
    platform: 'modrinth',
    id: '1KVoqpyW',
    slug: 'complementary-reimagined',
    title: 'Complementary Reimagined',
    summary: 'A shaderpack that aims for perfection while preserving the Minecraft visual identity.',
    description: '# Complementary Reimagined\n\nDetailed lighting, water effects, and high performance.',
    iconUrl: 'https://cdn.modrinth.com/data/1KVoqpyW/icon.png',
    author: 'EminGT',
    downloads: 8000000,
    follows: 45000,
    categories: ['shaderpacks'],
    mcVersions: ['1.20.1', '1.20.4', '1.21'],
    loaders: ['iris', 'optifine'],
    compType: 'shader',
    updatedAt: '2024-05-20T14:00:00Z',
    websiteUrl: 'https://modrinth.com/shader/complementary-reimagined',
  },
  {
    platform: 'curseforge',
    id: '685172',
    slug: 'cobblemon-modpack',
    title: 'Cobblemon Official Modpack',
    summary: 'The official modpack for Cobblemon, featuring optimization and QoL mods.',
    description: '# Cobblemon Official Modpack\n\nCatch Pokémon in Minecraft with open-world exploration.',
    iconUrl: 'https://media.forgecdn.net/avatars/655/444/638045612345678901.png',
    author: 'CobblemonTeam',
    downloads: 5000000,
    follows: 28000,
    categories: ['adventure', 'quests'],
    mcVersions: ['1.20.1'],
    loaders: ['fabric'],
    compType: 'modpack',
    updatedAt: '2024-05-18T16:00:00Z',
    websiteUrl: 'https://www.curseforge.com/minecraft/modpacks/cobblemon-fabric',
  },
]

const mockFiles: Record<string, CompFile[]> = {
  AANobbMI: [
    {
      id: 'file-sodium-1',
      projectId: 'AANobbMI',
      platform: 'modrinth',
      name: 'Sodium 0.5.8 for 1.20.1',
      fileName: 'sodium-fabric-0.5.8+mc1.20.1.jar',
      mcVersions: ['1.20.1'],
      loaders: ['fabric'],
      channel: 'release',
      releaseDate: '2024-04-12T10:00:00Z',
      size: 1450230,
      hashes: { sha1: '11223344556677889900aabbccddeeff00112233' },
      downloadUrls: ['https://cdn.modrinth.com/data/AANobbMI/versions/file.jar'],
      distributionDisabled: false,
      dependencies: [],
    },
    {
      id: 'file-sodium-2',
      projectId: 'AANobbMI',
      platform: 'modrinth',
      name: 'Sodium 0.5.11 for 1.20.4',
      fileName: 'sodium-fabric-0.5.11+mc1.20.4.jar',
      mcVersions: ['1.20.4'],
      loaders: ['fabric'],
      channel: 'beta',
      releaseDate: '2024-05-15T12:00:00Z',
      size: 1489200,
      hashes: { sha1: 'aabbccddeeff00112233445566778899aabbccdd' },
      downloadUrls: ['https://cdn.modrinth.com/data/AANobbMI/versions/file2.jar'],
      distributionDisabled: false,
      dependencies: [],
    },
  ],
  '238222': [
    {
      id: 'file-jei-1',
      projectId: '238222',
      platform: 'curseforge',
      name: 'jei-1.20.1-forge-15.3.0.4.jar',
      fileName: 'jei-1.20.1-forge-15.3.0.4.jar',
      mcVersions: ['1.20.1'],
      loaders: ['forge'],
      channel: 'release',
      releaseDate: '2024-03-20T09:00:00Z',
      size: 1250000,
      hashes: { sha1: '5566778899aabbccddeeff001122334455667788' },
      downloadUrls: ['https://edge.forgecdn.net/files/5123/456/jei.jar'],
      distributionDisabled: false,
      dependencies: [],
    },
    {
      id: 'file-jei-disabled',
      projectId: '238222',
      platform: 'curseforge',
      name: 'jei-legacy-restricted.jar',
      fileName: 'jei-legacy-restricted.jar',
      mcVersions: ['1.16.5'],
      loaders: ['forge'],
      channel: 'release',
      releaseDate: '2021-01-01T00:00:00Z',
      size: 980000,
      hashes: { sha1: '99887766554433221100ffeeddccbbaa99887766' },
      downloadUrls: [],
      distributionDisabled: true,
      dependencies: [],
    },
  ],
}

export default {
  comp_search: (args: Record<string, unknown>): CompSearchResult => {
    const query = (args.query as CompSearchQuery) || {}
    let list = [...mockProjects]

    if (query.platform) {
      list = list.filter((p) => p.platform === query.platform)
    }
    if (query.compType) {
      list = list.filter((p) => p.compType === query.compType)
    }
    if (query.query && query.query.trim()) {
      const q = query.query.trim().toLowerCase()
      list = list.filter(
        (p) =>
          p.title.toLowerCase().includes(q) ||
          p.summary.toLowerCase().includes(q) ||
          p.slug.toLowerCase().includes(q),
      )
    }
    if (query.mcVersion) {
      list = list.filter((p) => p.mcVersions.includes(query.mcVersion!))
    }
    if (query.loader) {
      list = list.filter((p) => p.loaders.includes(query.loader!))
    }
    if (query.category) {
      list = list.filter((p) => p.categories.includes(query.category!))
    }

    const offset = query.offset || 0
    const limit = query.limit || 20
    const hits = list.slice(offset, offset + limit)

    return {
      hits,
      totalHits: list.length,
      offset,
      limit,
    }
  },

  comp_project: (args: Record<string, unknown>): CompProject => {
    const platform = args.platform as CompPlatform
    const projectId = String(args.projectId || '')
    const project = mockProjects.find(
      (p) => p.platform === platform && (p.id === projectId || p.slug === projectId),
    )
    if (!project) {
      throw { code: 'comp.projectFailed', message: 'Project not found' }
    }
    return project
  },

  comp_files: (args: Record<string, unknown>): CompFile[] => {
    const projectId = String(args.projectId || '')
    const mcVersion = args.mcVersion ? String(args.mcVersion) : null
    const loader = args.loader ? String(args.loader) : null

    let files = mockFiles[projectId] || [
      {
        id: `mock-file-${projectId}-1`,
        projectId,
        platform: (args.platform as CompPlatform) || 'modrinth',
        name: `Release File for ${projectId}`,
        fileName: `${projectId}-1.0.0.jar`,
        mcVersions: ['1.20.1'],
        loaders: ['fabric', 'forge'],
        channel: 'release',
        releaseDate: '2024-05-01T00:00:00Z',
        size: 2048500,
        hashes: { sha1: '1234567890abcdef1234567890abcdef12345678' },
        downloadUrls: ['https://example.com/download.jar'],
        distributionDisabled: false,
        dependencies: [],
      },
    ]

    if (mcVersion) {
      files = files.filter((f) => f.mcVersions.includes(mcVersion))
    }
    if (loader) {
      files = files.filter((f) => f.loaders.includes(loader))
    }
    return files
  },

  comp_download: (_args: Record<string, unknown>): string => {
    return 'task-comp-download-' + Math.random().toString(36).substring(2, 9)
  },
}

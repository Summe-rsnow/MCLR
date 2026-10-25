import type {
  ResourceEntry,
  ResourceKind,
  SchematicInfo,
  ImportResourcesResult,
  JarInJarNode,
} from '../../ipc/client'

export const mockModEntries: ResourceEntry[] = [
  {
    path: 'C:\\mc\\versions\\1.20.4-fabric\\mods\\fabric-api-0.96.0+1.20.4.jar',
    relPath: 'fabric-api-0.96.0+1.20.4.jar',
    fileName: 'fabric-api-0.96.0+1.20.4.jar',
    isDir: false,
    size: 2145320,
    createdMs: 1700000000000,
    modifiedMs: 1700000000000,
    enabled: true,
    state: 'ok',
    title: 'Fabric API',
    description: 'Core API library for the Fabric toolchain',
    version: '0.96.0+1.20.4',
    icon: null,
    tags: ['Fabric'],
    modId: 'fabric-api',
    projectId: 'P7dR8mSH',
    latestFile: null,
    updateAvailable: false,
    dependents: ['sodium-fabric-0.5.8+mc1.20.4.jar'],
  },
  {
    path: 'C:\\mc\\versions\\1.20.4-fabric\\mods\\sodium-fabric-0.5.8+mc1.20.4.jar',
    relPath: 'sodium-fabric-0.5.8+mc1.20.4.jar',
    fileName: 'sodium-fabric-0.5.8+mc1.20.4.jar',
    isDir: false,
    size: 1048576,
    createdMs: 1700001000000,
    modifiedMs: 1700001000000,
    enabled: true,
    state: 'ok',
    title: 'Sodium',
    description: 'Modern rendering engine and client-side optimization mod',
    version: '0.5.8',
    icon: null,
    tags: ['Fabric'],
    modId: 'sodium',
    projectId: 'AANobbMI',
    latestFile: null,
    updateAvailable: false,
    dependents: [],
  },
]

export const mockResourcePacks: ResourceEntry[] = [
  {
    path: 'C:\\mc\\versions\\1.20.4-fabric\\resourcepacks\\Faithful-32x.zip',
    relPath: 'Faithful-32x.zip',
    fileName: 'Faithful-32x.zip',
    isDir: false,
    size: 8388608,
    createdMs: 1700002000000,
    modifiedMs: 1700002000000,
    enabled: true,
    state: 'ok',
    title: 'Faithful 32x',
    description: 'Double resolution default textures',
    version: null,
    icon: null,
    tags: ['ResourcePack'],
    modId: null,
    projectId: null,
    latestFile: null,
    updateAvailable: false,
    dependents: [],
  },
]

const handlers = {
  assets_allow: () => null,
  resources_list: (args: Record<string, unknown>): ResourceEntry[] => {
    const kind = String(args.kind || 'mod') as ResourceKind
    if (kind === 'mod') return [...mockModEntries]
    if (kind === 'resourcePack') return [...mockResourcePacks]
    return []
  },
  local_mods_scan_start: (): { entries: ResourceEntry[] } => ({
    entries: [...mockModEntries],
  }),
  resource_toggle: (args: Record<string, unknown>) =>
    String(args?.path || '').endsWith('.disabled')
      ? String(args?.path || '').replace(/\.disabled$/, '')
      : `${String(args?.path || '')}.disabled`,
  resources_delete: () => null,
  resources_import: (): ImportResourcesResult => ({ imported: [], conflicts: [] }),
  resources_export: () => 'name,file,version\nSodium,sodium.jar,0.5.8',
  schematic_inspect: (): SchematicInfo => ({
    name: 'Castle',
    author: 'Steve',
    size: [32, 24, 32],
    totalVolume: 24576,
    totalBlocks: 8192,
    regionCount: 1,
    createdMs: 1700000000000,
    modifiedMs: 1700000000000,
  }),
  local_mods_dependents: () => [],
  local_mods_update_many: () => 'task-mod-update-1',
  local_mod_jar_in_jar: (): JarInJarNode[] => [],
}

export default handlers

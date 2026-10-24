import type { InstanceInfo, JavaInstallation } from '../../ipc/client'

export const mockInstances: InstanceInfo[] = [
  {
    id: '1.20.1-vanilla',
    name: 'Vanilla 1.20.1',
    folder: 'versions/1.20.1-vanilla',
    path: 'C:\\Users\\User\\AppData\\Roaming\\MCLR\\.minecraft\\versions\\1.20.1-vanilla',
    mcVersion: '1.20.1',
    kind: 'vanilla',
    loaders: [],
    state: 'ok',
    config: {
      icon: 'grass_block',
      category: 'Vanilla',
      pinned: true,
      isolation: 'on',
      java: null,
      memory: 4096,
      jvmArgs: null,
      gameArgs: null,
      windowTitle: null,
      preLaunchCommand: null,
      loginRequirement: 'any',
      serverQuickPlay: null,
    },
  },
  {
    id: '1.20.4-fabric',
    name: 'Fabric 1.20.4',
    folder: 'versions/1.20.4-fabric',
    path: 'C:\\Users\\User\\AppData\\Roaming\\MCLR\\.minecraft\\versions\\1.20.4-fabric',
    mcVersion: '1.20.4',
    kind: 'fabric',
    loaders: ['Fabric 0.15.11'],
    state: 'ok',
    config: {
      icon: 'fabric',
      category: 'Modded',
      pinned: true,
      isolation: 'on',
      java: null,
      memory: 6144,
      jvmArgs: null,
      gameArgs: null,
      windowTitle: null,
      preLaunchCommand: null,
      loginRequirement: 'any',
      serverQuickPlay: null,
    },
  },
  {
    id: '1.21.1-neoforge',
    name: 'NeoForge 1.21.1',
    folder: 'versions/1.21.1-neoforge',
    path: 'C:\\Users\\User\\AppData\\Roaming\\MCLR\\.minecraft\\versions\\1.21.1-neoforge',
    mcVersion: '1.21.1',
    kind: 'neoforge',
    loaders: ['NeoForge 21.1.65'],
    state: 'ok',
    config: {
      icon: 'anvil',
      category: 'Modded',
      pinned: false,
      isolation: 'on',
      java: null,
      memory: 8192,
      jvmArgs: null,
      gameArgs: null,
      windowTitle: null,
      preLaunchCommand: null,
      loginRequirement: 'microsoft',
      serverQuickPlay: null,
    },
  },
  {
    id: '1.12.2-forge',
    name: 'Forge 1.12.2',
    folder: 'versions/1.12.2-forge',
    path: 'C:\\Users\\User\\AppData\\Roaming\\MCLR\\.minecraft\\versions\\1.12.2-forge',
    mcVersion: '1.12.2',
    kind: 'forge',
    loaders: ['Forge 14.23.5.2860', 'OptiFine HD_U_G5'],
    state: 'ok',
    config: {
      icon: 'furnace',
      category: 'Legacy',
      pinned: false,
      isolation: 'global',
      java: null,
      memory: 4096,
      jvmArgs: null,
      gameArgs: null,
      windowTitle: null,
      preLaunchCommand: null,
      loginRequirement: 'any',
      serverQuickPlay: null,
    },
  },
  {
    id: 'broken-instance',
    name: 'Incomplete Snapshot',
    folder: 'versions/broken-instance',
    path: 'C:\\Users\\User\\AppData\\Roaming\\MCLR\\.minecraft\\versions\\broken-instance',
    mcVersion: '24w40a',
    kind: 'snapshot',
    loaders: [],
    state: 'broken',
    config: {
      icon: 'barrier',
      category: 'Test',
      pinned: false,
      isolation: 'on',
      java: null,
      memory: null,
      jvmArgs: null,
      gameArgs: null,
      windowTitle: null,
      preLaunchCommand: null,
      loginRequirement: 'any',
      serverQuickPlay: null,
    },
  },
]

export const mockJavaInstallations: JavaInstallation[] = [
  {
    path: 'C:\\Program Files\\Java\\jdk-25\\bin\\javaw.exe',
    home: 'C:\\Program Files\\Java\\jdk-25',
    version: '25.0.3',
    major: 25,
    implementor: 'Oracle Corporation',
    arch: 'x64',
    isJdk: true,
    source: 'Registry',
  },
  {
    path: 'C:\\Program Files\\Eclipse Adoptium\\jdk-17.0.10.7-hotspot\\bin\\javaw.exe',
    home: 'C:\\Program Files\\Eclipse Adoptium\\jdk-17.0.10.7-hotspot',
    version: '17.0.10',
    major: 17,
    implementor: 'Eclipse Adoptium',
    arch: 'x64',
    isJdk: true,
    source: 'Directory',
  },
  {
    path: 'C:\\Program Files\\Zulu\\zulu-8\\bin\\javaw.exe',
    home: 'C:\\Program Files\\Zulu\\zulu-8',
    version: '1.8.0_402',
    major: 8,
    implementor: 'Azul Systems',
    arch: 'x64',
    isJdk: false,
    source: 'Registry',
  },
]

let instancesData = JSON.parse(JSON.stringify(mockInstances)) as InstanceInfo[]

export function resetMockInstances(): void {
  instancesData = JSON.parse(JSON.stringify(mockInstances)) as InstanceInfo[]
}
const handlers = {
  instances_list: () => [...instancesData],
  instance_rename: (args: Record<string, unknown>) => {
    const id = String(args.id)
    const newName = String(args.newName)
    const inst = instancesData.find((i) => i.id === id)
    if (!inst) throw { code: 'game.instanceNotFound', params: { id } }
    inst.name = newName
    return { ...inst }
  },
  instance_duplicate: (args: Record<string, unknown>) => {
    const id = String(args.id)
    const newName = String(args.newName)
    const inst = instancesData.find((i) => i.id === id)
    if (!inst) throw { code: 'game.instanceNotFound', params: { id } }
    const copy: InstanceInfo = {
      ...inst,
      id: `${id}-copy`,
      name: newName,
      folder: `versions/${id}-copy`,
      path: `${inst.path}-copy`,
      config: { ...inst.config, pinned: false },
    }
    instancesData.push(copy)
    return { ...copy }
  },
  instance_delete: (args: Record<string, unknown>) => {
    const id = String(args.id)
    instancesData = instancesData.filter((i) => i.id !== id)
    return null
  },
  instance_update_config: (args: Record<string, unknown>) => {
    const id = String(args.id)
    const config = (args.config ?? {}) as InstanceInfo['config']
    const inst = instancesData.find((i) => i.id === id)
    if (!inst) throw { code: 'game.instanceNotFound', params: { id } }
    inst.config = { ...inst.config, ...config }
    return { ...inst }
  },
  instance_open_folder: () => null,
  java_scan: () => [...mockJavaInstallations],
  java_download: (args: Record<string, unknown>) => `task-java-${String(args.component)}`,
}

export default handlers

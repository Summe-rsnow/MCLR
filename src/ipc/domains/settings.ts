import { invokeCommand } from '../client'
import type { CompSettings } from './comp'

export interface JavaInstallation {
  path: string
  home: string
  version: string
  major: number
  implementor: string
  arch: string
  isJdk: boolean
  source: string
}

export interface JavaConstraint {
  minMajor?: number | null
  maxMajor?: number | null
  exactMajors: number[]
}

export interface GameFolderEntry {
  name: string
  path: string
}

export interface GameSettings {
  folders: GameFolderEntry[]
  currentFolder: string
  defaultIsolation: string
  sortBy: string
}

export interface LaunchSettings {
  ramMode: 'auto' | 'custom'
  ramCustomMb: number
  jvmArgs: string
  gameArgs: string
  windowTitle: string
  fullscreen: boolean
  width: number
  height: number
  preferredIpStack: 'auto' | 'ipv4' | 'ipv6'
  processPriority: 'normal' | 'aboveNormal' | 'high'
  closeLauncherOnStart: 'keep' | 'hide' | 'exit'
  disableCrashAnalysis: boolean
}

export interface JavaSettings {
  custom: string[]
  select: string
  preferJdk: boolean
}

export interface UpdateSettings {
  channel: string
  autoCheck: boolean
  announcementUrl: string
}

export interface StatesSettings {
  dismissedAnnouncements: string[]
}

export type ProxyMode = 'none' | 'system' | 'custom'
export type SourcePriority = 'mirrorFirst' | 'smart' | 'officialFirst'

export interface NetSettings {
  proxyMode: ProxyMode
  proxyUrl: string
  doh: boolean
  sourcePriority: SourcePriority
  connectionsPerFile: number
  maxConcurrentFiles: number
}

export interface Settings {
  schemaVersion: number
  game?: GameSettings
  launch?: LaunchSettings
  java?: JavaSettings
  ui: {
    seed: string
    mode: 'light' | 'dark' | 'system'
    material: 'flat' | 'glass'
    scale: number
    fontFamily: string
    backgroundMode: string
    backgroundPath: string
    backgroundBlur: number
    backgroundOpacity: number
    musicEnabled: boolean
    musicFolder: string
    musicVolume: number
  }
  language: {
    locale: string
    formatCulture: string
  }
  system: {
    logLevel: string
    logRetentionDays: number
    disableGpu: boolean
  }
  net?: NetSettings
  comp?: CompSettings
  update?: UpdateSettings
  states?: StatesSettings
}

export const settingsIpc = {
  settingsGet: (): Promise<Settings> => invokeCommand<Settings>('settings_get'),
  settingsPatch: (patch: Record<string, unknown>): Promise<Settings> =>
    invokeCommand<Settings>('settings_patch', { patch }),
  windowSetMaterial: (glass: boolean): Promise<void> =>
    invokeCommand<void>('window_set_material', { glass }),
  javaDownload: (component: string): Promise<string> =>
    invokeCommand<string>('java_download', { component }),
}

import { defineStore } from 'pinia'
import { ref } from 'vue'
import {
  applyTheme,
  type ThemeMode,
  type MaterialMode,
  type ThemeConfig,
} from '../ui/theme'
import { ipcClient } from '../ipc/client'

export type BackgroundMode = 'none' | 'image' | 'slideshow' | 'video'
export type MusicPlayMode = 'loop' | 'shuffle'

export const useThemeStore = defineStore('theme', () => {
  // Theme & Material
  const seed = ref<string>('#3b82f6')
  const mode = ref<ThemeMode>('system')
  const material = ref<MaterialMode>('flat')

  // Typography & Scale
  const scale = ref<number>(1.0)
  const fontFamily = ref<string>('')

  // Background
  const backgroundMode = ref<BackgroundMode>('none')
  const backgroundPath = ref<string>('')
  const backgroundBlur = ref<number>(0)
  const backgroundOpacity = ref<number>(0.3)

  // Music
  const musicEnabled = ref<boolean>(false)
  const musicFolder = ref<string>('')
  const musicVolume = ref<number>(50)
  const musicPlayMode = ref<MusicPlayMode>('loop')

  // Playback state (ephemeral, not persisted)
  const isPlaying = ref<boolean>(false)
  const currentTrackIndex = ref<number>(0)
  const playlist = ref<string[]>([])
  const pausedByGame = ref<boolean>(false)
  let saveTimer: ReturnType<typeof setTimeout> | undefined = undefined
  function schedulePersist() {
    clearTimeout(saveTimer)
    saveTimer = setTimeout(async () => {
      try {
        await ipcClient.settingsPatch({
          ui: {
            seed: seed.value,
            mode: mode.value,
            material: material.value,
            scale: scale.value,
            fontFamily: fontFamily.value,
            backgroundMode: backgroundMode.value,
            backgroundPath: backgroundPath.value,
            backgroundBlur: backgroundBlur.value,
            backgroundOpacity: backgroundOpacity.value,
            musicEnabled: musicEnabled.value,
            musicFolder: musicFolder.value,
            musicVolume: musicVolume.value,
            musicPlayMode: musicPlayMode.value,
          },
        })
      } catch {
        // Ignore in test/headless environment
      }
    }, 200)
  }

  function setSeed(newSeed: string) {
    seed.value = newSeed
    apply()
    schedulePersist()
  }

  function setMode(newMode: ThemeMode) {
    mode.value = newMode
    apply()
    schedulePersist()
  }

  async function setMaterial(newMaterial: MaterialMode) {
    material.value = newMaterial
    apply()
    schedulePersist()
    try {
      await ipcClient.windowSetMaterial(newMaterial === 'glass')
    } catch {
      // Ignore in non-Tauri browser/test environment
    }
  }

  function setScale(newScale: number) {
    scale.value = Math.max(0.8, Math.min(1.25, newScale))
    applyTypography()
    schedulePersist()
  }

  function setFontFamily(newFont: string) {
    fontFamily.value = newFont
    applyTypography()
    schedulePersist()
  }

  function setBackgroundMode(newMode: BackgroundMode) {
    backgroundMode.value = newMode
    schedulePersist()
  }

  function setBackgroundPath(newPath: string) {
    backgroundPath.value = newPath
    schedulePersist()
  }

  function setBackgroundBlur(newBlur: number) {
    backgroundBlur.value = Math.max(0, Math.min(32, newBlur))
    schedulePersist()
  }

  function setBackgroundOpacity(newOpacity: number) {
    backgroundOpacity.value = Math.max(0, Math.min(1, newOpacity))
    schedulePersist()
  }

  function setMusicEnabled(enabled: boolean) {
    musicEnabled.value = enabled
    if (!enabled) {
      isPlaying.value = false
    }
    schedulePersist()
  }

  function setMusicFolder(folder: string) {
    musicFolder.value = folder
    schedulePersist()
  }

  function setMusicVolume(vol: number) {
    musicVolume.value = Math.max(0, Math.min(100, Math.round(vol)))
    schedulePersist()
  }

  function setMusicPlayMode(playMode: MusicPlayMode) {
    musicPlayMode.value = playMode
    schedulePersist()
  }

  function applyTypography() {
    if (typeof document !== 'undefined' && document.documentElement) {
      if (fontFamily.value) {
        document.documentElement.style.fontFamily = fontFamily.value
      } else {
        document.documentElement.style.fontFamily = ''
      }
      document.documentElement.style.fontSize = `${scale.value * 16}px`
    }
  }

  function apply() {
    const config: ThemeConfig = {
      seed: seed.value,
      mode: mode.value,
      material: material.value,
    }
    applyTheme(config)
    applyTypography()
  }

  async function init() {
    try {
      const s = await ipcClient.settingsGet()
      if (s && s.ui) {
        if (s.ui.seed) seed.value = s.ui.seed
        if (s.ui.mode) mode.value = s.ui.mode
        if (s.ui.material) material.value = s.ui.material
        if (typeof s.ui.scale === 'number') scale.value = s.ui.scale
        if (typeof s.ui.fontFamily === 'string') fontFamily.value = s.ui.fontFamily
        if (s.ui.backgroundMode) backgroundMode.value = s.ui.backgroundMode as BackgroundMode
        if (typeof s.ui.backgroundPath === 'string') backgroundPath.value = s.ui.backgroundPath
        if (typeof s.ui.backgroundBlur === 'number') backgroundBlur.value = s.ui.backgroundBlur
        if (typeof s.ui.backgroundOpacity === 'number') backgroundOpacity.value = s.ui.backgroundOpacity
        if (typeof s.ui.musicEnabled === 'boolean') musicEnabled.value = s.ui.musicEnabled
        if (typeof s.ui.musicFolder === 'string') musicFolder.value = s.ui.musicFolder
        if (typeof s.ui.musicVolume === 'number') musicVolume.value = s.ui.musicVolume
        const uiRecord = s.ui as Record<string, unknown>
        if (typeof uiRecord.musicPlayMode === 'string') {
          musicPlayMode.value = uiRecord.musicPlayMode as MusicPlayMode
        }
      }
    } catch {
      // In tests/mock
    }

    apply()
    if (typeof window !== 'undefined' && window.matchMedia) {
      window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
        if (mode.value === 'system') {
          apply()
        }
      })
    }
  }

  return {
    seed,
    mode,
    material,
    scale,
    fontFamily,
    backgroundMode,
    backgroundPath,
    backgroundBlur,
    backgroundOpacity,
    musicEnabled,
    musicFolder,
    musicVolume,
    musicPlayMode,
    isPlaying,
    currentTrackIndex,
    playlist,
    pausedByGame,
    setSeed,
    setMode,
    setMaterial,
    setScale,
    setFontFamily,
    setBackgroundMode,
    setBackgroundPath,
    setBackgroundBlur,
    setBackgroundOpacity,
    setMusicEnabled,
    setMusicFolder,
    setMusicVolume,
    setMusicPlayMode,
    applyTypography,
    apply,
    init,
  }
})

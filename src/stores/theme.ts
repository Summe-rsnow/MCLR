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
export type MotionMode = 'full' | 'reduced' | 'off'
const MOTION_KEY = 'mclr.motion'

export const useThemeStore = defineStore('theme', () => {
  // Theme & Material
  const seed = ref<string>('#3b82f6')
  const mode = ref<ThemeMode>('system')
  const material = ref<MaterialMode>('flat')
  const motion = ref<MotionMode>(initialMotion())

  function initialMotion(): MotionMode {
    try {
      const saved = typeof localStorage !== 'undefined' ? localStorage.getItem(MOTION_KEY) : null
      if (saved === 'full' || saved === 'reduced' || saved === 'off') return saved
    } catch {
      // ignore
    }
    if (
      typeof window !== 'undefined' &&
      window.matchMedia?.('(prefers-reduced-motion: reduce)').matches
    ) {
      return 'reduced'
    }
    return 'full'
  }

  // Scale
  const scale = ref<number>(1.0)
  // Background
  const backgroundMode = ref<BackgroundMode>('none')
  const backgroundPath = ref<string>('')
  const backgroundBlur = ref<number>(0)
  const backgroundOpacity = ref<number>(0.3)
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
            backgroundMode: backgroundMode.value,
            backgroundPath: backgroundPath.value,
            backgroundBlur: backgroundBlur.value,
            backgroundOpacity: backgroundOpacity.value,
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

  function applyTypography() {
    if (typeof document !== 'undefined' && document.documentElement) {
      document.documentElement.style.fontSize = `${scale.value * 16}px`
    }
  }

  function setMotion(newMotion: MotionMode) {
    motion.value = newMotion
    applyMotion()
    try {
      localStorage.setItem(MOTION_KEY, newMotion)
    } catch {
      // ignore
    }
  }

  function applyMotion() {
    if (typeof document !== 'undefined' && document.documentElement) {
      document.documentElement.dataset.motion = motion.value
    }
  }

  function apply() {
    applyMotion()
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
        if (s.ui.backgroundMode) backgroundMode.value = s.ui.backgroundMode as BackgroundMode
        if (typeof s.ui.backgroundPath === 'string') backgroundPath.value = s.ui.backgroundPath
        if (typeof s.ui.backgroundBlur === 'number') backgroundBlur.value = s.ui.backgroundBlur
        if (typeof s.ui.backgroundOpacity === 'number') backgroundOpacity.value = s.ui.backgroundOpacity
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
    motion,
    scale,
    backgroundMode,
    backgroundPath,
    backgroundBlur,
    backgroundOpacity,
    setSeed,
    setMode,
    setMaterial,
    setMotion,
    setScale,
    setBackgroundMode,
    setBackgroundPath,
    setBackgroundBlur,
    setBackgroundOpacity,
    applyTypography,
    apply,
    init,
  }
})

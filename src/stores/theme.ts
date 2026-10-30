import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import {
  applyTheme,
  scheduleThemeFrame,
  type ThemeMode,
  type MaterialMode,
  type ThemeConfig,
} from '../ui/theme'
import { ipcClient } from '../ipc/client'

export type BackgroundMode = 'none' | 'image'
export type MotionMode = 'full' | 'reduced' | 'off'
/**
 * One ordinal knob over the three cost knobs (material / motion / splash),
 * cheapest first. The array order is the slider order.
 */
export type PerformanceTier = 'performance' | 'balanced' | 'quality'
export const PERFORMANCE_TIERS: readonly PerformanceTier[] = ['performance', 'balanced', 'quality']
const MOTION_KEY = 'mclr.motion'

export const useThemeStore = defineStore('theme', () => {
  // Theme & Material
  const seed = ref<string>('#3b82f6')
  const mode = ref<ThemeMode>('system')
  const material = ref<MaterialMode>('flat')
  const motion = ref<MotionMode>(initialMotion())
  // Startup splash
  const startupSplash = ref<boolean>(
    typeof localStorage !== 'undefined' ? localStorage.getItem('mclr.splash') !== 'off' : true,
  )

  function setStartupSplash(v: boolean) {
    startupSplash.value = v
    try {
      if (typeof localStorage !== 'undefined') {
        localStorage.setItem('mclr.splash', v ? 'on' : 'off')
      }
    } catch {
      // Ignore localStorage errors
    }
  }

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

  let typographyScheduled = false

  // Theme variables and typography must never be observed half-applied, so the
  // deferred write joins the pending theme frame (at most one per frame).
  function scheduleTypography() {
    if (typographyScheduled) return
    typographyScheduled = true
    scheduleThemeFrame(() => {
      typographyScheduled = false
      applyTypography()
    })
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

  // Glass is the expensive knob, so it wins the tier; without it the motion
  // budget decides between "performance" (no animation) and "balanced".
  // Other writers (quick-settings material toggle, restored settings) therefore
  // still land on a meaningful tier instead of an unrepresentable combination.
  const performanceTier = computed<PerformanceTier>(() => {
    if (material.value === 'glass') return 'quality'
    return motion.value === 'off' ? 'performance' : 'balanced'
  })

  function setPerformanceTier(tier: PerformanceTier) {
    if (tier === 'quality') {
      void setMaterial('glass')
      setMotion('full')
      setStartupSplash(true)
    } else if (tier === 'balanced') {
      void setMaterial('flat')
      setMotion('reduced')
      setStartupSplash(true)
    } else {
      void setMaterial('flat')
      setMotion('off')
      setStartupSplash(false)
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
    scheduleTypography()
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
        if (typeof s.ui.backgroundOpacity === 'number')
          backgroundOpacity.value = s.ui.backgroundOpacity
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
    performanceTier,
    setPerformanceTier,
    startupSplash,
    setStartupSplash,
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

import { defineStore } from 'pinia'
import { ref } from 'vue'
import {
  applyTheme,
  type ThemeMode,
  type MaterialMode,
  type ThemeConfig,
} from '../ui/theme'
import { ipcClient } from '../ipc/client'

export const useThemeStore = defineStore('theme', () => {
  const seed = ref<string>('#3b82f6')
  const mode = ref<ThemeMode>('system')
  const material = ref<MaterialMode>('flat')

  function setSeed(newSeed: string) {
    seed.value = newSeed
    apply()
  }

  function setMode(newMode: ThemeMode) {
    mode.value = newMode
    apply()
  }

  async function setMaterial(newMaterial: MaterialMode) {
    material.value = newMaterial
    apply()
    try {
      await ipcClient.windowSetMaterial(newMaterial === 'glass')
    } catch {
      // Ignore in non-Tauri browser/test environment
    }
  }

  function apply() {
    const config: ThemeConfig = {
      seed: seed.value,
      mode: mode.value,
      material: material.value,
    }
    applyTheme(config)
  }

  function init() {
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
    setSeed,
    setMode,
    setMaterial,
    apply,
    init,
  }
})

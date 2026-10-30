import { computed } from 'vue'
import { convertFileSrc } from '@tauri-apps/api/core'
import { useThemeStore } from '../../stores/theme'

export function resolveMediaSrc(filePath: string): string {
  if (!filePath) return ''
  const trimmed = filePath.trim()
  if (
    trimmed.startsWith('http://') ||
    trimmed.startsWith('https://') ||
    trimmed.startsWith('data:')
  ) {
    return trimmed
  }
  try {
    return convertFileSrc(trimmed)
  } catch {
    return trimmed
  }
}

/**
 * The background layer shows a single user-picked image (or nothing).
 * Slideshow/video modes were removed; only `image` and `none` remain.
 */
export function useBackgroundMedia() {
  const themeStore = useThemeStore()

  const backgroundMediaSrc = computed(() => {
    if (themeStore.backgroundMode !== 'image') return ''
    return resolveMediaSrc(themeStore.backgroundPath)
  })

  function cleanup() {
    // Nothing to tear down: no timers, no video element.
  }

  return {
    backgroundMediaSrc,
    cleanup,
  }
}

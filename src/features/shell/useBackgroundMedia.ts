import { computed, ref, watch } from 'vue'
import { convertFileSrc, invoke } from '@tauri-apps/api/core'
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

export function useBackgroundMedia() {
  const themeStore = useThemeStore()
  const slideshowIndex = ref(0)
  const slideshowFiles = ref<string[]>([])
  let slideshowTimer: number | null = null

  async function scanSlideshowFolder(folder: string) {
    if (!folder) {
      slideshowFiles.value = []
      return
    }
    try {
      const files = await invoke<string[]>('tools_list_media_files', {
        folderPath: folder,
        kind: 'image',
      })
      if (Array.isArray(files) && files.length > 0) {
        slideshowFiles.value = files
      } else {
        slideshowFiles.value = []
      }
    } catch {
      slideshowFiles.value = []
    }
  }

  const backgroundMediaSrc = computed(() => {
    if (themeStore.backgroundMode === 'none') return ''
    if (themeStore.backgroundMode === 'slideshow') {
      if (slideshowFiles.value.length > 0) {
        const idx = slideshowIndex.value % slideshowFiles.value.length
        return resolveMediaSrc(slideshowFiles.value[idx])
      }
      return resolveMediaSrc(themeStore.backgroundPath)
    }
    return resolveMediaSrc(themeStore.backgroundPath)
  })

  watch(
    () => [themeStore.backgroundMode, themeStore.backgroundPath] as const,
    ([mode, path]) => {
      if (mode === 'slideshow') {
        scanSlideshowFolder(path)
        if (!slideshowTimer) {
          slideshowTimer = window.setInterval(() => {
            if (slideshowFiles.value.length > 0) {
              slideshowIndex.value = (slideshowIndex.value + 1) % slideshowFiles.value.length
            }
          }, 8000) as unknown as number
        }
      } else {
        if (slideshowTimer) {
          clearInterval(slideshowTimer)
          slideshowTimer = null
        }
      }
    },
    { immediate: true },
  )

  function cleanup() {
    if (slideshowTimer) {
      clearInterval(slideshowTimer)
      slideshowTimer = null
    }
  }

  return {
    backgroundMediaSrc,
    cleanup,
  }
}

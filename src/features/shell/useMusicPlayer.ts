import { computed, ref, watch, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useThemeStore } from '../../stores/theme'
import { resolveMediaSrc } from './useBackgroundMedia'

export function useMusicPlayer() {
  const themeStore = useThemeStore()
  const audioElementRef = ref<HTMLAudioElement | null>(null)
  let unlistenStarted: UnlistenFn | null = null
  let unlistenExit: UnlistenFn | null = null

  const currentTrackSrc = computed(() => {
    if (!themeStore.musicEnabled || themeStore.playlist.length === 0) return ''
    const track = themeStore.playlist[themeStore.currentTrackIndex]
    if (!track) return ''
    return resolveMediaSrc(track)
  })

  async function scanMusicFolder(folder: string) {
    if (!folder) {
      themeStore.playlist = []
      return
    }
    try {
      const files = await invoke<string[]>('tools_list_media_files', {
        folderPath: folder,
        kind: 'audio',
      })
      if (Array.isArray(files) && files.length > 0) {
        themeStore.playlist = files
        if (themeStore.currentTrackIndex >= files.length) {
          themeStore.currentTrackIndex = 0
        }
      } else {
        themeStore.playlist = []
      }
    } catch {
      themeStore.playlist = []
    }
  }

  // Watch volume and apply to audio element
  watch(
    () => themeStore.musicVolume,
    (vol) => {
      if (audioElementRef.value) {
        audioElementRef.value.volume = Math.max(0, Math.min(1, vol / 100))
      }
    },
    { immediate: true },
  )

  // Watch playback state
  watch(
    () => themeStore.isPlaying,
    (playing) => {
      if (!audioElementRef.value) return
      if (playing && themeStore.musicEnabled && currentTrackSrc.value) {
        audioElementRef.value.play().catch(() => {
          // Autoplay policy or empty audio in tests
        })
      } else {
        audioElementRef.value.pause()
      }
    },
  )

  // Watch folder change
  watch(
    () => themeStore.musicFolder,
    (f) => {
      scanMusicFolder(f)
    },
  )

  function handleAudioEnded() {
    if (!themeStore.musicEnabled || themeStore.playlist.length === 0) return
    if (themeStore.musicPlayMode === 'shuffle') {
      const nextIdx = Math.floor(Math.random() * themeStore.playlist.length)
      themeStore.currentTrackIndex = nextIdx
    } else {
      themeStore.currentTrackIndex = (themeStore.currentTrackIndex + 1) % themeStore.playlist.length
    }
    if (themeStore.isPlaying && audioElementRef.value) {
      audioElementRef.value.play().catch(() => {})
    }
  }

  function togglePlay() {
    themeStore.isPlaying = !themeStore.isPlaying
  }

  function nextTrack() {
    if (themeStore.playlist.length === 0) return
    if (themeStore.musicPlayMode === 'shuffle') {
      const nextIdx = Math.floor(Math.random() * themeStore.playlist.length)
      themeStore.currentTrackIndex = nextIdx
    } else {
      themeStore.currentTrackIndex = (themeStore.currentTrackIndex + 1) % themeStore.playlist.length
    }
    themeStore.isPlaying = true
  }

  function prevTrack() {
    if (themeStore.playlist.length === 0) return
    themeStore.currentTrackIndex =
      (themeStore.currentTrackIndex - 1 + themeStore.playlist.length) % themeStore.playlist.length
    themeStore.isPlaying = true
  }

  function togglePlayMode() {
    themeStore.setMusicPlayMode(themeStore.musicPlayMode === 'loop' ? 'shuffle' : 'loop')
  }

  async function setupGameListeners() {
    try {
      unlistenStarted = await listen('launch://started', () => {
        if (themeStore.isPlaying) {
          themeStore.pausedByGame = true
          themeStore.isPlaying = false
        }
      })

      unlistenExit = await listen('launch://exit', () => {
        if (themeStore.pausedByGame) {
          themeStore.pausedByGame = false
          if (themeStore.musicEnabled) {
            themeStore.isPlaying = true
          }
        }
      })
    } catch {
      // In browser / test mock mode
    }
  }

  onMounted(() => {
    setupGameListeners()
    if (themeStore.musicFolder) {
      scanMusicFolder(themeStore.musicFolder)
    }
  })

  onUnmounted(() => {
    if (unlistenStarted) {
      unlistenStarted()
      unlistenStarted = null
    }
    if (unlistenExit) {
      unlistenExit()
      unlistenExit = null
    }
  })

  return {
    audioElementRef,
    currentTrackSrc,
    handleAudioEnded,
    togglePlay,
    nextTrack,
    prevTrack,
    togglePlayMode,
  }
}

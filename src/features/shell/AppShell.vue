<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted, watch } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { featureRoutes, type NavMeta } from '../../router'
import { useThemeStore } from '../../stores/theme'
import { MToast, MDialog, useDialog } from '../../ui'
import TaskIndicator from '../tasks/TaskIndicator.vue'
import { useCrashStore } from '../crash/store'
import CrashDialog from '../crash/CrashDialog.vue'

const { t } = useI18n()
const router = useRouter()
const route = useRoute()
const themeStore = useThemeStore()
const { activeDialog, handleConfirm, handleCancel } = useDialog()
const crashStore = useCrashStore()

// Music & Launch lifecycle
let unlistenStarted: UnlistenFn | null = null
let unlistenExit: UnlistenFn | null = null

// Audio element ref
const audioElementRef = ref<HTMLAudioElement | null>(null)
const slideshowIndex = ref(0)
let slideshowTimer: ReturnType<typeof setInterval> | null = null
const slideshowFiles = ref<string[]>([])

// Popover/expand for music controls in titlebar
const showMusicPopover = ref(false)

const navItems = computed(() => {
  const items: Array<{ path: string; name: string; meta: NavMeta }> = []
  for (const r of featureRoutes) {
    if (r.meta && 'nav' in r.meta && r.meta.nav) {
      items.push({
        path: r.path,
        name: String(r.name ?? r.path),
        meta: r.meta.nav as NavMeta,
      })
    }
  }
  return items.sort((a, b) => (a.meta.order ?? 0) - (b.meta.order ?? 0))
})

// Resolve file src for background (supports http, data:, or local path via convertFileSrc)
function resolveMediaSrc(filePath: string): string {
  if (!filePath) return ''
  const trimmed = filePath.trim()
  if (trimmed.startsWith('http://') || trimmed.startsWith('https://') || trimmed.startsWith('data:')) {
    return trimmed
  }
  try {
    return convertFileSrc(trimmed)
  } catch {
    return trimmed
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
    // Fallback if tools command not yet registered / mock
    themeStore.playlist = []
  }
}

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

// Watch slideshow folder
watch(
  () => [themeStore.backgroundMode, themeStore.backgroundPath] as const,
  ([mode, path]) => {
    if (mode === 'slideshow') {
      scanSlideshowFolder(path)
      if (!slideshowTimer) {
        slideshowTimer = setInterval(() => {
          if (slideshowFiles.value.length > 0) {
            slideshowIndex.value = (slideshowIndex.value + 1) % slideshowFiles.value.length
          }
        }, 8000)
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
  crashStore.startListening()
  setupGameListeners()
  if (themeStore.musicFolder) {
    scanMusicFolder(themeStore.musicFolder)
  }
})

onUnmounted(() => {
  crashStore.stopListening()
  if (unlistenStarted) {
    unlistenStarted()
    unlistenStarted = null
  }
  if (unlistenExit) {
    unlistenExit()
    unlistenExit = null
  }
  if (slideshowTimer) {
    clearInterval(slideshowTimer)
    slideshowTimer = null
  }
})

async function minimizeWindow() {
  try {
    const win = getCurrentWindow()
    await win.minimize()
  } catch {
    // Non-tauri browser mode fallback
  }
}

async function toggleMaximizeWindow() {
  try {
    const win = getCurrentWindow()
    await win.toggleMaximize()
  } catch {
    // Non-tauri browser mode fallback
  }
}

async function closeWindow() {
  try {
    const win = getCurrentWindow()
    await win.close()
  } catch {
    // Non-tauri browser mode fallback
  }
}

function toggleMode() {
  if (themeStore.mode === 'light') {
    themeStore.setMode('dark')
  } else if (themeStore.mode === 'dark') {
    themeStore.setMode('system')
  } else {
    themeStore.setMode('light')
  }
}

function toggleMaterial() {
  themeStore.setMaterial(themeStore.material === 'glass' ? 'flat' : 'glass')
}
</script>

<template>
  <div class="relative w-screen h-screen flex flex-col bg-surface text-text select-none overflow-hidden">
    <!-- SVG Refraction Filter for Liquid Glass -->
    <svg class="absolute w-0 h-0 overflow-hidden" aria-hidden="true">
      <defs>
        <filter id="mclr-glass" x="-20%" y="-20%" width="140%" height="140%">
          <feTurbulence
            type="fractalNoise"
            baseFrequency="0.04"
            numOctaves="3"
            result="noise"
          />
          <feDisplacementMap
            in="SourceGraphic"
            in2="noise"
            scale="4"
            xChannelSelector="R"
            yChannelSelector="G"
          />
        </filter>
      </defs>
    </svg>

    <!-- Background Layer (Behind shell content) -->
    <div
      v-if="themeStore.backgroundMode !== 'none' && backgroundMediaSrc"
      data-testid="bg-layer"
      class="pointer-events-none fixed inset-0 z-0 overflow-hidden"
    >
      <video
        v-if="themeStore.backgroundMode === 'video'"
        data-testid="bg-video"
        autoplay
        muted
        loop
        playsinline
        class="w-full h-full object-cover transition-opacity duration-500"
        :src="backgroundMediaSrc"
        :style="{
          filter: `blur(${themeStore.backgroundBlur}px)`,
          opacity: themeStore.backgroundOpacity,
        }"
      />
      <img
        v-else-if="themeStore.backgroundMode === 'image' || themeStore.backgroundMode === 'slideshow'"
        data-testid="bg-img"
        class="w-full h-full object-cover transition-opacity duration-500"
        :src="backgroundMediaSrc"
        :style="{
          filter: `blur(${themeStore.backgroundBlur}px)`,
          opacity: themeStore.backgroundOpacity,
        }"
        alt="Background"
      />
    </div>

    <!-- Hidden HTMLAudioElement for Background Music -->
    <audio
      v-if="themeStore.musicEnabled"
      ref="audioElementRef"
      data-testid="bg-audio"
      class="hidden"
      :src="currentTrackSrc"
      @ended="handleAudioEnded"
    />

    <!-- Custom Frameless Titlebar -->
    <header
      data-tauri-drag-region
      class="relative h-10 shrink-0 flex items-center justify-between px-3 border-b border-outline/40 bg-surfaceVariant/30 backdrop-blur-md z-40 select-none"
    >
      <div data-tauri-drag-region class="flex items-center gap-2.5">
        <div class="w-5 h-5 rounded bg-primary text-onPrimary flex items-center justify-center font-bold text-xs shadow-sm">
          M
        </div>
        <span data-tauri-drag-region class="text-xs font-semibold tracking-wide text-text">
          {{ t('app.name') }}
        </span>
      </div>

      <!-- Center Titlebar / Task Indicator Slot -->
      <div data-tauri-drag-region class="flex-1 flex items-center justify-center px-4">
        <div id="mclr-task-indicator-slot" class="flex items-center gap-2 text-xs text-textMuted">
          <TaskIndicator />
        </div>
      </div>

      <!-- Titlebar Quick Actions & Window Controls -->
      <div class="flex items-center gap-1">
        <!-- Music Player Controller in Titlebar (data-testid="music-controls") -->
        <div
          v-if="themeStore.musicEnabled"
          data-testid="music-controls"
          class="flex items-center gap-1 mr-1 px-1.5 py-0.5 rounded-lg bg-surfaceVariant/60 border border-outline/30 text-xs"
        >
          <button
            type="button"
            :title="t('shell.titlebar.musicPrev')"
            class="w-6 h-6 flex items-center justify-center rounded hover:bg-surface text-textMuted hover:text-text cursor-pointer transition-colors"
            @click="prevTrack"
          >
            <span class="i-lucide-skip-back w-3 h-3" />
          </button>

          <button
            type="button"
            data-testid="music-play-btn"
            :title="themeStore.isPlaying ? t('shell.titlebar.musicPause') : t('shell.titlebar.musicPlay')"
            class="w-6 h-6 flex items-center justify-center rounded bg-primary text-onPrimary cursor-pointer transition-colors shadow-xs"
            @click="togglePlay"
          >
            <span :class="themeStore.isPlaying ? 'i-lucide-pause' : 'i-lucide-play'" class="w-3 h-3" />
          </button>

          <button
            type="button"
            data-testid="music-next-btn"
            :title="t('shell.titlebar.musicNext')"
            class="w-6 h-6 flex items-center justify-center rounded hover:bg-surface text-textMuted hover:text-text cursor-pointer transition-colors"
            @click="nextTrack"
          >
            <span class="i-lucide-skip-forward w-3 h-3" />
          </button>

          <button
            type="button"
            data-testid="music-mode-btn"
            :title="themeStore.musicPlayMode === 'loop' ? t('shell.titlebar.musicLoop') : t('shell.titlebar.musicShuffle')"
            class="w-6 h-6 flex items-center justify-center rounded hover:bg-surface text-textMuted hover:text-text cursor-pointer transition-colors"
            @click="togglePlayMode"
          >
            <span
              :class="themeStore.musicPlayMode === 'loop' ? 'i-lucide-repeat' : 'i-lucide-shuffle'"
              class="w-3 h-3"
            />
          </button>

          <!-- Compact volume popover trigger -->
          <div class="relative">
            <button
              type="button"
              :title="t('shell.titlebar.music')"
              class="w-6 h-6 flex items-center justify-center rounded hover:bg-surface text-textMuted hover:text-text cursor-pointer transition-colors"
              @click="showMusicPopover = !showMusicPopover"
            >
              <span class="i-lucide-volume-2 w-3 h-3" />
            </button>

            <!-- Volume slider dropdown -->
            <div
              v-if="showMusicPopover"
              class="absolute right-0 top-8 w-36 p-2 rounded-xl bg-surface border border-outline shadow-lg z-50 flex items-center gap-2"
            >
              <span class="i-lucide-volume w-3 h-3 text-textMuted" />
              <input
                type="range"
                :value="themeStore.musicVolume"
                min="0"
                max="100"
                class="w-full h-1.5 bg-surfaceVariant rounded appearance-none cursor-pointer accent-primary"
                @input="themeStore.setMusicVolume(Number(($event.target as HTMLInputElement).value))"
              />
            </div>
          </div>
        </div>

        <button
          type="button"
          :title="t('shell.titlebar.theme')"
          class="w-7 h-7 flex items-center justify-center rounded hover:bg-surfaceVariant/80 text-textMuted hover:text-text cursor-pointer transition-colors"
          @click="toggleMode"
        >
          <span
            class="w-3.5 h-3.5"
            :class="{
              'i-lucide-sun': themeStore.mode === 'light',
              'i-lucide-moon': themeStore.mode === 'dark',
              'i-lucide-laptop': themeStore.mode === 'system',
            }"
          />
        </button>

        <button
          type="button"
          :title="t('shell.titlebar.material')"
          class="w-7 h-7 flex items-center justify-center rounded hover:bg-surfaceVariant/80 text-textMuted hover:text-text cursor-pointer transition-colors"
          :class="{ 'text-primary': themeStore.material === 'glass' }"
          @click="toggleMaterial"
        >
          <span class="i-lucide-sparkles w-3.5 h-3.5" />
        </button>

        <div class="w-px h-3.5 bg-outline/40 mx-1" />

        <button
          type="button"
          :title="t('app.window.minimize')"
          class="w-7 h-7 flex items-center justify-center rounded hover:bg-surfaceVariant/80 text-textMuted hover:text-text cursor-pointer transition-colors"
          @click="minimizeWindow"
        >
          <span class="i-lucide-minus w-3.5 h-3.5" />
        </button>

        <button
          type="button"
          :title="t('app.window.maximize')"
          class="w-7 h-7 flex items-center justify-center rounded hover:bg-surfaceVariant/80 text-textMuted hover:text-text cursor-pointer transition-colors"
          @click="toggleMaximizeWindow"
        >
          <span class="i-lucide-square w-3 h-3" />
        </button>

        <button
          type="button"
          :title="t('app.window.close')"
          class="w-7 h-7 flex items-center justify-center rounded hover:bg-danger hover:text-white text-textMuted cursor-pointer transition-colors"
          @click="closeWindow"
        >
          <span class="i-lucide-x w-3.5 h-3.5" />
        </button>
      </div>
    </header>

    <!-- Main Body: Sidebar + Content (Relative z-10 over background) -->
    <div class="relative z-10 flex-1 flex overflow-hidden">
      <!-- Left Navigation Bar -->
      <nav class="w-48 shrink-0 flex flex-col gap-1 p-2 border-r border-outline/40 bg-surfaceVariant/20 backdrop-blur-sm select-none">
        <button
          v-for="item in navItems"
          :key="item.path"
          type="button"
          class="flex items-center gap-2.5 px-3 py-2 text-xs font-medium rounded-xl transition-all cursor-pointer text-left"
          :class="
            route?.path === item.path
              ? 'bg-primary text-onPrimary font-semibold shadow-sm'
              : 'text-textMuted hover:text-text hover:bg-surfaceVariant/60'
          "
          @click="router?.push(item.path)"
        >
          <span :class="item.meta.icon" class="w-4 h-4 shrink-0" />
          <span class="truncate">{{ t(item.meta.titleKey) }}</span>
        </button>
      </nav>

      <!-- Content Area -->
      <main class="flex-1 overflow-auto bg-surface/50 p-6">
        <RouterView v-slot="{ Component }">
          <Transition name="page" mode="out-in">
            <component :is="Component" />
          </Transition>
        </RouterView>
      </main>
    </div>

    <!-- Global Toast & Dialog Hosts -->
    <MToast />

    <MDialog
      v-if="activeDialog"
      :open="activeDialog.open"
      :title="activeDialog.title"
      :confirm-text="activeDialog.confirmText"
      :cancel-text="activeDialog.cancelText"
      @confirm="handleConfirm"
      @cancel="handleCancel"
    >
      <p>{{ activeDialog.content }}</p>
    </MDialog>

    <!-- Global Crash Analysis Dialog -->
    <CrashDialog />
  </div>
</template>

<style scoped>
.page-enter-active,
.page-leave-active {
  transition: opacity 0.15s ease, transform 0.15s ease;
}

.page-enter-from {
  opacity: 0;
  transform: translateY(4px);
}

.page-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}
</style>

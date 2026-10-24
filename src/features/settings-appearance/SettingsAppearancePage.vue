<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  useThemeStore,
  type BackgroundMode,
  type MusicPlayMode,
} from '../../stores/theme'
import type { ThemeMode, MaterialMode } from '../../ui/theme'
import MCard from '../../ui/MCard.vue'
import MButton from '../../ui/MButton.vue'
import MInput from '../../ui/MInput.vue'
import MSelect from '../../ui/MSelect.vue'
import MSlider from '../../ui/MSlider.vue'
import MSwitch from '../../ui/MSwitch.vue'
import MSegmented from '../../ui/MSegmented.vue'
import MColorPicker from '../../ui/MColorPicker.vue'
import { convertFileSrc } from '@tauri-apps/api/core'
import { open as openFileDialog } from '@tauri-apps/plugin-dialog'

const { t } = useI18n()
const themeStore = useThemeStore()

// System fonts detection via Local Font Access API
const systemFonts = ref<string[]>([])
const isScanningFonts = ref(false)

const modeOptions = computed(() => [
  { label: t('settingsAppearance.theme.modeLight'), value: 'light', icon: 'i-lucide-sun' },
  { label: t('settingsAppearance.theme.modeDark'), value: 'dark', icon: 'i-lucide-moon' },
  { label: t('settingsAppearance.theme.modeSystem'), value: 'system', icon: 'i-lucide-laptop' },
])

const materialOptions = computed(() => [
  { label: t('settingsAppearance.theme.materialFlat'), value: 'flat', icon: 'i-lucide-square' },
  { label: t('settingsAppearance.theme.materialGlass'), value: 'glass', icon: 'i-lucide-sparkles' },
])

const bgModeOptions = computed(() => [
  { label: t('settingsAppearance.background.modeNone'), value: 'none' },
  { label: t('settingsAppearance.background.modeImage'), value: 'image' },
  { label: t('settingsAppearance.background.modeSlideshow'), value: 'slideshow' },
  { label: t('settingsAppearance.background.modeVideo'), value: 'video' },
])

const musicPlayModeOptions = computed(() => [
  { label: t('settingsAppearance.music.modeLoop'), value: 'loop', icon: 'i-lucide-repeat' },
  { label: t('settingsAppearance.music.modeShuffle'), value: 'shuffle', icon: 'i-lucide-shuffle' },
])

const fontSelectOptions = computed(() => {
  const options = [{ label: t('settingsAppearance.typography.defaultFont'), value: '' }]
  for (const f of systemFonts.value) {
    options.push({ label: f, value: f })
  }
  return options
})

const previewMediaSrc = computed(() => {
  if (!themeStore.backgroundPath) return ''
  const p = themeStore.backgroundPath.trim()
  if (p.startsWith('http://') || p.startsWith('https://') || p.startsWith('data:')) {
    return p
  }
  try {
    return convertFileSrc(p)
  } catch {
    return p
  }
})

async function querySystemFonts() {
  isScanningFonts.value = true
  try {
    const windowWithFonts = window as unknown as {
      queryLocalFonts?: () => Promise<Array<{ family: string }>>
    }
    if (typeof windowWithFonts.queryLocalFonts === 'function') {
      const fonts = await windowWithFonts.queryLocalFonts()
      const families = Array.from(new Set(fonts.map((f) => f.family))).sort()
      systemFonts.value = families
    }
  } catch {
    // If permission denied or unsupported in environment
  } finally {
    isScanningFonts.value = false
  }
}

async function browseBackground() {
  try {
    if (themeStore.backgroundMode === 'slideshow') {
      const selected = await openFileDialog({
        directory: true,
        multiple: false,
      })
      if (typeof selected === 'string' && selected) {
        themeStore.setBackgroundPath(selected)
      }
    } else if (themeStore.backgroundMode === 'video') {
      const selected = await openFileDialog({
        multiple: false,
        filters: [{ name: 'Video', extensions: ['mp4', 'webm', 'mkv', 'avi'] }],
      })
      if (typeof selected === 'string' && selected) {
        themeStore.setBackgroundPath(selected)
      }
    } else {
      const selected = await openFileDialog({
        multiple: false,
        filters: [{ name: 'Image', extensions: ['png', 'jpg', 'jpeg', 'webp', 'gif', 'bmp'] }],
      })
      if (typeof selected === 'string' && selected) {
        themeStore.setBackgroundPath(selected)
      }
    }
  } catch {
    // Handled in browser / mock mode
  }
}

async function browseMusicFolder() {
  try {
    const selected = await openFileDialog({
      directory: true,
      multiple: false,
    })
    if (typeof selected === 'string' && selected) {
      themeStore.setMusicFolder(selected)
    }
  } catch {
    // Handled in browser / mock mode
  }
}

function togglePlayback() {
  themeStore.isPlaying = !themeStore.isPlaying
}

function playNextTrack() {
  if (themeStore.playlist.length > 0) {
    if (themeStore.musicPlayMode === 'shuffle') {
      const nextIdx = Math.floor(Math.random() * themeStore.playlist.length)
      themeStore.currentTrackIndex = nextIdx
    } else {
      themeStore.currentTrackIndex = (themeStore.currentTrackIndex + 1) % themeStore.playlist.length
    }
    themeStore.isPlaying = true
  }
}

function playPrevTrack() {
  if (themeStore.playlist.length > 0) {
    themeStore.currentTrackIndex =
      (themeStore.currentTrackIndex - 1 + themeStore.playlist.length) % themeStore.playlist.length
    themeStore.isPlaying = true
  }
}

onMounted(() => {
  querySystemFonts()
})
</script>

<template>
  <div class="max-w-4xl mx-auto space-y-6 pb-12 select-none">
    <!-- Header -->
    <div>
      <h1 class="text-xl font-bold tracking-tight text-text">
        {{ t('settingsAppearance.title') }}
      </h1>
      <p class="text-xs text-textMuted mt-1">
        {{ t('settingsAppearance.subtitle') }}
      </p>
    </div>

    <!-- 1. Theme & Material Section -->
    <MCard>
      <div class="space-y-5">
        <div class="flex items-center gap-2 border-b border-outline/30 pb-3">
          <span class="i-lucide-palette w-4 h-4 text-primary" />
          <h2 class="text-sm font-semibold text-text">
            {{ t('settingsAppearance.sections.theme') }}
          </h2>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-2 gap-6 items-start">
          <!-- Seed Color Picker -->
          <div class="space-y-2">
            <label class="text-xs font-medium text-text">
              {{ t('settingsAppearance.theme.seedColor') }}
            </label>
            <MColorPicker
              :model-value="themeStore.seed"
              @update:model-value="themeStore.setSeed($event)"
            />
          </div>

          <!-- Mode & Material -->
          <div class="space-y-4">
            <div class="space-y-2">
              <label class="text-xs font-medium text-text">
                {{ t('settingsAppearance.theme.mode') }}
              </label>
              <div>
                <MSegmented
                  :model-value="themeStore.mode"
                  :options="modeOptions"
                  @update:model-value="themeStore.setMode($event as ThemeMode)"
                />
              </div>
            </div>

            <div class="space-y-2">
              <label class="text-xs font-medium text-text">
                {{ t('settingsAppearance.theme.material') }}
              </label>
              <div>
                <MSegmented
                  :model-value="themeStore.material"
                  :options="materialOptions"
                  @update:model-value="themeStore.setMaterial($event as MaterialMode)"
                />
              </div>
              <p class="text-[11px] text-textMuted leading-relaxed">
                {{ t('settingsAppearance.theme.materialDesc') }}
              </p>
            </div>
          </div>
        </div>
      </div>
    </MCard>

    <!-- 2. Typography & Scale Section -->
    <MCard>
      <div class="space-y-5">
        <div class="flex items-center gap-2 border-b border-outline/30 pb-3">
          <span class="i-lucide-type w-4 h-4 text-primary" />
          <h2 class="text-sm font-semibold text-text">
            {{ t('settingsAppearance.sections.typography') }}
          </h2>
        </div>

        <div class="space-y-4">
          <!-- Font Family -->
          <div class="space-y-2">
            <div class="flex items-center justify-between">
              <label class="text-xs font-medium text-text">
                {{ t('settingsAppearance.typography.fontFamily') }}
              </label>
              <MButton
                variant="tonal"
                size="sm"
                icon="i-lucide-refresh-cw"
                :loading="isScanningFonts"
                @click="querySystemFonts"
              >
                {{ t('settingsAppearance.typography.loadSystemFonts') }}
              </MButton>
            </div>

            <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
              <MSelect
                v-if="systemFonts.length > 0"
                :model-value="themeStore.fontFamily"
                :options="fontSelectOptions"
                @update:model-value="themeStore.setFontFamily(String($event))"
              />
              <MInput
                :model-value="themeStore.fontFamily"
                :placeholder="t('settingsAppearance.typography.fontFamilyPlaceholder')"
                @update:model-value="themeStore.setFontFamily($event)"
              />
            </div>
          </div>

          <!-- UI Scale Slider -->
          <div class="space-y-2 pt-2">
            <div class="flex items-center justify-between">
              <label class="text-xs font-medium text-text">
                {{ t('settingsAppearance.typography.scale') }}
              </label>
              <span class="text-xs font-mono font-semibold text-primary">
                {{ t('settingsAppearance.typography.scaleValue', { val: Math.round(themeStore.scale * 100) }) }}
              </span>
            </div>
            <MSlider
              :model-value="themeStore.scale"
              :min="0.8"
              :max="1.25"
              :step="0.05"
              @update:model-value="themeStore.setScale($event)"
            />
          </div>
        </div>
      </div>
    </MCard>

    <!-- 3. Background Section -->
    <MCard>
      <div class="space-y-5">
        <div class="flex items-center gap-2 border-b border-outline/30 pb-3">
          <span class="i-lucide-image w-4 h-4 text-primary" />
          <h2 class="text-sm font-semibold text-text">
            {{ t('settingsAppearance.sections.background') }}
          </h2>
        </div>

        <div class="space-y-4">
          <!-- Background Mode -->
          <div class="space-y-2">
            <label class="text-xs font-medium text-text">
              {{ t('settingsAppearance.background.mode') }}
            </label>
            <div>
              <MSegmented
                :model-value="themeStore.backgroundMode"
                :options="bgModeOptions"
                @update:model-value="themeStore.setBackgroundMode($event as BackgroundMode)"
              />
            </div>
          </div>

          <!-- Background Path & Browse -->
          <div v-if="themeStore.backgroundMode !== 'none'" class="space-y-2">
            <label class="text-xs font-medium text-text">
              {{ t('settingsAppearance.background.path') }}
            </label>
            <div class="flex items-center gap-2">
              <div class="flex-1">
                <MInput
                  :model-value="themeStore.backgroundPath"
                  :placeholder="t('settingsAppearance.background.pathPlaceholder')"
                  @update:model-value="themeStore.setBackgroundPath($event)"
                />
              </div>
              <MButton variant="tonal" icon="i-lucide-folder-open" @click="browseBackground">
                {{ t('settingsAppearance.background.browse') }}
              </MButton>
            </div>
          </div>

          <!-- Blur & Opacity Sliders -->
          <div v-if="themeStore.backgroundMode !== 'none'" class="grid grid-cols-1 md:grid-cols-2 gap-4 pt-2">
            <!-- Blur -->
            <div class="space-y-2">
              <div class="flex items-center justify-between">
                <label class="text-xs font-medium text-text">
                  {{ t('settingsAppearance.background.blur') }}
                </label>
                <span class="text-xs font-mono font-semibold text-primary">
                  {{ t('settingsAppearance.background.blurValue', { val: themeStore.backgroundBlur }) }}
                </span>
              </div>
              <MSlider
                :model-value="themeStore.backgroundBlur"
                :min="0"
                :max="32"
                :step="1"
                @update:model-value="themeStore.setBackgroundBlur($event)"
              />
            </div>

            <!-- Opacity -->
            <div class="space-y-2">
              <div class="flex items-center justify-between">
                <label class="text-xs font-medium text-text">
                  {{ t('settingsAppearance.background.opacity') }}
                </label>
                <span class="text-xs font-mono font-semibold text-primary">
                  {{ t('settingsAppearance.background.opacityValue', { val: Math.round(themeStore.backgroundOpacity * 100) }) }}
                </span>
              </div>
              <MSlider
                :model-value="themeStore.backgroundOpacity"
                :min="0"
                :max="1"
                :step="0.05"
                @update:model-value="themeStore.setBackgroundOpacity($event)"
              />
            </div>
          </div>

          <!-- Live Preview Box -->
          <div class="space-y-2 pt-2">
            <label class="text-xs font-medium text-text">
              {{ t('settingsAppearance.background.preview') }}
            </label>
            <div
              class="relative w-full h-44 rounded-xl border border-outline/50 bg-surfaceVariant/30 overflow-hidden flex items-center justify-center"
            >
              <template v-if="themeStore.backgroundMode === 'video' && previewMediaSrc">
                <video
                  data-testid="bg-preview-video"
                  autoplay
                  muted
                  loop
                  playsinline
                  class="absolute inset-0 w-full h-full object-cover pointer-events-none"
                  :src="previewMediaSrc"
                  :style="{
                    filter: `blur(${themeStore.backgroundBlur}px)`,
                    opacity: themeStore.backgroundOpacity,
                  }"
                />
              </template>
              <template v-else-if="(themeStore.backgroundMode === 'image' || themeStore.backgroundMode === 'slideshow') && previewMediaSrc">
                <img
                  data-testid="bg-preview-img"
                  class="absolute inset-0 w-full h-full object-cover pointer-events-none"
                  :src="previewMediaSrc"
                  :style="{
                    filter: `blur(${themeStore.backgroundBlur}px)`,
                    opacity: themeStore.backgroundOpacity,
                  }"
                  alt="Background preview"
                />
              </template>
              <div
                v-else
                class="text-xs text-textMuted flex items-center gap-1.5 z-10"
              >
                <span class="i-lucide-image-off w-4 h-4" />
                <span>{{ t('settingsAppearance.background.previewEmpty') }}</span>
              </div>

              <!-- Foreground Card Sample for Contrast -->
              <div
                class="relative z-10 px-4 py-2.5 rounded-lg border border-outline/60 bg-surface/80 backdrop-blur-md shadow-md text-center"
              >
                <p class="text-xs font-semibold text-text">MCLR Preview</p>
                <p class="text-[11px] text-textMuted">Sample UI Foreground Content</p>
              </div>
            </div>
          </div>
        </div>
      </div>
    </MCard>

    <!-- 4. Background Music Section -->
    <MCard>
      <div class="space-y-5">
        <div class="flex items-center gap-2 border-b border-outline/30 pb-3">
          <span class="i-lucide-music w-4 h-4 text-primary" />
          <h2 class="text-sm font-semibold text-text">
            {{ t('settingsAppearance.sections.music') }}
          </h2>
        </div>

        <div class="space-y-4">
          <!-- Enable Music Switch -->
          <div class="flex items-center justify-between">
            <div>
              <p class="text-xs font-medium text-text">
                {{ t('settingsAppearance.music.enable') }}
              </p>
            </div>
            <MSwitch
              :model-value="themeStore.musicEnabled"
              @update:model-value="themeStore.setMusicEnabled($event)"
            />
          </div>

          <template v-if="themeStore.musicEnabled">
            <!-- Music Folder Input & Browse -->
            <div class="space-y-2">
              <label class="text-xs font-medium text-text">
                {{ t('settingsAppearance.music.folder') }}
              </label>
              <div class="flex items-center gap-2">
                <div class="flex-1">
                  <MInput
                    :model-value="themeStore.musicFolder"
                    :placeholder="t('settingsAppearance.music.folderPlaceholder')"
                    @update:model-value="themeStore.setMusicFolder($event)"
                  />
                </div>
                <MButton variant="tonal" icon="i-lucide-folder-open" @click="browseMusicFolder">
                  {{ t('settingsAppearance.music.browse') }}
                </MButton>
              </div>
            </div>

            <!-- Play Mode & Volume -->
            <div class="grid grid-cols-1 md:grid-cols-2 gap-4 pt-2">
              <div class="space-y-2">
                <label class="text-xs font-medium text-text">
                  {{ t('settingsAppearance.music.playMode') }}
                </label>
                <div>
                  <MSegmented
                    :model-value="themeStore.musicPlayMode"
                    :options="musicPlayModeOptions"
                    @update:model-value="themeStore.setMusicPlayMode($event as MusicPlayMode)"
                  />
                </div>
              </div>

              <div class="space-y-2">
                <div class="flex items-center justify-between">
                  <label class="text-xs font-medium text-text">
                    {{ t('settingsAppearance.music.volume') }}
                  </label>
                  <span class="text-xs font-mono font-semibold text-primary">
                    {{ t('settingsAppearance.music.volumeValue', { val: themeStore.musicVolume }) }}
                  </span>
                </div>
                <MSlider
                  :model-value="themeStore.musicVolume"
                  :min="0"
                  :max="100"
                  :step="1"
                  @update:model-value="themeStore.setMusicVolume($event)"
                />
              </div>
            </div>

            <!-- Music Controls Preview -->
            <div class="p-3 rounded-xl border border-outline/40 bg-surfaceVariant/30 flex items-center justify-between">
              <div class="flex items-center gap-3">
                <div class="w-8 h-8 rounded-lg bg-primary/20 text-primary flex items-center justify-center">
                  <span class="i-lucide-disc w-4 h-4 animate-spin" :class="{ 'animate-none': !themeStore.isPlaying }" />
                </div>
                <div>
                  <p class="text-xs font-semibold text-text truncate max-w-48">
                    {{ themeStore.playlist[themeStore.currentTrackIndex] || t('settingsAppearance.music.noTrack') }}
                  </p>
                  <p class="text-[11px] text-textMuted">
                    {{ themeStore.playlist.length }} tracks
                  </p>
                </div>
              </div>

              <!-- Controls -->
              <div class="flex items-center gap-1.5" data-testid="music-page-controls">
                <MButton
                  variant="tonal"
                  size="sm"
                  icon="i-lucide-skip-back"
                  @click="playPrevTrack"
                />
                <MButton
                  variant="primary"
                  size="sm"
                  :icon="themeStore.isPlaying ? 'i-lucide-pause' : 'i-lucide-play'"
                  @click="togglePlayback"
                >
                  {{ themeStore.isPlaying ? t('settingsAppearance.music.pause') : t('settingsAppearance.music.play') }}
                </MButton>
                <MButton
                  variant="tonal"
                  size="sm"
                  icon="i-lucide-skip-forward"
                  @click="playNextTrack"
                />
              </div>
            </div>
          </template>
        </div>
      </div>
    </MCard>
  </div>
</template>

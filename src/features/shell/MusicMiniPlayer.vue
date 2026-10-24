<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useThemeStore } from '../../stores/theme'

const props = defineProps<{
  togglePlay: () => void
  nextTrack: () => void
  prevTrack: () => void
  togglePlayMode: () => void
}>()

const { t } = useI18n()
const themeStore = useThemeStore()
const showMusicPopover = ref(false)
</script>

<template>
  <div
    data-testid="music-controls"
    class="flex items-center gap-1 mr-1 px-1.5 py-0.5 rounded-lg bg-sunken/60 border border-line text-xs"
  >
    <button
      type="button"
      :title="t('shell.titlebar.musicPrev')"
      class="w-6 h-6 flex items-center justify-center rounded hover:bg-raised text-fg-muted hover:text-fg cursor-pointer transition-colors"
      @click="props.prevTrack"
    >
      <span class="i-lucide-skip-back w-3 h-3" />
    </button>

    <button
      type="button"
      data-testid="music-play-btn"
      :title="themeStore.isPlaying ? t('shell.titlebar.musicPause') : t('shell.titlebar.musicPlay')"
      class="w-6 h-6 flex items-center justify-center rounded bg-primary text-primary-on cursor-pointer transition-colors shadow-xs"
      @click="props.togglePlay"
    >
      <span :class="themeStore.isPlaying ? 'i-lucide-pause' : 'i-lucide-play'" class="w-3 h-3" />
    </button>

    <button
      type="button"
      data-testid="music-next-btn"
      :title="t('shell.titlebar.musicNext')"
      class="w-6 h-6 flex items-center justify-center rounded hover:bg-raised text-fg-muted hover:text-fg cursor-pointer transition-colors"
      @click="props.nextTrack"
    >
      <span class="i-lucide-skip-forward w-3 h-3" />
    </button>

    <button
      type="button"
      data-testid="music-mode-btn"
      :title="themeStore.musicPlayMode === 'loop' ? t('shell.titlebar.musicLoop') : t('shell.titlebar.musicShuffle')"
      class="w-6 h-6 flex items-center justify-center rounded hover:bg-raised text-fg-muted hover:text-fg cursor-pointer transition-colors"
      @click="props.togglePlayMode"
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
        class="w-6 h-6 flex items-center justify-center rounded hover:bg-raised text-fg-muted hover:text-fg cursor-pointer transition-colors"
        @click="showMusicPopover = !showMusicPopover"
      >
        <span class="i-lucide-volume-2 w-3 h-3" />
      </button>

      <!-- Volume slider dropdown -->
      <div
        v-if="showMusicPopover"
        class="absolute right-0 top-8 w-36 p-2 rounded-xl bg-overlay border border-line shadow-2 z-50 flex items-center gap-2"
      >
        <span class="i-lucide-volume w-3 h-3 text-fg-muted" />
        <input
          type="range"
          :value="themeStore.musicVolume"
          min="0"
          max="100"
          class="w-full h-1.5 bg-sunken rounded appearance-none cursor-pointer accent-primary"
          @input="themeStore.setMusicVolume(Number(($event.target as HTMLInputElement).value))"
        />
      </div>
    </div>
  </div>
</template>

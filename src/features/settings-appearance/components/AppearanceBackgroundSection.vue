<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { convertFileSrc } from '@tauri-apps/api/core'
import { open as openFileDialog } from '@tauri-apps/plugin-dialog'
import { useThemeStore } from '../../../stores/theme'
import { ipcClient } from '../../../ipc/client'
import { MButton, MIconButton, MSlider, MSection, MSettingRow, MHelpTooltip } from '../../../ui'

const { t } = useI18n()
const themeStore = useThemeStore()

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

function getFileName(p: string): string {
  if (!p) return ''
  const parts = p.replace(/\\/g, '/').split('/').filter(Boolean)
  return parts[parts.length - 1] || p
}

async function onPickBackgroundImage() {
  try {
    const selected = await openFileDialog({
      multiple: false,
      filters: [{ name: 'Image', extensions: ['png', 'jpg', 'jpeg', 'webp', 'gif', 'bmp'] }],
    })
    if (!selected) return
    const path = Array.isArray(selected) ? selected[0] : selected
    if (path) {
      await onBackgroundPathChange(path)
    }
  } catch (err) {
    console.warn('Failed to pick background image:', err)
  }
}

async function onBackgroundPathChange(val: string) {
  themeStore.setBackgroundPath(val)
  if (val.trim()) {
    themeStore.setBackgroundMode('image')
    try {
      await ipcClient.assetsAllow(val)
    } catch {
      // ignore in mock/test
    }
  } else {
    themeStore.setBackgroundMode('none')
  }
}

function clearBackground() {
  themeStore.setBackgroundPath('')
  themeStore.setBackgroundMode('none')
}
</script>

<template>
  <MSection :title="t('settingsAppearance.sections.background')" icon="i-lucide-image">
    <div class="flex flex-col">
      <!-- Background Path & Browse -->
      <MSettingRow :label="t('settingsAppearance.background.path')">
        <div v-if="themeStore.backgroundPath" class="flex items-center gap-2">
          <div
            class="flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-sunken border border-line text-xs font-mono text-fg max-w-[200px] truncate"
          >
            <span class="i-lucide-image w-3.5 h-3.5 text-primary shrink-0" />
            <span class="truncate">{{ getFileName(themeStore.backgroundPath) }}</span>
            <MHelpTooltip :text="themeStore.backgroundPath" />
          </div>
          <MButton
            variant="tonal"
            size="sm"
            icon="i-lucide-refresh-cw"
            @click="onPickBackgroundImage"
          >
            {{ t('settingsAppearance.background.change') }}
          </MButton>
          <MIconButton
            variant="text"
            size="sm"
            icon="i-lucide-x"
            class="text-danger hover:bg-danger-soft p-1.5"
            :title="t('common.clear')"
            @click="clearBackground"
          />
        </div>
        <MButton
          v-else
          variant="primary"
          size="sm"
          icon="i-lucide-image-plus"
          data-testid="pick-bg-btn"
          @click="onPickBackgroundImage"
        >
          {{ t('settingsAppearance.background.browse') }}
        </MButton>
      </MSettingRow>

      <!-- Blur Slider -->
      <MSettingRow
        v-if="themeStore.backgroundMode !== 'none'"
        :label="t('settingsAppearance.background.blur')"
      >
        <div class="flex items-center gap-3 w-64">
          <div class="flex-1">
            <MSlider
              :model-value="themeStore.backgroundBlur"
              :min="0"
              :max="32"
              :step="1"
              @update:model-value="themeStore.setBackgroundBlur($event)"
            />
          </div>
          <span class="text-xs font-mono font-semibold text-primary w-12 text-right">
            {{ t('settingsAppearance.background.blurValue', { val: themeStore.backgroundBlur }) }}
          </span>
        </div>
      </MSettingRow>

      <!-- Opacity Slider -->
      <MSettingRow
        v-if="themeStore.backgroundMode !== 'none'"
        :label="t('settingsAppearance.background.opacity')"
      >
        <div class="flex items-center gap-3 w-64">
          <div class="flex-1">
            <MSlider
              :model-value="themeStore.backgroundOpacity"
              :min="0"
              :max="1"
              :step="0.05"
              @update:model-value="themeStore.setBackgroundOpacity($event)"
            />
          </div>
          <span class="text-xs font-mono font-semibold text-primary w-12 text-right">
            {{
              t('settingsAppearance.background.opacityValue', {
                val: Math.round(themeStore.backgroundOpacity * 100),
              })
            }}
          </span>
        </div>
      </MSettingRow>

      <!-- Preview Box -->
      <div class="pt-4 border-t border-line mt-2 space-y-2">
        <label class="text-xs font-medium text-fg-muted">
          {{ t('settingsAppearance.background.preview') }}
        </label>
        <div
          class="relative w-full h-44 rounded-xl border border-line bg-sunken/30 overflow-hidden flex items-center justify-center"
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
          <template
            v-else-if="
              (themeStore.backgroundMode === 'image' ||
                themeStore.backgroundMode === 'slideshow') &&
              previewMediaSrc
            "
          >
            <img
              data-testid="bg-preview-img"
              class="absolute inset-0 w-full h-full object-cover pointer-events-none"
              :src="previewMediaSrc"
              :style="{
                filter: `blur(${themeStore.backgroundBlur}px)`,
                opacity: themeStore.backgroundOpacity,
              }"
              alt="Background preview"
              loading="lazy"
              decoding="async"
            />
          </template>
          <div v-else class="text-xs text-fg-muted flex items-center gap-1.5 z-10">
            <span class="i-lucide-image-off w-4 h-4" />
            <span>{{ t('settingsAppearance.background.previewEmpty') }}</span>
          </div>

          <!-- Foreground Card Sample for Contrast (Only show when background media is loaded) -->
          <div
            v-if="previewMediaSrc && themeStore.backgroundMode !== 'none'"
            class="relative z-10 px-4 py-2.5 rounded-lg border border-line bg-raised/80 backdrop-blur-md shadow-md text-center"
          >
            <p class="text-xs font-semibold text-fg">MCLR Preview</p>
            <p class="text-[11px] text-fg-muted">Sample UI Foreground Content</p>
          </div>
        </div>
      </div>
    </div>
  </MSection>
</template>

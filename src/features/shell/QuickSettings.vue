<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useThemeStore } from '../../stores/theme'
import type { ThemeMode, MaterialMode } from '../../ui/theme'
import { MPopover, MSegmented, SEED_PRESETS } from '../../ui'

const { t } = useI18n()
const themeStore = useThemeStore()
const open = ref(false)

const modeOptions = computed(() => [
  { label: t('shell.mode.light'), value: 'light', icon: 'i-lucide-sun' },
  { label: t('shell.mode.dark'), value: 'dark', icon: 'i-lucide-moon' },
  { label: t('shell.mode.system'), value: 'system', icon: 'i-lucide-monitor' },
])

const materialOptions = computed(() => [
  { label: t('shell.material.flat'), value: 'flat', icon: 'i-lucide-square' },
  { label: t('shell.material.glass'), value: 'glass', icon: 'i-lucide-sparkles' },
])
</script>

<template>
  <MPopover v-model:open="open" placement="bottom-end">
    <template #trigger>
      <button
        type="button"
        data-testid="quick-settings-btn"
        :title="t('shell.quick.title')"
        class="w-7 h-7 flex items-center justify-center rounded hover:bg-sunken text-fg-muted hover:text-fg cursor-pointer transition-colors"
        @click="open = !open"
      >
        <span class="i-lucide-swatch-book w-3.5 h-3.5" />
      </button>
    </template>

    <div class="w-64 flex flex-col gap-3 p-1">
      <div class="flex items-center justify-between">
        <span class="text-xs font-semibold text-fg">{{ t('shell.quick.title') }}</span>
      </div>

      <!-- Mode -->
      <div class="flex flex-col gap-1.5 items-start">
        <span class="text-xs text-fg-muted">{{ t('shell.quick.mode') }}</span>
        <MSegmented
          size="sm"
          :model-value="themeStore.mode"
          :options="modeOptions"
          @update:model-value="themeStore.setMode($event as ThemeMode)"
        />
      </div>

      <!-- Material -->
      <div class="flex flex-col gap-1.5 items-start">
        <span class="text-xs text-fg-muted">{{ t('shell.quick.material') }}</span>
        <MSegmented
          size="sm"
          :model-value="themeStore.material"
          :options="materialOptions"
          @update:model-value="themeStore.setMaterial($event as MaterialMode)"
        />
      </div>

      <!-- Accent Presets -->
      <div class="flex flex-col gap-1.5">
        <span class="text-xs text-fg-muted">{{ t('shell.quick.accent') }}</span>
        <div class="grid grid-cols-4 gap-2">
          <button
            v-for="preset in SEED_PRESETS"
            :key="preset.key"
            type="button"
            :title="t('theme.preset.' + preset.key)"
            class="w-6 h-6 rounded-full flex items-center justify-center cursor-pointer transition-transform hover:scale-110 active:scale-95 border border-line"
            :style="{ backgroundColor: preset.hex }"
            @click="themeStore.setSeed(preset.hex)"
          >
            <span
              v-if="themeStore.seed.toLowerCase() === preset.hex.toLowerCase()"
              class="i-lucide-check w-3.5 h-3.5 text-white drop-shadow"
            />
          </button>
        </div>
      </div>
    </div>
  </MPopover>
</template>

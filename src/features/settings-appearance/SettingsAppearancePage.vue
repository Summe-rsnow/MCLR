<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useThemeStore } from '../../stores/theme'
import { MSlider, MSection, MSettingRow } from '../../ui'
import AppearanceThemeSection from './components/AppearanceThemeSection.vue'
import AppearanceBackgroundSection from './components/AppearanceBackgroundSection.vue'

const { t } = useI18n()
const themeStore = useThemeStore()

const localScale = ref(themeStore.scale)
watch(
  () => themeStore.scale,
  (val) => {
    localScale.value = val
  },
)

function onScaleInput(val: number) {
  localScale.value = val
}

function onScaleCommit(val: number) {
  themeStore.setScale(val)
}
</script>

<template>
  <div class="space-y-6 pb-12 select-none">
    <!-- Header title/subtitle preserved for test text assertion -->
    <div class="sr-only">
      <h1>{{ t('settingsAppearance.title') }}</h1>
      <p>{{ t('settingsAppearance.subtitle') }}</p>
    </div>

    <!-- 1. Theme & Material Section -->
    <AppearanceThemeSection />

    <!-- 2. UI Scale Section -->
    <MSection :title="t('settingsAppearance.sections.scale')" icon="i-lucide-maximize">
      <div class="flex flex-col">
        <!-- UI Scale Slider -->
        <MSettingRow :label="t('settingsAppearance.typography.scale')">
          <div class="flex items-center gap-3 w-64">
            <div class="flex-1">
              <MSlider
                :model-value="localScale"
                :min="0.8"
                :max="1.25"
                :step="0.05"
                @update:model-value="onScaleInput"
                @change="onScaleCommit"
              />
            </div>
            <span class="text-xs font-mono font-semibold text-primary w-12 text-right">
              {{
                t('settingsAppearance.typography.scaleValue', { val: Math.round(localScale * 100) })
              }}
            </span>
          </div>
        </MSettingRow>
      </div>
    </MSection>

    <!-- 3. Background Section -->
    <AppearanceBackgroundSection />
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useThemeStore, PERFORMANCE_TIERS, type PerformanceTier } from '../../../stores/theme'
import type { MotionMode } from '../../../stores/theme'
import { type ThemeMode, type MaterialMode } from '../../../ui/theme'
import {
  MCard,
  MButton,
  MSegmented,
  MSlider,
  MSwitch,
  MColorPicker,
  MSection,
  MSettingRow,
  MHelpTooltip,
} from '../../../ui'

const { t } = useI18n()
/** `/dev/ui` is only registered by the dev build. */
const isDevBuild = import.meta.env.DEV
const router = useRouter()
const themeStore = useThemeStore()

const modeOptions = computed(() => [
  { label: t('settingsAppearance.theme.modeLight'), value: 'light', icon: 'i-lucide-sun' },
  { label: t('settingsAppearance.theme.modeDark'), value: 'dark', icon: 'i-lucide-moon' },
  { label: t('settingsAppearance.theme.modeSystem'), value: 'system', icon: 'i-lucide-laptop' },
])

const materialOptions = computed(() => [
  { label: t('settingsAppearance.theme.materialFlat'), value: 'flat', icon: 'i-lucide-square' },
  { label: t('settingsAppearance.theme.materialGlass'), value: 'glass', icon: 'i-lucide-sparkles' },
])

const motionOptions = computed(() => [
  // Cheapest first, matching the tier slider's left-to-right cost order.
  { label: t('settingsAppearance.motion.off'), value: 'off' },
  { label: t('settingsAppearance.motion.reduced'), value: 'reduced' },
  { label: t('settingsAppearance.motion.full'), value: 'full' },
])

const tierIndex = computed(() => PERFORMANCE_TIERS.indexOf(themeStore.performanceTier))

const tierLabels = computed(() => [
  t('settingsAppearance.performance.performance'),
  t('settingsAppearance.performance.balanced'),
  t('settingsAppearance.performance.quality'),
])

/** What the active tier actually turns on, shown as the row hint. */
const tierSummaries = computed<Record<PerformanceTier, string>>(() => ({
  performance: t('settingsAppearance.performance.performanceSummary'),
  balanced: t('settingsAppearance.performance.balancedSummary'),
  quality: t('settingsAppearance.performance.qualitySummary'),
}))

function onTierSelect(index: number) {
  const tier = PERFORMANCE_TIERS[index]
  if (tier) themeStore.setPerformanceTier(tier)
}

function navigateToDevUi() {
  if (!isDevBuild) return
  if (router && typeof router.push === 'function') {
    router.push('/dev/ui')
  }
}
</script>

<template>
  <MSection :title="t('settingsAppearance.sections.theme')" icon="i-lucide-palette">
    <div class="flex flex-col">
      <!-- Seed Color -->
      <MSettingRow :label="t('settingsAppearance.theme.seedColor')">
        <MColorPicker
          :model-value="themeStore.seed"
          @update:model-value="themeStore.setSeed($event)"
        />
      </MSettingRow>

      <!-- Mode -->
      <MSettingRow :label="t('settingsAppearance.theme.mode')" stretch>
        <MSegmented
          block
          :model-value="themeStore.mode"
          :options="modeOptions"
          @update:model-value="themeStore.setMode($event as ThemeMode)"
        />
      </MSettingRow>

      <!-- Performance tier: one slider drives material, motion and splash -->
      <MSettingRow :label="t('settingsAppearance.performance.label')" stretch>
        <template #label>
          <span>{{ t('settingsAppearance.performance.label') }}</span>
          <MHelpTooltip :text="t('settingsAppearance.performance.desc')" />
        </template>
        <template #hint>{{ tierSummaries[themeStore.performanceTier] }}</template>
        <div class="w-full max-w-sm">
          <MSlider
            data-testid="perf-tier-slider"
            :model-value="tierIndex"
            :min="0"
            :max="PERFORMANCE_TIERS.length - 1"
            :step="1"
            :labels="tierLabels"
            @update:model-value="onTierSelect"
          />
        </div>
      </MSettingRow>

      <!-- Material -->
      <MSettingRow :label="t('settingsAppearance.theme.material')" stretch>
        <template #label>
          <span>{{ t('settingsAppearance.theme.material') }}</span>
          <MHelpTooltip :text="t('settingsAppearance.theme.materialDesc')" />
        </template>
        <MSegmented
          block
          :model-value="themeStore.material"
          :options="materialOptions"
          @update:model-value="themeStore.setMaterial($event as MaterialMode)"
        />
      </MSettingRow>

      <!-- Motion -->
      <MSettingRow :label="t('settingsAppearance.motion.label')" stretch>
        <template #label>
          <span>{{ t('settingsAppearance.motion.label') }}</span>
          <MHelpTooltip :text="t('settingsAppearance.motion.desc')" />
        </template>
        <MSegmented
          block
          :model-value="themeStore.motion"
          :options="motionOptions"
          @update:model-value="themeStore.setMotion($event as MotionMode)"
        />
      </MSettingRow>

      <!-- Startup Splash -->
      <MSettingRow :label="t('settingsAppearance.startupSplash')">
        <template #label>
          <span>{{ t('settingsAppearance.startupSplash') }}</span>
          <MHelpTooltip :text="t('settingsAppearance.startupSplashDesc')" />
        </template>
        <MSwitch
          :model-value="themeStore.startupSplash"
          @update:model-value="themeStore.setStartupSplash($event)"
        />
      </MSettingRow>

      <!-- Live Preview Box -->
      <div class="pt-4 border-t border-line mt-2">
        <div class="flex items-center justify-between mb-2.5">
          <div class="text-xs font-medium text-fg-muted">
            {{ t('settingsAppearance.theme.livePreview') }}
          </div>
          <!-- Hidden in release builds: `/dev/ui` is not registered there. -->
          <MButton
            v-if="isDevBuild"
            variant="text"
            size="xs"
            icon-right="i-lucide-arrow-right"
            @click="navigateToDevUi"
          >
            {{ t('settingsAppearance.theme.viewComponents') }}
          </MButton>
        </div>
        <div class="p-4 rounded-xl bg-sunken border border-line flex flex-wrap items-center gap-4">
          <MCard variant="raised" padding="sm" class="flex items-center gap-2">
            <span class="i-lucide-layers w-4 h-4 text-primary" />
            <span class="text-xs font-medium">{{
              t('settingsAppearance.theme.preview.raisedCard')
            }}</span>
          </MCard>
          <div class="m-well px-3 py-2 rounded-lg text-xs font-mono text-fg-muted">
            {{ t('settingsAppearance.theme.preview.wellContainer') }}
          </div>
          <MButton variant="primary" size="sm" icon="i-lucide-sparkles">
            {{ t('settingsAppearance.theme.preview.primaryButton') }}
          </MButton>
        </div>
      </div>
    </div>
  </MSection>
</template>

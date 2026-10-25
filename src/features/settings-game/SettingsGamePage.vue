<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  ipcClient,
  type LaunchSettings,
} from '../../ipc/client'
import { useToast } from '../../ui/useToast'
import {
  MInput,
  MSelect,
  MSlider,
  MSwitch,
  MSection,
  MSettingRow,
  MHelpTooltip,
} from '../../ui'

const { t } = useI18n()
const toast = useToast()

const isSaving = ref(false)

// Launch settings form state
const launchSettings = ref<LaunchSettings>({
  ramMode: 'auto',
  ramCustomMb: 4096,
  jvmArgs: '',
  gameArgs: '',
  windowTitle: '',
  fullscreen: false,
  width: 854,
  height: 480,
  preferredIpStack: 'auto',
  processPriority: 'normal',
  closeLauncherOnStart: 'keep',
  disableCrashAnalysis: false,
})

onMounted(async () => {
  try {
    const s = await ipcClient.settingsGet()
    if (s.launch) {
      launchSettings.value = { ...launchSettings.value, ...s.launch }
    }
  } catch (err) {
    console.warn('Failed to load settings in SettingsGamePage:', err)
  }
})

async function saveLaunchSettings() {
  isSaving.value = true
  try {
    await ipcClient.settingsPatch({
      launch: launchSettings.value,
    })
    toast.success(t('common.saved'))
  } catch (err) {
    toast.danger(`${t('common.error')}: ${String(err)}`)
  } finally {
    isSaving.value = false
  }
}
</script>

<template>
  <div class="space-y-6 pb-12 select-none">
    <!-- Header title preserved for test text assertion -->
    <div class="sr-only">
      <h1>{{ t('settingsGame.title') }}</h1>
    </div>

    <!-- Launch Settings -->
    <div data-testid="tab-launch" class="flex flex-col gap-6">
      <!-- RAM Section -->
      <MSection
        :title="t('settingsGame.launch.ramSection')"
        icon="i-lucide-cpu"
      >
        <template #title>
          <span>{{ t('settingsGame.launch.ramSection') }}</span>
          <MHelpTooltip :text="t('settingsGame.launch.ramModeAuto')" />
        </template>
        <div class="flex flex-col">
          <MSettingRow
            :label="t('settingsGame.launch.ramMode')"
            :hint="launchSettings.ramMode === 'auto' ? t('settingsGame.launch.ramModeAuto') : t('settingsGame.launch.ramModeCustom')"
          >
            <MSelect
              v-model="launchSettings.ramMode"
              :options="[
                { label: t('settingsGame.launch.ramModeAuto'), value: 'auto' },
                { label: t('settingsGame.launch.ramModeCustom'), value: 'custom' },
              ]"
              size="sm"
              class="w-48"
              @update:model-value="saveLaunchSettings"
            />
          </MSettingRow>

          <MSettingRow
            v-if="launchSettings.ramMode === 'custom'"
            :label="t('settingsGame.launch.ramCustomMb')"
            :hint="t('settingsGame.launch.ramCustomMbUnit', { mb: launchSettings.ramCustomMb })"
          >
            <div class="w-64 flex items-center gap-4">
              <MSlider
                v-model="launchSettings.ramCustomMb"
                :min="1024"
                :max="32768"
                :step="512"
                class="flex-1"
                @change="saveLaunchSettings"
              />
              <span class="text-xs font-mono w-16 text-right text-fg-muted">
                {{ (launchSettings.ramCustomMb / 1024).toFixed(1) }} GB
              </span>
            </div>
          </MSettingRow>
        </div>
      </MSection>

      <!-- Window Section -->
      <MSection
        :title="t('settingsGame.launch.windowSection')"
        icon="i-lucide-monitor"
      >
        <div class="flex flex-col">
          <MSettingRow
            :label="t('settingsGame.launch.fullscreen')"
          >
            <MSwitch
              v-model="launchSettings.fullscreen"
              @update:model-value="saveLaunchSettings"
            />
          </MSettingRow>

          <MSettingRow
            v-if="!launchSettings.fullscreen"
            :label="t('settingsGame.launch.resolution')"
          >
            <div class="flex items-center gap-2">
              <MInput
                v-model.number="launchSettings.width"
                type="number"
                size="sm"
                class="w-24"
                :placeholder="t('settingsGame.launch.resolutionWidth')"
                @blur="saveLaunchSettings"
              />
              <span class="text-fg-muted text-sm">×</span>
              <MInput
                v-model.number="launchSettings.height"
                type="number"
                size="sm"
                class="w-24"
                :placeholder="t('settingsGame.launch.resolutionHeight')"
                @blur="saveLaunchSettings"
              />
            </div>
          </MSettingRow>

          <MSettingRow
            :label="t('settingsGame.launch.windowTitle')"
          >
            <MInput
              v-model="launchSettings.windowTitle"
              size="sm"
              class="w-64"
              :placeholder="t('settingsGame.launch.windowTitlePlaceholder')"
              @blur="saveLaunchSettings"
            />
          </MSettingRow>
        </div>
      </MSection>

      <!-- Advanced Args Section -->
      <MSection
        :title="t('settingsGame.launch.argsSection')"
        icon="i-lucide-terminal"
      >
        <div class="flex flex-col">
          <MSettingRow
            :label="t('settingsGame.launch.jvmArgs')"
          >
            <MInput
              v-model="launchSettings.jvmArgs"
              size="sm"
              class="w-full sm:w-80"
              :placeholder="t('settingsGame.launch.jvmArgsPlaceholder')"
              @blur="saveLaunchSettings"
            />
          </MSettingRow>

          <MSettingRow
            :label="t('settingsGame.launch.gameArgs')"
          >
            <MInput
              v-model="launchSettings.gameArgs"
              size="sm"
              class="w-full sm:w-80"
              :placeholder="t('settingsGame.launch.gameArgsPlaceholder')"
              @blur="saveLaunchSettings"
            />
          </MSettingRow>
        </div>
      </MSection>

      <!-- Runtime Section -->
      <MSection
        :title="t('settingsGame.launch.runtimeSection')"
        icon="i-lucide-activity"
      >
        <div class="flex flex-col">
          <MSettingRow
            :label="t('settingsGame.launch.preferredIpStack')"
          >
            <template #label>
              <span>{{ t('settingsGame.launch.preferredIpStack') }}</span>
              <MHelpTooltip text="配置 Java 虚拟机的网络协议栈偏好，可根据网络环境强制优先 IPv4 或 IPv6" />
            </template>
            <MSelect
              v-model="launchSettings.preferredIpStack"
              :options="[
                { label: t('settingsGame.launch.ipStackAuto'), value: 'auto' },
                { label: t('settingsGame.launch.ipStackV4'), value: 'v4' },
                { label: t('settingsGame.launch.ipStackV6'), value: 'v6' },
              ]"
              size="sm"
              class="w-48"
              @update:model-value="saveLaunchSettings"
            />
          </MSettingRow>

          <MSettingRow
            :label="t('settingsGame.launch.processPriority')"
          >
            <MSelect
              v-model="launchSettings.processPriority"
              :options="[
                { label: t('settingsGame.launch.priorityNormal'), value: 'normal' },
                { label: t('settingsGame.launch.priorityAboveNormal'), value: 'above-normal' },
                { label: t('settingsGame.launch.priorityHigh'), value: 'high' },
              ]"
              size="sm"
              class="w-48"
              @update:model-value="saveLaunchSettings"
            />
          </MSettingRow>

          <MSettingRow
            :label="t('settingsGame.launch.closeLauncher')"
          >
            <MSelect
              v-model="launchSettings.closeLauncherOnStart"
              :options="[
                { label: t('settingsGame.launch.closeLauncherKeep'), value: 'keep' },
                { label: t('settingsGame.launch.closeLauncherHide'), value: 'hide' },
                { label: t('settingsGame.launch.closeLauncherExit'), value: 'exit' },
              ]"
              size="sm"
              class="w-48"
              @update:model-value="saveLaunchSettings"
            />
          </MSettingRow>

          <MSettingRow
            :label="t('settingsGame.launch.disableCrashAnalysis')"
          >
            <MSwitch
              v-model="launchSettings.disableCrashAnalysis"
              @update:model-value="saveLaunchSettings"
            />
          </MSettingRow>
        </div>
      </MSection>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { ipcClient, type LaunchSettings } from '../../ipc/client'
import { useToast } from '../../ui/useToast'
import {
  MInput,
  MSelect,
  MSlider,
  MSwitch,
  MSection,
  MSettingRow,
  MHelpTooltip,
  MButton,
  MTextarea,
} from '../../ui'

const { t } = useI18n()
const toast = useToast()

const isSaving = ref(false)

// Launch settings form state
/** Balanced JVM defaults, matching PCL's recommended JVM argument head. */
const DEFAULT_JVM_ARGS =
  '-XX:+UseG1GC -XX:+UseAdaptiveSizePolicy -XX:-OmitStackTraceInFastThrow ' +
  '-Djdk.lang.Process.allowAmbiguousCommands=true -Dfml.ignoreInvalidMinecraftCertificates=True ' +
  '-Dfml.ignorePatchDiscrepancies=True -Dlog4j2.formatMsgNoLookups=true'

// Mirrors `launch::DEFAULT_JVM_ARGS` in mclr-core; keep both strings identical.
const launchSettings = ref<LaunchSettings>({
  ramMode: 'auto',
  ramCustomMb: 4096,
  jvmArgs: DEFAULT_JVM_ARGS,
  gameArgs: '',
  windowTitle: '',
  fullscreen: false,
  width: 854,
  height: 480,
  preferredIpStack: 'auto',
  processPriority: 'normal',
  closeLauncherOnStart: 'keep',
  crashAnalysis: true,
  renderer: 'default',
  preLaunchCommand: '',
  useJavaExe: true,
  requireGpuHighPerformance: true,
  lockMemory: true,
  legacyFix: true,
  lwjglUnsafeAgent: true,
})

function resetJvmArgs() {
  launchSettings.value.jvmArgs = ''
  void saveLaunchSettings()
}

function applyDefaultJvmArgs() {
  launchSettings.value.jvmArgs = DEFAULT_JVM_ARGS
  void saveLaunchSettings()
}

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
      <MSection :title="t('settingsGame.launch.ramSection')" icon="i-lucide-cpu">
        <template #title>
          <span>{{ t('settingsGame.launch.ramSection') }}</span>
          <MHelpTooltip :text="t('settingsGame.launch.ramModeAuto')" />
        </template>
        <div class="flex flex-col">
          <MSettingRow
            :label="t('settingsGame.launch.ramMode')"
            :hint="
              launchSettings.ramMode === 'auto'
                ? t('settingsGame.launch.ramModeAuto')
                : t('settingsGame.launch.ramModeCustom')
            "
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
      <MSection :title="t('settingsGame.launch.windowSection')" icon="i-lucide-monitor">
        <div class="flex flex-col">
          <MSettingRow :label="t('settingsGame.launch.fullscreen')">
            <MSwitch v-model="launchSettings.fullscreen" @update:model-value="saveLaunchSettings" />
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

          <MSettingRow :label="t('settingsGame.launch.windowTitle')">
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

      <!-- Advanced Launch Options (PCL parity) -->
      <MSection
        :title="t('settingsGame.launch.advancedSection')"
        icon="i-lucide-sliders-horizontal"
      >
        <div class="flex flex-col">
          <MSettingRow :label="t('settingsGame.launch.renderer')">
            <template #label>
              <span>{{ t('settingsGame.launch.renderer') }}</span>
              <MHelpTooltip :text="t('settingsGame.launch.rendererHint')" />
            </template>
            <MSelect
              v-model="launchSettings.renderer"
              :options="[
                { label: t('settingsGame.launch.rendererDefault'), value: 'default' },
                { label: t('settingsGame.launch.rendererSoftware'), value: 'software' },
                { label: t('settingsGame.launch.rendererD3d12'), value: 'd3d12' },
                { label: t('settingsGame.launch.rendererVulkan'), value: 'vulkan' },
              ]"
              size="sm"
              class="w-56"
              @update:model-value="saveLaunchSettings"
            />
          </MSettingRow>

          <div class="flex flex-col gap-2 py-3">
            <div class="flex items-center justify-between gap-2">
              <div class="flex items-center gap-1.5">
                <span class="text-sm font-medium text-fg">
                  {{ t('settingsGame.launch.jvmArgsHead') }}
                </span>
                <MHelpTooltip :text="t('settingsGame.launch.jvmArgsHeadHint')" />
              </div>
              <div class="flex items-center gap-1.5">
                <MButton
                  variant="text"
                  size="xs"
                  icon="i-lucide-sparkles"
                  @click="applyDefaultJvmArgs"
                >
                  {{ t('settingsGame.launch.jvmArgsDefault') }}
                </MButton>
                <MButton variant="text" size="xs" icon="i-lucide-rotate-ccw" @click="resetJvmArgs">
                  {{ t('settingsGame.launch.jvmArgsReset') }}
                </MButton>
              </div>
            </div>
            <MTextarea
              v-model="launchSettings.jvmArgs"
              :rows="4"
              :placeholder="t('settingsGame.launch.jvmArgsPlaceholder')"
              class="font-mono text-xs"
              @blur="saveLaunchSettings"
            />
          </div>

          <MSettingRow :label="t('settingsGame.launch.gameArgsTail')">
            <template #label>
              <span>{{ t('settingsGame.launch.gameArgsTail') }}</span>
              <MHelpTooltip :text="t('settingsGame.launch.gameArgsTailHint')" />
            </template>
            <MInput
              v-model="launchSettings.gameArgs"
              size="sm"
              class="w-full"
              :placeholder="t('settingsGame.launch.gameArgsPlaceholder')"
              @blur="saveLaunchSettings"
            />
          </MSettingRow>

          <MSettingRow :label="t('settingsGame.launch.preLaunchCommand')">
            <template #label>
              <span>{{ t('settingsGame.launch.preLaunchCommand') }}</span>
              <MHelpTooltip :text="t('settingsGame.launch.preLaunchCommandHint')" />
            </template>
            <MInput
              v-model="launchSettings.preLaunchCommand"
              size="sm"
              class="w-full font-mono text-xs"
              :placeholder="t('settingsGame.launch.preLaunchCommandPlaceholder')"
              @blur="saveLaunchSettings"
            />
          </MSettingRow>

          <MSettingRow :label="t('settingsGame.launch.requireGpu')">
            <template #label>
              <span>{{ t('settingsGame.launch.requireGpu') }}</span>
              <MHelpTooltip :text="t('settingsGame.launch.requireGpuHint')" />
            </template>
            <MSwitch
              v-model="launchSettings.requireGpuHighPerformance"
              @update:model-value="saveLaunchSettings"
            />
          </MSettingRow>

          <MSettingRow :label="t('settingsGame.launch.useJavaExe')">
            <template #label>
              <span>{{ t('settingsGame.launch.useJavaExe') }}</span>
              <MHelpTooltip :text="t('settingsGame.launch.useJavaExeHint')" />
            </template>
            <MSwitch v-model="launchSettings.useJavaExe" @update:model-value="saveLaunchSettings" />
          </MSettingRow>

          <MSettingRow :label="t('settingsGame.launch.lockMemory')">
            <template #label>
              <span>{{ t('settingsGame.launch.lockMemory') }}</span>
              <MHelpTooltip :text="t('settingsGame.launch.lockMemoryHint')" />
            </template>
            <MSwitch v-model="launchSettings.lockMemory" @update:model-value="saveLaunchSettings" />
          </MSettingRow>

          <MSettingRow :label="t('settingsGame.launch.legacyFix')">
            <template #label>
              <span>{{ t('settingsGame.launch.legacyFix') }}</span>
              <MHelpTooltip :text="t('settingsGame.launch.legacyFixHint')" />
            </template>
            <MSwitch v-model="launchSettings.legacyFix" @update:model-value="saveLaunchSettings" />
          </MSettingRow>

          <MSettingRow :label="t('settingsGame.launch.lwjglUnsafeAgent')">
            <template #label>
              <span>{{ t('settingsGame.launch.lwjglUnsafeAgent') }}</span>
              <MHelpTooltip :text="t('settingsGame.launch.lwjglUnsafeAgentHint')" />
            </template>
            <MSwitch
              v-model="launchSettings.lwjglUnsafeAgent"
              @update:model-value="saveLaunchSettings"
            />
          </MSettingRow>

          <MSettingRow :label="t('settingsGame.launch.crashAnalysis')">
            <template #label>
              <span>{{ t('settingsGame.launch.crashAnalysis') }}</span>
              <MHelpTooltip :text="t('settingsGame.launch.crashAnalysisHint')" />
            </template>
            <MSwitch
              v-model="launchSettings.crashAnalysis"
              @update:model-value="saveLaunchSettings"
            />
          </MSettingRow>
        </div>
      </MSection>

      <!-- Runtime Section -->
      <MSection :title="t('settingsGame.launch.runtimeSection')" icon="i-lucide-activity">
        <div class="flex flex-col">
          <MSettingRow :label="t('settingsGame.launch.preferredIpStack')">
            <template #label>
              <span>{{ t('settingsGame.launch.preferredIpStack') }}</span>
              <MHelpTooltip :text="t('settingsGame.launch.preferredIpStackHint')" />
            </template>
            <MSelect
              v-model="launchSettings.preferredIpStack"
              :options="[
                { label: t('settingsGame.launch.ipStackAuto'), value: 'auto' },
                { label: t('settingsGame.launch.ipStackV4'), value: 'ipv4' },
                { label: t('settingsGame.launch.ipStackV6'), value: 'ipv6' },
              ]"
              size="sm"
              class="w-48"
              @update:model-value="saveLaunchSettings"
            />
          </MSettingRow>

          <MSettingRow :label="t('settingsGame.launch.processPriority')">
            <MSelect
              v-model="launchSettings.processPriority"
              :options="[
                { label: t('settingsGame.launch.priorityNormal'), value: 'normal' },
                { label: t('settingsGame.launch.priorityAboveNormal'), value: 'aboveNormal' },
                { label: t('settingsGame.launch.priorityHigh'), value: 'high' },
              ]"
              size="sm"
              class="w-48"
              @update:model-value="saveLaunchSettings"
            />
          </MSettingRow>

          <MSettingRow :label="t('settingsGame.launch.closeLauncher')">
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
        </div>
      </MSection>
    </div>
  </div>
</template>

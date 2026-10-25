<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute } from 'vue-router'
import {
  ipcClient,
  type JavaInstallation,
  type LaunchSettings,
  type JavaSettings,
} from '../../ipc/client'
import { useToast } from '../../ui/useToast'
import {
  MButton,
  MInput,
  MSelect,
  MSlider,
  MSwitch,
  MBadge,
  MSection,
  MSettingRow,
} from '../../ui'

const { t } = useI18n()
const toast = useToast()
const route = useRoute()

const activeTab = computed<'launch' | 'java'>(() => {
  const tab = route?.query?.tab as string
  if (tab === 'java') return 'java'
  return 'launch'
})

const isSaving = ref(false)
const isScanning = ref(false)
const downloadingComponent = ref<string | null>(null)
const customJavaInput = ref('')

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

// Java settings and discovery state
const javaSettings = ref<JavaSettings>({
  custom: [],
  select: 'auto',
  preferJdk: false,
})
const javaInstallations = ref<JavaInstallation[]>([])
const hasScannedJava = ref(false)

async function ensureJavaScanned(force = false) {
  if (hasScannedJava.value && !force) return
  hasScannedJava.value = true
  try {
    javaInstallations.value = await ipcClient.javaScan(force)
  } catch (err) {
    console.warn('Failed to scan Java in SettingsGamePage:', err)
  }
}

watch(
  activeTab,
  (tab) => {
    if (tab === 'java') {
      ensureJavaScanned()
    }
  },
  { immediate: true },
)

onMounted(async () => {
  try {
    const s = await ipcClient.settingsGet()
    if (s.launch) {
      launchSettings.value = { ...launchSettings.value, ...s.launch }
    }
    if (s.java) {
      javaSettings.value = { ...javaSettings.value, ...s.java }
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

async function saveJavaSettings() {
  isSaving.value = true
  try {
    await ipcClient.settingsPatch({
      java: javaSettings.value,
    })
    toast.success(t('common.saved'))
  } catch (err) {
    toast.danger(`${t('common.error')}: ${String(err)}`)
  } finally {
    isSaving.value = false
  }
}

async function onScanJava() {
  isScanning.value = true
  try {
    javaInstallations.value = await ipcClient.javaScan(true)
    toast.success(t('settingsGame.java.scanComplete', { count: javaInstallations.value.length }))
  } catch (err) {
    toast.danger(`${t('common.error')}: ${String(err)}`)
  } finally {
    isScanning.value = false
  }
}

async function onAddCustomJava() {
  if (!customJavaInput.value.trim()) return
  const path = customJavaInput.value.trim()
  if (javaSettings.value.custom.includes(path)) {
    toast.warning(t('settingsGame.java.alreadyExists'))
    return
  }

  javaSettings.value.custom.push(path)
  customJavaInput.value = ''
  await saveJavaSettings()
  await onScanJava()
}

async function onRemoveCustomJava(path: string) {
  javaSettings.value.custom = javaSettings.value.custom.filter((p) => p !== path)
  await saveJavaSettings()
  await onScanJava()
}

async function onDownloadMojangJava(component: string) {
  downloadingComponent.value = component
  try {
    const taskId = await ipcClient.javaDownload(component)
    toast.info(`${t('settingsGame.java.downloadStarted')} (${taskId})`)
  } catch (err) {
    toast.danger(`${t('common.error')}: ${String(err)}`)
  } finally {
    downloadingComponent.value = null
  }
}
</script>

<template>
  <div class="space-y-6 pb-12 select-none">
    <!-- Header title preserved for test text assertion -->
    <div class="sr-only">
      <h1>{{ t('settingsGame.title') }}</h1>
    </div>

    <!-- Tab 1: Launch Settings -->
    <div v-if="activeTab === 'launch'" data-testid="tab-launch" class="flex flex-col gap-6">
      <!-- RAM Section -->
      <MSection
        :title="t('settingsGame.launch.ramSection')"
        icon="i-lucide-cpu"
      >
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
          >
            <div class="flex items-center gap-3 w-64">
              <div class="flex-1">
                <MSlider
                  v-model="launchSettings.ramCustomMb"
                  :min="1024"
                  :max="16384"
                  :step="512"
                  @change="saveLaunchSettings"
                />
              </div>
              <span class="text-xs font-mono font-bold text-primary w-20 text-right">
                {{ t('settingsGame.launch.ramCustomMbUnit', { mb: launchSettings.ramCustomMb }) }}
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
            :label="t('settingsGame.launch.resolution')"
          >
            <div class="flex items-center gap-2">
              <div class="w-28">
                <MInput
                  :model-value="String(launchSettings.width)"
                  type="number"
                  size="sm"
                  :placeholder="t('settingsGame.launch.resolutionWidth')"
                  @update:model-value="(val) => { launchSettings.width = Number(val) || 854; saveLaunchSettings(); }"
                />
              </div>
              <span class="text-xs text-fg-muted">×</span>
              <div class="w-28">
                <MInput
                  :model-value="String(launchSettings.height)"
                  type="number"
                  size="sm"
                  :placeholder="t('settingsGame.launch.resolutionHeight')"
                  @update:model-value="(val) => { launchSettings.height = Number(val) || 480; saveLaunchSettings(); }"
                />
              </div>
            </div>
          </MSettingRow>

          <MSettingRow
            :label="t('settingsGame.launch.windowTitle')"
          >
            <div class="w-64">
              <MInput
                v-model="launchSettings.windowTitle"
                :placeholder="t('settingsGame.launch.windowTitlePlaceholder')"
                size="sm"
                @blur="saveLaunchSettings"
              />
            </div>
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
            <div class="w-80">
              <MInput
                v-model="launchSettings.jvmArgs"
                :placeholder="t('settingsGame.launch.jvmArgsPlaceholder')"
                size="sm"
                @blur="saveLaunchSettings"
              />
            </div>
          </MSettingRow>

          <MSettingRow
            :label="t('settingsGame.launch.gameArgs')"
          >
            <div class="w-80">
              <MInput
                v-model="launchSettings.gameArgs"
                :placeholder="t('settingsGame.launch.gameArgsPlaceholder')"
                size="sm"
                @blur="saveLaunchSettings"
              />
            </div>
          </MSettingRow>
        </div>
      </MSection>

      <!-- Runtime Section -->
      <MSection
        :title="t('settingsGame.launch.runtimeSection')"
        icon="i-lucide-settings-2"
      >
        <div class="flex flex-col">
          <MSettingRow
            :label="t('settingsGame.launch.preferredIpStack')"
          >
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

          <MSettingRow
            :label="t('settingsGame.launch.processPriority')"
          >
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

    <!-- Tab 2: Java Settings -->
    <div v-if="activeTab === 'java'" data-testid="tab-java" class="flex flex-col gap-6">
      <!-- Java Selection & Scan Card -->
      <MSection
        :title="t('settingsGame.java.selectSection')"
        icon="i-lucide-coffee"
      >
        <template #actions>
          <MButton
            variant="tonal"
            size="sm"
            icon="i-lucide-refresh-cw"
            :loading="isScanning"
            @click="onScanJava"
          >
            {{ t('settingsGame.java.scanButton') }}
          </MButton>
        </template>

        <div class="flex flex-col">
          <MSettingRow
            :label="t('settingsGame.java.preferJdk')"
          >
            <MSwitch
              v-model="javaSettings.preferJdk"
              @update:model-value="saveJavaSettings"
            />
          </MSettingRow>

          <!-- Add Custom Java input -->
          <MSettingRow
            :label="t('settingsGame.java.addCustomButton')"
          >
            <div class="flex items-center gap-2 w-full sm:w-80">
              <MInput
                v-model="customJavaInput"
                :placeholder="t('settingsGame.java.addCustomPlaceholder')"
                size="sm"
                class="flex-1"
              />
              <MButton
                variant="primary"
                size="sm"
                icon="i-lucide-plus"
                @click="onAddCustomJava"
              >
                {{ t('settingsGame.java.addCustomButton') }}
              </MButton>
            </div>
          </MSettingRow>
        </div>
      </MSection>

      <!-- Mojang Java Download Card -->
      <MSection
        :title="t('settingsGame.java.downloadSection')"
        icon="i-lucide-download"
      >
        <div class="grid grid-cols-1 sm:grid-cols-3 gap-3">
          <MButton
            variant="tonal"
            size="sm"
            icon="i-lucide-download"
            :loading="downloadingComponent === 'java-runtime-delta'"
            @click="onDownloadMojangJava('java-runtime-delta')"
          >
            {{ t('settingsGame.java.downloadJava21') }}
          </MButton>
          <MButton
            variant="tonal"
            size="sm"
            icon="i-lucide-download"
            :loading="downloadingComponent === 'java-runtime-gamma'"
            @click="onDownloadMojangJava('java-runtime-gamma')"
          >
            {{ t('settingsGame.java.downloadJava17') }}
          </MButton>
          <MButton
            variant="tonal"
            size="sm"
            icon="i-lucide-download"
            :loading="downloadingComponent === 'jre-legacy'"
            @click="onDownloadMojangJava('jre-legacy')"
          >
            {{ t('settingsGame.java.downloadJava8') }}
          </MButton>
        </div>
      </MSection>

      <!-- Installed Java List -->
      <MSection
        :title="t('settingsGame.java.installedList', { count: javaInstallations.length })"
        icon="i-lucide-list"
      >
        <div v-if="javaInstallations.length === 0" class="text-sm text-fg-muted py-4 text-center">
          {{ t('settingsGame.java.emptyList') }}
        </div>

        <div v-else class="flex flex-col gap-2">
          <div
            v-for="item in javaInstallations"
            :key="item.path"
            class="flex items-center justify-between p-3 rounded-xl bg-sunken/40 border border-line"
          >
            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-2">
                <span class="font-bold text-sm text-fg">
                  {{ t('settingsGame.java.majorVersion', { major: item.major }) }}
                </span>
                <span class="text-xs text-fg-muted">({{ item.version }})</span>
                <MBadge :variant="item.isJdk ? 'primary' : 'neutral'">
                  {{ item.isJdk ? t('settingsGame.java.jdkBadge') : t('settingsGame.java.jreBadge') }}
                </MBadge>
                <MBadge variant="neutral">
                  {{ item.arch }}
                </MBadge>
              </div>
              <div class="text-xs text-fg-muted mt-1 truncate">
                {{ t('settingsGame.java.path', { path: item.path }) }}
              </div>
              <div class="text-xs text-fg-muted truncate">
                {{ t('settingsGame.java.vendor', { vendor: item.implementor }) }}
              </div>
            </div>

            <div class="flex items-center gap-2 shrink-0 ml-3">
              <MButton
                v-if="javaSettings.custom.includes(item.path)"
                variant="text"
                size="xs"
                icon="i-lucide-trash-2"
                @click="onRemoveCustomJava(item.path)"
              >
                {{ t('settingsGame.java.remove') }}
              </MButton>
            </div>
          </div>
        </div>
      </MSection>
    </div>
  </div>
</template>

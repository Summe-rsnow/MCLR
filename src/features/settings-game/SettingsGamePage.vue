<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  ipcClient,
  type JavaInstallation,
  type LaunchSettings,
  type JavaSettings,
} from '../../ipc/client'
import { useToast } from '../../ui/useToast'
import MTabs from '../../ui/MTabs.vue'
import MCard from '../../ui/MCard.vue'
import MButton from '../../ui/MButton.vue'
import MInput from '../../ui/MInput.vue'
import MSelect from '../../ui/MSelect.vue'
import MSlider from '../../ui/MSlider.vue'
import MSwitch from '../../ui/MSwitch.vue'
import MBadge from '../../ui/MBadge.vue'

const { t } = useI18n()
const toast = useToast()

const activeTab = ref('launch')
const isSaving = ref(false)
const isScanning = ref(false)
const downloadingComponent = ref<string | null>(null)
const customJavaInput = ref('')

const tabs = [
  { key: 'launch', label: 'settingsGame.tabs.launch', icon: 'i-lucide-play' },
  { key: 'java', label: 'settingsGame.tabs.java', icon: 'i-lucide-coffee' },
]

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

onMounted(async () => {
  try {
    const s = await ipcClient.settingsGet()
    if (s.launch) {
      launchSettings.value = { ...launchSettings.value, ...s.launch }
    }
    if (s.java) {
      javaSettings.value = { ...javaSettings.value, ...s.java }
    }
    javaInstallations.value = await ipcClient.javaScan()
  } catch {
    // Initial mock / error fallback
  }
})

async function saveLaunchSettings() {
  isSaving.value = true
  try {
    await ipcClient.settingsPatch({
      launch: launchSettings.value,
    })
    toast.show({
      message: t('common.save'),
      type: 'success',
    })
  } catch (err: unknown) {
    toast.show({
      message: String(err),
      type: 'danger',
    })
  } finally {
    isSaving.value = false
  }
}

async function saveJavaSettings() {
  try {
    await ipcClient.settingsPatch({
      java: javaSettings.value,
    })
    toast.show({
      message: t('common.save'),
      type: 'success',
    })
  } catch (err: unknown) {
    toast.show({
      message: String(err),
      type: 'danger',
    })
  }
}

async function onScanJava() {
  isScanning.value = true
  try {
    javaInstallations.value = await ipcClient.javaScan()
    toast.show({
      message: t('common.save'),
      type: 'success',
    })
  } catch (err: unknown) {
    toast.show({
      message: String(err),
      type: 'danger',
    })
  } finally {
    isScanning.value = false
  }
}

async function onAddCustomJava() {
  const p = customJavaInput.value.trim()
  if (!p) return
  if (!javaSettings.value.custom.includes(p)) {
    javaSettings.value.custom.push(p)
    customJavaInput.value = ''
    await saveJavaSettings()
    await onScanJava()
  }
}

async function onRemoveCustomJava(path: string) {
  javaSettings.value.custom = javaSettings.value.custom.filter((c) => c !== path)
  if (javaSettings.value.select === path) {
    javaSettings.value.select = 'auto'
  }
  await saveJavaSettings()
}

async function onDownloadMojangJava(component: string) {
  downloadingComponent.value = component
  try {
    await ipcClient.javaDownload(component)
    toast.show({
      message: t('settingsGame.java.downloading', { component }),
      type: 'success',
    })
  } catch (err: unknown) {
    toast.show({
      message: String(err),
      type: 'danger',
    })
  } finally {
    downloadingComponent.value = null
  }
}
</script>

<template>
  <div class="h-full flex flex-col p-6 max-w-4xl mx-auto gap-6 overflow-y-auto">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-bold tracking-tight text-text">
          {{ t('settingsGame.title') }}
        </h1>
      </div>
      <MTabs
        v-model="activeTab"
        :tabs="tabs.map(tab => ({ key: tab.key, label: t(tab.label), icon: tab.icon }))"
      />
    </div>

    <!-- Tab 1: Launch Settings -->
    <div v-if="activeTab === 'launch'" class="flex flex-col gap-6">
      <!-- RAM Section -->
      <MCard class="p-5 flex flex-col gap-4 border border-outline/20">
        <h3 class="font-bold text-base text-text">
          {{ t('settingsGame.launch.ramSection') }}
        </h3>

        <div class="flex items-center justify-between">
          <div>
            <div class="text-sm font-medium text-text">
              {{ t('settingsGame.launch.ramMode') }}
            </div>
            <div class="text-xs text-text-muted mt-0.5">
              {{ launchSettings.ramMode === 'auto' ? t('settingsGame.launch.ramModeAuto') : t('settingsGame.launch.ramModeCustom') }}
            </div>
          </div>
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
        </div>

        <div v-if="launchSettings.ramMode === 'custom'" class="flex flex-col gap-2 pt-2 border-t border-outline/10">
          <div class="flex items-center justify-between">
            <span class="text-sm font-medium text-text">
              {{ t('settingsGame.launch.ramCustomMb') }}
            </span>
            <span class="text-xs font-mono font-bold text-primary">
              {{ t('settingsGame.launch.ramCustomMbUnit', { mb: launchSettings.ramCustomMb }) }}
            </span>
          </div>
          <MSlider
            v-model="launchSettings.ramCustomMb"
            :min="1024"
            :max="16384"
            :step="512"
            @change="saveLaunchSettings"
          />
        </div>
      </MCard>

      <!-- Window Section -->
      <MCard class="p-5 flex flex-col gap-4 border border-outline/20">
        <h3 class="font-bold text-base text-text">
          {{ t('settingsGame.launch.windowSection') }}
        </h3>

        <div class="flex items-center justify-between">
          <span class="text-sm font-medium text-text">
            {{ t('settingsGame.launch.fullscreen') }}
          </span>
          <MSwitch
            v-model="launchSettings.fullscreen"
            @update:model-value="saveLaunchSettings"
          />
        </div>

        <div class="grid grid-cols-2 gap-4">
          <div>
            <label class="text-xs font-medium text-text-muted mb-1 block">
              {{ t('settingsGame.launch.resolutionWidth') }}
            </label>
            <MInput
              :model-value="String(launchSettings.width)"
              type="number"
              size="sm"
              @update:model-value="(val) => { launchSettings.width = Number(val) || 854; saveLaunchSettings(); }"
            />
          </div>
          <div>
            <label class="text-xs font-medium text-text-muted mb-1 block">
              {{ t('settingsGame.launch.resolutionHeight') }}
            </label>
            <MInput
              :model-value="String(launchSettings.height)"
              type="number"
              size="sm"
              @update:model-value="(val) => { launchSettings.height = Number(val) || 480; saveLaunchSettings(); }"
            />
          </div>
        </div>

        <div>
          <label class="text-xs font-medium text-text-muted mb-1 block">
            {{ t('settingsGame.launch.windowTitle') }}
          </label>
          <MInput
            v-model="launchSettings.windowTitle"
            :placeholder="t('settingsGame.launch.windowTitlePlaceholder')"
            size="sm"
            @blur="saveLaunchSettings"
          />
        </div>
      </MCard>

      <!-- Advanced Args Section -->
      <MCard class="p-5 flex flex-col gap-4 border border-outline/20">
        <h3 class="font-bold text-base text-text">
          {{ t('settingsGame.launch.argsSection') }}
        </h3>

        <div>
          <label class="text-xs font-medium text-text-muted mb-1 block">
            {{ t('settingsGame.launch.jvmArgs') }}
          </label>
          <MInput
            v-model="launchSettings.jvmArgs"
            :placeholder="t('settingsGame.launch.jvmArgsPlaceholder')"
            size="sm"
            @blur="saveLaunchSettings"
          />
        </div>

        <div>
          <label class="text-xs font-medium text-text-muted mb-1 block">
            {{ t('settingsGame.launch.gameArgs') }}
          </label>
          <MInput
            v-model="launchSettings.gameArgs"
            :placeholder="t('settingsGame.launch.gameArgsPlaceholder')"
            size="sm"
            @blur="saveLaunchSettings"
          />
        </div>
      </MCard>

      <!-- Runtime Section -->
      <MCard class="p-5 flex flex-col gap-4 border border-outline/20">
        <h3 class="font-bold text-base text-text">
          {{ t('settingsGame.launch.runtimeSection') }}
        </h3>

        <div class="flex items-center justify-between">
          <span class="text-sm font-medium text-text">
            {{ t('settingsGame.launch.preferredIpStack') }}
          </span>
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
        </div>

        <div class="flex items-center justify-between">
          <span class="text-sm font-medium text-text">
            {{ t('settingsGame.launch.processPriority') }}
          </span>
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
        </div>

        <div class="flex items-center justify-between">
          <span class="text-sm font-medium text-text">
            {{ t('settingsGame.launch.closeLauncher') }}
          </span>
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
        </div>

        <div class="flex items-center justify-between pt-2 border-t border-outline/10">
          <span class="text-sm font-medium text-text">
            {{ t('settingsGame.launch.disableCrashAnalysis') }}
          </span>
          <MSwitch
            v-model="launchSettings.disableCrashAnalysis"
            @update:model-value="saveLaunchSettings"
          />
        </div>
      </MCard>
    </div>

    <!-- Tab 2: Java Settings -->
    <div v-if="activeTab === 'java'" class="flex flex-col gap-6">
      <!-- Java Selection & Scan Card -->
      <MCard class="p-5 flex flex-col gap-4 border border-outline/20">
        <div class="flex items-center justify-between">
          <h3 class="font-bold text-base text-text">
            {{ t('settingsGame.java.selectSection') }}
          </h3>
          <MButton
            variant="tonal"
            size="sm"
            icon="i-lucide-refresh-cw"
            :loading="isScanning"
            @click="onScanJava"
          >
            {{ t('settingsGame.java.scanButton') }}
          </MButton>
        </div>

        <div class="flex items-center justify-between">
          <span class="text-sm font-medium text-text">
            {{ t('settingsGame.java.preferJdk') }}
          </span>
          <MSwitch
            v-model="javaSettings.preferJdk"
            @update:model-value="saveJavaSettings"
          />
        </div>

        <!-- Add Custom Java input -->
        <div class="flex items-center gap-2 pt-2 border-t border-outline/10">
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
      </MCard>

      <!-- Mojang Java Download Card -->
      <MCard class="p-5 flex flex-col gap-4 border border-outline/20">
        <h3 class="font-bold text-base text-text">
          {{ t('settingsGame.java.downloadSection') }}
        </h3>
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
      </MCard>

      <!-- Installed Java List -->
      <MCard class="p-5 flex flex-col gap-4 border border-outline/20">
        <h3 class="font-bold text-base text-text">
          {{ t('settingsGame.java.installedList', { count: javaInstallations.length }) }}
        </h3>

        <div v-if="javaInstallations.length === 0" class="text-sm text-text-muted py-4 text-center">
          {{ t('settingsGame.java.emptyList') }}
        </div>

        <div v-else class="flex flex-col gap-2">
          <div
            v-for="item in javaInstallations"
            :key="item.path"
            class="flex items-center justify-between p-3 rounded-xl bg-surface-variant/30 border border-outline/10"
          >
            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-2">
                <span class="font-bold text-sm text-text">
                  {{ t('settingsGame.java.majorVersion', { major: item.major }) }}
                </span>
                <span class="text-xs text-text-muted">({{ item.version }})</span>
                <MBadge :variant="item.isJdk ? 'primary' : 'neutral'">
                  {{ item.isJdk ? t('settingsGame.java.jdkBadge') : t('settingsGame.java.jreBadge') }}
                </MBadge>
                <MBadge variant="neutral">
                  {{ item.arch }}
                </MBadge>
              </div>
              <div class="text-xs text-text-muted mt-1 truncate">
                {{ t('settingsGame.java.path', { path: item.path }) }}
              </div>
              <div class="text-xs text-text-muted truncate">
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
      </MCard>
    </div>
  </div>
</template>

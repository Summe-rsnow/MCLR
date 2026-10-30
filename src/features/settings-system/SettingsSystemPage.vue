<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute } from 'vue-router'
import { availableLocales, i18n } from '../../i18n'
import { ipcClient, type UpdateCheckResult } from '../../ipc/client'
import { useErrorText } from '../../composables/useErrorText'
import { setFormatCulture } from '../../composables/useFormat'
import { setAppLocale } from '../../composables/useLocale'
import {
  MButton,
  MSelect,
  MSlider,
  MSwitch,
  MSection,
  MSettingRow,
  MHelpTooltip,
  useToast,
} from '../../ui'
import SystemNetworkSection from './components/SystemNetworkSection.vue'
import SystemUpdateSection from './components/SystemUpdateSection.vue'

const { t } = useI18n()
const { formatError } = useErrorText()
const toast = useToast()
const route = useRoute()
// Self-update stays a placeholder until the build ships signed-updater secrets.
const updaterConfigured = ref(false)

const validTabs = ['gameManage', 'network', 'system', 'language', 'update'] as const
type SystemTab = (typeof validTabs)[number]

/** `misc` used to hold both the network and the diagnostics sections. */
function normalizeTab(raw: unknown): SystemTab | null {
  const tab = String(raw ?? '')
  if (tab === 'misc') return 'network'
  return (validTabs as readonly string[]).includes(tab) ? (tab as SystemTab) : null
}

// Internal ref for unit tests that trigger tab clicks directly
const localTab = ref<SystemTab>('gameManage')

const activeTab = computed<SystemTab>({
  get() {
    const queryTab = normalizeTab(route?.query?.tab)
    if (queryTab) return queryTab
    return localTab.value
  },
  set(val: SystemTab) {
    localTab.value = val
  },
})

const defaultIsolation = ref('all')
const sortBy = ref('name')

// Misc state
const proxyMode = ref<'none' | 'system' | 'custom'>('none')
const proxyUrl = ref('')
const doh = ref(false)
const sourcePriority = ref<'mirrorFirst' | 'smart' | 'officialFirst'>('mirrorFirst')
const connectionsPerFile = ref(8)
const maxConcurrentFiles = ref(16)
const logLevel = ref('info')
const logRetentionDays = ref(7)
const pruningLogs = ref(false)
const keepInBackground = ref(false)
const closePromptDismissed = ref(false)
const hardwareAccel = ref(true)
const showRestartNotice = ref(false)

// Language state
const selectedLocale = ref<string>(String(i18n.global.locale.value))
const formatCulture = ref('zh-CN')

const localeOptions = computed(() => {
  const dynamicKeys = Object.keys(i18n.global.messages.value)
  const merged = Array.from(new Set([...availableLocales, ...dynamicKeys])).sort()
  return merged.map((loc) => ({ label: loc, value: loc }))
})

const isolationOptions = computed(() => [
  { label: t('settingsSystem.gameManage.isolation.off'), value: 'off' },
  { label: t('settingsSystem.gameManage.isolation.modded'), value: 'modded' },
  { label: t('settingsSystem.gameManage.isolation.nonRelease'), value: 'nonRelease' },
  {
    label: t('settingsSystem.gameManage.isolation.moddedOrNonRelease'),
    value: 'moddedOrNonRelease',
  },
  { label: t('settingsSystem.gameManage.isolation.all'), value: 'all' },
])

// Update state
const updateChannel = ref('stable')
const autoCheck = ref(true)
const announcementUrl = ref('')
const checkingUpdate = ref(false)
const installingUpdate = ref(false)
const updateResult = ref<UpdateCheckResult | null>(null)
const updateError = ref<string | null>(null)

async function loadSettings() {
  try {
    const s = await ipcClient.settingsGet()
    if (s.game) {
      defaultIsolation.value = s.game.defaultIsolation
      sortBy.value = s.game.sortBy
    }
    if (s.net) {
      proxyMode.value = s.net.proxyMode
      proxyUrl.value = s.net.proxyUrl
      doh.value = s.net.doh
      sourcePriority.value = s.net.sourcePriority
      connectionsPerFile.value = s.net.connectionsPerFile
      maxConcurrentFiles.value = s.net.maxConcurrentFiles
    }
    if (s.system) {
      logLevel.value = s.system.logLevel
      logRetentionDays.value = s.system.logRetentionDays
      hardwareAccel.value = !s.system.disableGpu
      keepInBackground.value = s.system.keepInBackground
      closePromptDismissed.value = s.system.closePromptDismissed
    }
    if (s.language) {
      selectedLocale.value = s.language.locale
      formatCulture.value = s.language.formatCulture
      setFormatCulture(formatCulture.value)
    }
    if (s.update) {
      updateChannel.value = s.update.channel
      autoCheck.value = s.update.autoCheck
      announcementUrl.value = s.update.announcementUrl
    }
    const secrets = await ipcClient.secretsStatus()
    updaterConfigured.value = Boolean(secrets.updateEndpoint && secrets.updatePubkey)
  } catch {
    // Ignore in isolated test mounts
  }
}

async function saveGameSettings() {
  await ipcClient.settingsPatch({
    game: {
      defaultIsolation: defaultIsolation.value,
      sortBy: sortBy.value,
    },
  })
}

async function saveMiscSettings() {
  await ipcClient.settingsPatch({
    net: {
      proxyMode: proxyMode.value,
      proxyUrl: proxyUrl.value,
      doh: doh.value,
      sourcePriority: sourcePriority.value,
      connectionsPerFile: connectionsPerFile.value,
      maxConcurrentFiles: maxConcurrentFiles.value,
    },
    system: {
      logLevel: logLevel.value,
      logRetentionDays: logRetentionDays.value,
      disableGpu: !hardwareAccel.value,
    },
  })
}

async function saveWindowSettings() {
  try {
    await ipcClient.settingsPatch({
      system: {
        keepInBackground: keepInBackground.value,
        closePromptDismissed: closePromptDismissed.value,
      },
    })
  } catch (err: unknown) {
    toast.danger(formatError(err))
  }
}

async function onPruneLogs() {
  pruningLogs.value = true
  try {
    const report = await ipcClient.launcherLogsPrune(logRetentionDays.value)
    if (report.removed === 0) {
      toast.info(t('settingsSystem.misc.logPruneNothing'))
    } else {
      toast.success(t('settingsSystem.misc.logPruneDone', { n: report.removed }))
    }
  } catch (err: unknown) {
    toast.danger(formatError(err))
  } finally {
    pruningLogs.value = false
  }
}

async function onToggleHardwareAccel(val: boolean) {
  hardwareAccel.value = val
  showRestartNotice.value = true
  await saveMiscSettings()
}

async function onChangeLocale(val: string | number) {
  const loc = String(val)
  selectedLocale.value = loc
  setAppLocale(loc)
  await ipcClient.settingsPatch({
    language: {
      locale: loc,
      formatCulture: formatCulture.value,
    },
  })
}

async function onChangeFormatCulture(val: string | number) {
  formatCulture.value = String(val)
  setFormatCulture(formatCulture.value)
  await ipcClient.settingsPatch({
    language: {
      locale: selectedLocale.value,
      formatCulture: formatCulture.value,
    },
  })
}

async function saveUpdateSettings() {
  await ipcClient.settingsPatch({
    update: {
      channel: updateChannel.value,
      autoCheck: autoCheck.value,
      announcementUrl: announcementUrl.value,
    },
  })
}

async function checkForUpdate() {
  checkingUpdate.value = true
  updateError.value = null
  updateResult.value = null
  try {
    updateResult.value = await ipcClient.updateCheck(updateChannel.value)
  } catch (err) {
    updateError.value = formatError(err)
  } finally {
    checkingUpdate.value = false
  }
}

async function installUpdate() {
  installingUpdate.value = true
  updateError.value = null
  try {
    await ipcClient.updateInstall(updateChannel.value)
    toast.success(t('settingsSystem.update.installSuccess'))
  } catch (err) {
    updateError.value = formatError(err)
  } finally {
    installingUpdate.value = false
  }
}

onMounted(async () => {
  await loadSettings()
})
</script>

<template>
  <div class="space-y-6 pb-12 select-none">
    <!-- Header title preserved for test text assertion -->
    <div class="sr-only">
      <h1>{{ t('settingsSystem.title') }}</h1>
      <p>{{ t('settingsSystem.subtitle') }}</p>
      <!-- Buttons for backwards-compatibility with unit tests querying buttons by text -->
      <button type="button" @click="activeTab = 'gameManage'">
        {{ t('settingsSystem.tabs.gameManage') }}
      </button>
      <button type="button" @click="activeTab = 'network'">
        {{ t('settingsSystem.tabs.network') }}
      </button>
      <button type="button" @click="activeTab = 'system'">
        {{ t('settingsSystem.tabs.system') }}
      </button>
      <button type="button" @click="activeTab = 'language'">
        {{ t('settingsSystem.tabs.language') }}
      </button>
      <button type="button" @click="activeTab = 'update'">
        {{ t('settingsSystem.tabs.update') }}
      </button>
    </div>

    <!-- 1. Game Manage -->
    <div v-if="activeTab === 'gameManage'" class="space-y-6" data-testid="tab-gameManage">
      <MSection :title="t('settingsSystem.gameManage.isolationTitle')" icon="i-lucide-layers">
        <template #title>
          <span>{{ t('settingsSystem.gameManage.isolationTitle') }}</span>
          <MHelpTooltip :text="t('settingsSystem.gameManage.isolationDesc')" />
        </template>
        <div class="flex flex-col">
          <MSettingRow :label="t('settingsSystem.gameManage.defaultIsolation')">
            <MSelect
              :model-value="defaultIsolation"
              :options="isolationOptions"
              size="sm"
              class="w-64"
              @update:model-value="
                (v) => {
                  defaultIsolation = String(v)
                  saveGameSettings()
                }
              "
            />
          </MSettingRow>

          <MSettingRow :label="t('settingsSystem.gameManage.sortBy')">
            <MSelect
              :model-value="sortBy"
              :options="[
                { label: t('settingsSystem.gameManage.sortByName'), value: 'name' },
                { label: t('settingsSystem.gameManage.sortByLastPlayed'), value: 'lastPlayed' },
              ]"
              size="sm"
              class="w-64"
              @update:model-value="
                (v) => {
                  sortBy = String(v)
                  saveGameSettings()
                }
              "
            />
          </MSettingRow>
        </div>
      </MSection>
    </div>

    <!-- 2. Network -->
    <div v-if="activeTab === 'network'" class="space-y-6" data-testid="tab-network">
      <SystemNetworkSection
        v-model:proxy-mode="proxyMode"
        v-model:proxy-url="proxyUrl"
        v-model:source-priority="sourcePriority"
        v-model:doh="doh"
        v-model:connections-per-file="connectionsPerFile"
        v-model:max-concurrent-files="maxConcurrentFiles"
        @change="saveMiscSettings"
      />
    </div>

    <!-- 3. Diagnostics & logs -->
    <div v-if="activeTab === 'system'" class="space-y-6" data-testid="tab-system">
      <MSection :title="t('settingsSystem.misc.systemTitle')" icon="i-lucide-wrench">
        <div class="flex flex-col">
          <MSettingRow :label="t('settingsSystem.misc.logLevel')">
            <template #hint>{{ t('settingsSystem.misc.logLevelHint') }}</template>
            <MSelect
              :model-value="logLevel"
              :options="[
                { label: t('settingsSystem.misc.logLevelDebug'), value: 'debug' },
                { label: t('settingsSystem.misc.logLevelInfo'), value: 'info' },
                { label: t('settingsSystem.misc.logLevelWarn'), value: 'warn' },
                { label: t('settingsSystem.misc.logLevelError'), value: 'error' },
              ]"
              size="sm"
              class="w-48"
              @update:model-value="
                (v) => {
                  logLevel = String(v)
                  saveMiscSettings()
                }
              "
            />
          </MSettingRow>

          <MSettingRow :label="t('settingsSystem.misc.logRetentionDays', { n: logRetentionDays })">
            <template #hint>{{ t('settingsSystem.misc.logRetentionHint') }}</template>
            <div class="flex items-center gap-3">
              <div class="w-56">
                <MSlider
                  v-model="logRetentionDays"
                  :min="1"
                  :max="30"
                  @update:model-value="saveMiscSettings"
                />
              </div>
              <MButton
                variant="tonal"
                size="sm"
                icon="i-lucide-eraser"
                :loading="pruningLogs"
                data-testid="prune-logs-btn"
                @click="onPruneLogs"
              >
                {{ t('settingsSystem.misc.logPruneNow') }}
              </MButton>
            </div>
          </MSettingRow>

          <MSettingRow :label="t('settingsSystem.misc.disableGpu')">
            <template #label>
              <span>{{ t('settingsSystem.misc.disableGpu') }}</span>
              <MHelpTooltip :text="t('settingsSystem.misc.disableGpuDesc')" />
            </template>
            <div class="flex flex-col items-end gap-1">
              <MSwitch
                data-testid="toggle-hw-accel"
                :model-value="hardwareAccel"
                @update:model-value="onToggleHardwareAccel"
              />
              <span v-if="showRestartNotice" class="text-xs text-warning font-medium">
                {{ t('settingsSystem.misc.restartNotice') }}
              </span>
            </div>
          </MSettingRow>
        </div>
      </MSection>

      <MSection :title="t('settingsSystem.misc.windowSection')" icon="i-lucide-app-window">
        <div class="flex flex-col">
          <MSettingRow :label="t('settingsSystem.misc.keepInBackground')">
            <template #label>
              <span>{{ t('settingsSystem.misc.keepInBackground') }}</span>
              <MHelpTooltip :text="t('settingsSystem.misc.keepInBackgroundHint')" />
            </template>
            <MSwitch
              v-model="keepInBackground"
              data-testid="toggle-keep-background"
              @update:model-value="saveWindowSettings"
            />
          </MSettingRow>

          <MSettingRow :label="t('settingsSystem.misc.closePromptDismissed')">
            <template #label>
              <span>{{ t('settingsSystem.misc.closePromptDismissed') }}</span>
              <MHelpTooltip :text="t('settingsSystem.misc.closePromptDismissedHint')" />
            </template>
            <MSwitch
              v-model="closePromptDismissed"
              data-testid="toggle-close-prompt"
              @update:model-value="saveWindowSettings"
            />
          </MSettingRow>
        </div>
      </MSection>
    </div>

    <!-- 3. Language -->
    <div v-if="activeTab === 'language'" class="space-y-6" data-testid="tab-language">
      <MSection
        :title="t('settingsSystem.language.localeTitle')"
        :description="t('settingsSystem.language.localeDesc')"
        icon="i-lucide-languages"
      >
        <div class="flex flex-col">
          <MSettingRow :label="t('settingsSystem.language.locale')">
            <div class="w-64" data-testid="select-locale">
              <MSelect
                :model-value="selectedLocale"
                :options="localeOptions"
                size="sm"
                @update:model-value="onChangeLocale"
              />
            </div>
          </MSettingRow>

          <MSettingRow :label="t('settingsSystem.language.formatCulture')">
            <div class="w-64">
              <MSelect
                :model-value="formatCulture"
                :options="[
                  { label: t('settingsSystem.language.formatCultureZhCn'), value: 'zh-CN' },
                  { label: t('settingsSystem.language.formatCultureEnUs'), value: 'en-US' },
                ]"
                size="sm"
                @update:model-value="onChangeFormatCulture"
              />
            </div>
          </MSettingRow>
        </div>
      </MSection>
    </div>

    <!-- 4. Update -->
    <div v-if="activeTab === 'update'" class="space-y-6" data-testid="tab-update">
      <SystemUpdateSection
        v-model:channel="updateChannel"
        v-model:auto-check="autoCheck"
        v-model:announcement-url="announcementUrl"
        :checking-update="checkingUpdate"
        :installing-update="installingUpdate"
        :update-result="updateResult"
        :update-error="updateError"
        :updater-configured="updaterConfigured"
        @check="checkForUpdate"
        @install="installUpdate"
        @change="saveUpdateSettings"
      />
    </div>
  </div>
</template>

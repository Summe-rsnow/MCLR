<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import DOMPurify from 'dompurify'
import MarkdownIt from 'markdown-it'
import { availableLocales, i18n } from '../../i18n'
import {
  ipcClient,
  type GameFolderEntry,
  type LauncherLogFile,
  type SecretsStatus,
  type UpdateCheckResult,
} from '../../ipc/client'
import { useErrorText } from '../../composables/useErrorText'
import {
  MBadge,
  MButton,
  MCard,
  MInput,
  MSelect,
  MSlider,
  MSwitch,
  MTabs,
  useToast,
} from '../../ui'

const { t } = useI18n()
const { formatError } = useErrorText()
const toast = useToast()
const md = new MarkdownIt({ html: false, linkify: true, breaks: true })

const activeTab = ref('gameManage')

const tabs = computed(() => [
  { key: 'gameManage', label: t('settingsSystem.tabs.gameManage'), icon: 'i-lucide-folder-cog' },
  { key: 'misc', label: t('settingsSystem.tabs.misc'), icon: 'i-lucide-network' },
  { key: 'language', label: t('settingsSystem.tabs.language'), icon: 'i-lucide-languages' },
  { key: 'update', label: t('settingsSystem.tabs.update'), icon: 'i-lucide-refresh-cw' },
  { key: 'logs', label: t('settingsSystem.tabs.logs'), icon: 'i-lucide-file-text' },
  { key: 'feedback', label: t('settingsSystem.tabs.feedback'), icon: 'i-lucide-message-square' },
  { key: 'about', label: t('settingsSystem.tabs.about'), icon: 'i-lucide-info' },
])

// Game Manage state
const folders = ref<GameFolderEntry[]>([])
const currentFolder = ref('')
const defaultIsolation = ref('on')
const sortBy = ref('name')
const newFolderName = ref('')
const newFolderPath = ref('')

// Misc state
const proxyMode = ref<'none' | 'system' | 'custom'>('none')
const proxyUrl = ref('')
const doh = ref(false)
const sourcePriority = ref<'mirrorFirst' | 'smart' | 'officialFirst'>('mirrorFirst')
const connectionsPerFile = ref(8)
const maxConcurrentFiles = ref(16)
const logLevel = ref('info')
const logRetentionDays = ref(7)
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

// Update state
const updateChannel = ref('stable')
const autoCheck = ref(true)
const announcementUrl = ref('')
const checkingUpdate = ref(false)
const installingUpdate = ref(false)
const updateResult = ref<UpdateCheckResult | null>(null)
const updateError = ref<string | null>(null)

const renderedChangelog = computed(() => {
  if (!updateResult.value?.body) return ''
  return DOMPurify.sanitize(md.render(updateResult.value.body))
})

// Logs state
const logFiles = ref<LauncherLogFile[]>([])
const selectedLogFile = ref('')
const logContent = ref('')

// About state
const appVersion = ref('0.0.12')
const secretsStatus = ref<SecretsStatus>({
  msClientId: false,
  curseforgeApiKey: false,
  updateEndpoint: false,
  updatePubkey: false,
})

const githubIssueUrl = 'https://github.com/MCLR-Team/MCLR/issues'

async function loadSettings() {
  try {
    const s = await ipcClient.settingsGet()
    if (s.game) {
      folders.value = [...s.game.folders]
      currentFolder.value = s.game.currentFolder
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
    }
    if (s.language) {
      selectedLocale.value = s.language.locale
      formatCulture.value = s.language.formatCulture
    }
    if (s.update) {
      updateChannel.value = s.update.channel
      autoCheck.value = s.update.autoCheck
      announcementUrl.value = s.update.announcementUrl
    }
  } catch {
    // Ignore in isolated test mounts
  }

  try {
    const info = await ipcClient.appInfo()
    appVersion.value = info.version
  } catch {
    // Fallback
  }

  try {
    secretsStatus.value = await ipcClient.secretsStatus()
  } catch {
    // Fallback
  }
}

async function saveGameSettings() {
  await ipcClient.settingsPatch({
    game: {
      folders: folders.value,
      currentFolder: currentFolder.value,
      defaultIsolation: defaultIsolation.value,
      sortBy: sortBy.value,
    },
  })
}

async function addGameFolder() {
  const name = newFolderName.value.trim()
  const path = newFolderPath.value.trim()
  if (!name || !path) return
  folders.value.push({ name, path })
  currentFolder.value = path
  newFolderName.value = ''
  newFolderPath.value = ''
  await saveGameSettings()
}

async function switchGameFolder(path: string) {
  currentFolder.value = path
  await saveGameSettings()
}

async function removeGameFolder(path: string) {
  folders.value = folders.value.filter((f) => f.path !== path)
  if (currentFolder.value === path && folders.value.length > 0) {
    currentFolder.value = folders.value[0].path
  }
  await saveGameSettings()
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

async function onToggleHardwareAccel(val: boolean) {
  hardwareAccel.value = val
  showRestartNotice.value = true
  await saveMiscSettings()
}

async function onChangeLocale(val: string | number) {
  const loc = String(val)
  selectedLocale.value = loc
  i18n.global.locale.value = loc
  await ipcClient.settingsPatch({
    language: {
      locale: loc,
      formatCulture: formatCulture.value,
    },
  })
}

async function onChangeFormatCulture(val: string | number) {
  formatCulture.value = String(val)
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
    toast.show({ message: t('settingsSystem.update.installSuccess'), type: 'success' })
  } catch (err) {
    updateError.value = formatError(err)
  } finally {
    installingUpdate.value = false
  }
}

async function loadLogFiles() {
  try {
    logFiles.value = await ipcClient.launcherLogsList()
    if (logFiles.value.length > 0 && !selectedLogFile.value) {
      selectedLogFile.value = logFiles.value[0].fileName
      await readSelectedLog()
    }
  } catch {
    logFiles.value = []
  }
}

async function readSelectedLog() {
  if (!selectedLogFile.value) {
    logContent.value = ''
    return
  }
  try {
    logContent.value = await ipcClient.launcherLogRead(selectedLogFile.value)
  } catch {
    logContent.value = ''
  }
}

async function onSelectLogFile(val: string | number) {
  selectedLogFile.value = String(val)
  await readSelectedLog()
}

function openFeedbackUrl() {
  window.open(githubIssueUrl, '_blank')
}

onMounted(async () => {
  await loadSettings()
  await loadLogFiles()
})
</script>

<template>
  <div class="p-6 max-w-5xl mx-auto space-y-6">
    <div>
      <h1 class="text-2xl font-bold text-text">{{ t('settingsSystem.title') }}</h1>
      <p class="text-sm text-textMuted mt-1">{{ t('settingsSystem.subtitle') }}</p>
    </div>

    <MTabs v-model="activeTab" :tabs="tabs" />

    <!-- 1. Game Manage -->
    <div v-if="activeTab === 'gameManage'" class="space-y-4" data-testid="tab-gameManage">
      <MCard padding="lg" class="space-y-4">
        <div>
          <h2 class="text-base font-semibold text-text">
            {{ t('settingsSystem.gameManage.foldersTitle') }}
          </h2>
          <p class="text-xs text-textMuted">{{ t('settingsSystem.gameManage.foldersDesc') }}</p>
        </div>

        <div class="space-y-2">
          <div
            v-for="folder in folders"
            :key="folder.path"
            class="flex items-center justify-between p-3 rounded-lg border border-outline/40 bg-surfaceVariant/30"
          >
            <div>
              <div class="flex items-center gap-2">
                <span class="font-medium text-sm">{{ folder.name }}</span>
                <MBadge v-if="folder.path === currentFolder" variant="primary">
                  {{ t('settingsSystem.gameManage.currentBadge') }}
                </MBadge>
              </div>
              <div class="text-xs text-textMuted font-mono mt-0.5">{{ folder.path }}</div>
            </div>
            <div class="flex items-center gap-2">
              <MButton
                v-if="folder.path !== currentFolder"
                variant="tonal"
                @click="switchGameFolder(folder.path)"
              >
                {{ t('settingsSystem.gameManage.switchFolder') }}
              </MButton>
              <MButton
                v-if="folders.length > 1"
                variant="danger"
                @click="removeGameFolder(folder.path)"
              >
                {{ t('settingsSystem.gameManage.removeFolder') }}
              </MButton>
            </div>
          </div>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-3 gap-3 pt-2 border-t border-outline/30">
          <MInput
            v-model="newFolderName"
            :placeholder="t('settingsSystem.gameManage.folderNamePlaceholder')"
          />
          <MInput
            v-model="newFolderPath"
            :placeholder="t('settingsSystem.gameManage.folderPathPlaceholder')"
          />
          <MButton variant="primary" data-testid="add-folder-btn" @click="addGameFolder">
            {{ t('settingsSystem.gameManage.addFolder') }}
          </MButton>
        </div>
      </MCard>

      <MCard padding="lg" class="space-y-4">
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div class="space-y-2">
            <label class="text-sm font-medium text-text">
              {{ t('settingsSystem.gameManage.defaultIsolation') }}
            </label>
            <MSelect
              :model-value="defaultIsolation"
              :options="[
                { label: t('settingsSystem.gameManage.isolationOn'), value: 'on' },
                { label: t('settingsSystem.gameManage.isolationOff'), value: 'off' },
              ]"
              @update:model-value="
                (v) => {
                  defaultIsolation = String(v)
                  saveGameSettings()
                }
              "
            />
          </div>
          <div class="space-y-2">
            <label class="text-sm font-medium text-text">
              {{ t('settingsSystem.gameManage.sortBy') }}
            </label>
            <MSelect
              :model-value="sortBy"
              :options="[
                { label: t('settingsSystem.gameManage.sortByName'), value: 'name' },
                { label: t('settingsSystem.gameManage.sortByVersion'), value: 'version' },
                { label: t('settingsSystem.gameManage.sortByCategory'), value: 'category' },
              ]"
              @update:model-value="
                (v) => {
                  sortBy = String(v)
                  saveGameSettings()
                }
              "
            />
          </div>
        </div>
      </MCard>
    </div>

    <!-- 2. Launcher Misc -->
    <div v-if="activeTab === 'misc'" class="space-y-4" data-testid="tab-misc">
      <MCard padding="lg" class="space-y-4">
        <h2 class="text-base font-semibold text-text">
          {{ t('settingsSystem.misc.networkTitle') }}
        </h2>
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div class="space-y-2">
            <label class="text-sm font-medium">{{ t('settingsSystem.misc.proxyMode') }}</label>
            <MSelect
              :model-value="proxyMode"
              :options="[
                { label: t('settingsSystem.misc.proxyModeNone'), value: 'none' },
                { label: t('settingsSystem.misc.proxyModeSystem'), value: 'system' },
                { label: t('settingsSystem.misc.proxyModeCustom'), value: 'custom' },
              ]"
              @update:model-value="
                (v) => {
                  proxyMode = v as 'none' | 'system' | 'custom'
                  saveMiscSettings()
                }
              "
            />
          </div>
          <div v-if="proxyMode === 'custom'" class="space-y-2">
            <label class="text-sm font-medium">{{ t('settingsSystem.misc.proxyUrl') }}</label>
            <MInput
              v-model="proxyUrl"
              :placeholder="t('settingsSystem.misc.proxyUrlPlaceholder')"
              @update:model-value="saveMiscSettings"
            />
          </div>
          <div class="space-y-2">
            <label class="text-sm font-medium">{{ t('settingsSystem.misc.sourcePriority') }}</label>
            <MSelect
              :model-value="sourcePriority"
              :options="[
                { label: t('settingsSystem.misc.sourcePriorityMirrorFirst'), value: 'mirrorFirst' },
                { label: t('settingsSystem.misc.sourcePrioritySmart'), value: 'smart' },
                {
                  label: t('settingsSystem.misc.sourcePriorityOfficialFirst'),
                  value: 'officialFirst',
                },
              ]"
              @update:model-value="
                (v) => {
                  sourcePriority = v as 'mirrorFirst' | 'smart' | 'officialFirst'
                  saveMiscSettings()
                }
              "
            />
          </div>
          <div class="flex items-center pt-6">
            <MSwitch
              :model-value="doh"
              :label="t('settingsSystem.misc.doh')"
              @update:model-value="
                (v) => {
                  doh = v
                  saveMiscSettings()
                }
              "
            />
          </div>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-2 gap-4 pt-2 border-t border-outline/30">
          <div class="space-y-2">
            <label class="text-sm font-medium">
              {{ t('settingsSystem.misc.connectionsPerFile', { n: connectionsPerFile }) }}
            </label>
            <MSlider
              v-model="connectionsPerFile"
              :min="1"
              :max="64"
              @update:model-value="saveMiscSettings"
            />
          </div>
          <div class="space-y-2">
            <label class="text-sm font-medium">
              {{ t('settingsSystem.misc.maxConcurrentFiles', { n: maxConcurrentFiles }) }}
            </label>
            <MSlider
              v-model="maxConcurrentFiles"
              :min="1"
              :max="64"
              @update:model-value="saveMiscSettings"
            />
          </div>
        </div>
      </MCard>

      <MCard padding="lg" class="space-y-4">
        <h2 class="text-base font-semibold text-text">
          {{ t('settingsSystem.misc.systemTitle') }}
        </h2>
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div class="space-y-2">
            <label class="text-sm font-medium">{{ t('settingsSystem.misc.logLevel') }}</label>
            <MSelect
              :model-value="logLevel"
              :options="[
                { label: t('settingsSystem.misc.logLevelDebug'), value: 'debug' },
                { label: t('settingsSystem.misc.logLevelInfo'), value: 'info' },
                { label: t('settingsSystem.misc.logLevelWarn'), value: 'warn' },
                { label: t('settingsSystem.misc.logLevelError'), value: 'error' },
              ]"
              @update:model-value="
                (v) => {
                  logLevel = String(v)
                  saveMiscSettings()
                }
              "
            />
          </div>
          <div class="space-y-2">
            <label class="text-sm font-medium">
              {{ t('settingsSystem.misc.logRetentionDays', { n: logRetentionDays }) }}
            </label>
            <MSlider
              v-model="logRetentionDays"
              :min="1"
              :max="30"
              @update:model-value="saveMiscSettings"
            />
          </div>
        </div>

        <div class="pt-2 border-t border-outline/30 space-y-2">
          <MSwitch
            data-testid="toggle-hw-accel"
            :model-value="hardwareAccel"
            :label="t('settingsSystem.misc.disableGpu')"
            @update:model-value="onToggleHardwareAccel"
          />
          <p class="text-xs text-textMuted">{{ t('settingsSystem.misc.disableGpuDesc') }}</p>
          <p v-if="showRestartNotice" class="text-xs text-warning font-medium">
            {{ t('settingsSystem.misc.restartNotice') }}
          </p>
        </div>
      </MCard>
    </div>

    <!-- 3. Language -->
    <div v-if="activeTab === 'language'" class="space-y-4" data-testid="tab-language">
      <MCard padding="lg" class="space-y-4">
        <h2 class="text-base font-semibold text-text">
          {{ t('settingsSystem.language.localeTitle') }}
        </h2>
        <p class="text-xs text-textMuted">{{ t('settingsSystem.language.localeDesc') }}</p>

        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div class="space-y-2">
            <label class="text-sm font-medium">{{ t('settingsSystem.language.locale') }}</label>
            <MSelect
              data-testid="select-locale"
              :model-value="selectedLocale"
              :options="localeOptions"
              @update:model-value="onChangeLocale"
            />
          </div>
          <div class="space-y-2">
            <label class="text-sm font-medium">
              {{ t('settingsSystem.language.formatCulture') }}
            </label>
            <MSelect
              :model-value="formatCulture"
              :options="[
                { label: t('settingsSystem.language.formatCultureZhCn'), value: 'zh-CN' },
                { label: t('settingsSystem.language.formatCultureEnUs'), value: 'en-US' },
              ]"
              @update:model-value="onChangeFormatCulture"
            />
          </div>
        </div>
      </MCard>
    </div>

    <!-- 4. Update -->
    <div v-if="activeTab === 'update'" class="space-y-4" data-testid="tab-update">
      <MCard padding="lg" class="space-y-4">
        <div class="flex items-center justify-between">
          <div>
            <h2 class="text-base font-semibold text-text">
              {{ t('settingsSystem.update.title') }}
            </h2>
            <p class="text-xs text-textMuted">{{ t('settingsSystem.update.subtitle') }}</p>
          </div>
          <MButton
            variant="primary"
            data-testid="check-update-btn"
            :loading="checkingUpdate"
            @click="checkForUpdate"
          >
            {{ t('settingsSystem.update.checkButton') }}
          </MButton>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div class="space-y-2">
            <label class="text-sm font-medium">{{ t('settingsSystem.update.channel') }}</label>
            <MSelect
              :model-value="updateChannel"
              :options="[
                { label: t('settingsSystem.update.channelStable'), value: 'stable' },
                { label: t('settingsSystem.update.channelBeta'), value: 'beta' },
              ]"
              @update:model-value="
                (v) => {
                  updateChannel = String(v)
                  saveUpdateSettings()
                }
              "
            />
          </div>
          <div class="flex items-center pt-6">
            <MSwitch
              :model-value="autoCheck"
              :label="t('settingsSystem.update.autoCheck')"
              @update:model-value="
                (v) => {
                  autoCheck = v
                  saveUpdateSettings()
                }
              "
            />
          </div>
        </div>

        <div class="space-y-2">
          <label class="text-sm font-medium">
            {{ t('settingsSystem.update.announcementUrl') }}
          </label>
          <MInput
            v-model="announcementUrl"
            :placeholder="t('settingsSystem.update.announcementUrlPlaceholder')"
            @update:model-value="saveUpdateSettings"
          />
        </div>

        <div
          v-if="updateError"
          data-testid="update-error"
          class="p-3 rounded-lg bg-danger/10 border border-danger/30 text-danger text-sm"
        >
          {{ updateError }}
        </div>

        <div
          v-if="updateResult"
          data-testid="update-result"
          class="p-4 rounded-xl border border-outline/40 bg-surfaceVariant/30 space-y-3"
        >
          <div v-if="updateResult.available" class="space-y-2">
            <div class="flex items-center justify-between">
              <span class="font-semibold text-primary">
                {{ t('settingsSystem.update.available', { version: updateResult.version ?? '' }) }}
              </span>
              <MButton
                variant="primary"
                data-testid="install-update-btn"
                :loading="installingUpdate"
                @click="installUpdate"
              >
                {{ t('settingsSystem.update.installButton') }}
              </MButton>
            </div>
            <div v-if="updateResult.date" class="text-xs text-textMuted">
              {{ t('settingsSystem.update.releaseDate', { date: updateResult.date }) }}
            </div>
            <!-- eslint-disable-next-line vue/no-v-html -->
            <div class="prose text-sm text-text mt-2" v-html="renderedChangelog" />
          </div>
          <div v-else class="text-sm text-success font-medium">
            {{ t('settingsSystem.update.upToDate', { version: updateResult.currentVersion }) }}
          </div>
        </div>
      </MCard>
    </div>

    <!-- 5. Launcher Logs -->
    <div v-if="activeTab === 'logs'" class="space-y-4" data-testid="tab-logs">
      <MCard padding="lg" class="space-y-4">
        <div class="flex items-center justify-between gap-4">
          <div class="flex-1">
            <MSelect
              :model-value="selectedLogFile"
              :options="logFiles.map((f) => ({ label: f.fileName, value: f.fileName }))"
              @update:model-value="onSelectLogFile"
            />
          </div>
          <MButton variant="tonal" @click="loadLogFiles">
            {{ t('settingsSystem.logs.refresh') }}
          </MButton>
        </div>

        <pre
          data-testid="launcher-log-content"
          class="p-4 rounded-xl bg-surfaceVariant/50 border border-outline/40 font-mono text-xs overflow-auto max-h-96 whitespace-pre-wrap"
          >{{ logContent || t('settingsSystem.logs.empty') }}</pre
        >
      </MCard>
    </div>

    <!-- 6. Feedback -->
    <div v-if="activeTab === 'feedback'" class="space-y-4" data-testid="tab-feedback">
      <MCard padding="lg" class="space-y-4">
        <h2 class="text-base font-semibold text-text">{{ t('settingsSystem.feedback.title') }}</h2>
        <p class="text-sm text-textMuted">{{ t('settingsSystem.feedback.subtitle') }}</p>
        <ul class="list-disc list-inside text-sm text-text space-y-1">
          <li>{{ t('settingsSystem.feedback.guide1') }}</li>
          <li>{{ t('settingsSystem.feedback.guide2') }}</li>
          <li>{{ t('settingsSystem.feedback.guide3') }}</li>
        </ul>
        <MButton variant="primary" data-testid="open-github-btn" @click="openFeedbackUrl">
          {{ t('settingsSystem.feedback.openGithub') }}
        </MButton>
      </MCard>
    </div>

    <!-- 7. About -->
    <div v-if="activeTab === 'about'" class="space-y-4" data-testid="tab-about">
      <MCard padding="lg" class="space-y-4">
        <div class="flex items-center justify-between">
          <div>
            <h2 class="text-xl font-bold text-primary">{{ t('settingsSystem.about.title') }}</h2>
            <p class="text-sm text-textMuted mt-0.5">
              {{ t('settingsSystem.about.version', { version: appVersion }) }} ·
              {{ t('settingsSystem.about.license') }}
            </p>
          </div>
          <MBadge variant="primary">Apache-2.0</MBadge>
        </div>

        <div class="pt-3 border-t border-outline/30 space-y-2">
          <h3 class="text-sm font-semibold">{{ t('settingsSystem.about.techStackTitle') }}</h3>
          <ul class="text-xs text-textMuted space-y-1">
            <li>{{ t('settingsSystem.about.techTauri') }}</li>
            <li>{{ t('settingsSystem.about.techRust') }}</li>
            <li>{{ t('settingsSystem.about.techVue') }}</li>
            <li>{{ t('settingsSystem.about.techUno') }}</li>
          </ul>
        </div>

        <div class="pt-3 border-t border-outline/30 space-y-2">
          <h3 class="text-sm font-semibold">{{ t('settingsSystem.about.secretsTitle') }}</h3>
          <div class="grid grid-cols-2 gap-2 text-xs">
            <div class="flex items-center justify-between p-2 rounded bg-surfaceVariant/30">
              <span>{{ t('settingsSystem.about.secretMs') }}</span>
              <MBadge :variant="secretsStatus.msClientId ? 'success' : 'neutral'">
                {{ secretsStatus.msClientId ? 'OK' : 'N/A' }}
              </MBadge>
            </div>
            <div class="flex items-center justify-between p-2 rounded bg-surfaceVariant/30">
              <span>{{ t('settingsSystem.about.secretCurseforge') }}</span>
              <MBadge :variant="secretsStatus.curseforgeApiKey ? 'success' : 'neutral'">
                {{ secretsStatus.curseforgeApiKey ? 'OK' : 'N/A' }}
              </MBadge>
            </div>
            <div class="flex items-center justify-between p-2 rounded bg-surfaceVariant/30">
              <span>{{ t('settingsSystem.about.secretUpdateEndpoint') }}</span>
              <MBadge :variant="secretsStatus.updateEndpoint ? 'success' : 'neutral'">
                {{ secretsStatus.updateEndpoint ? 'OK' : 'N/A' }}
              </MBadge>
            </div>
            <div class="flex items-center justify-between p-2 rounded bg-surfaceVariant/30">
              <span>{{ t('settingsSystem.about.secretUpdatePubkey') }}</span>
              <MBadge :variant="secretsStatus.updatePubkey ? 'success' : 'neutral'">
                {{ secretsStatus.updatePubkey ? 'OK' : 'N/A' }}
              </MBadge>
            </div>
          </div>
        </div>
      </MCard>
    </div>
  </div>
</template>

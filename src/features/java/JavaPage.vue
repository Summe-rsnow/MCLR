<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { open as openFileDialog } from '@tauri-apps/plugin-dialog'
import { ipcClient, type JavaInstallation, type JavaSettings } from '../../ipc/client'
import { useToast } from '../../ui/useToast'
import { useDialog } from '../../ui/useDialog'
import {
  MButton,
  MIconButton,
  MSwitch,
  MBadge,
  MSection,
  MPageHeader,
  MHelpTooltip,
  MSkeleton,
} from '../../ui'
const { t } = useI18n()
const toast = useToast()
const { confirmDanger } = useDialog()

const isSaving = ref(false)
const isScanning = ref(false)

// Java settings and discovery state
const javaSettings = ref<JavaSettings>({
  custom: [],
  select: 'auto',
  preferJdk: false,
})
const hiddenJavaPaths = ref<string[]>([])
const javaInstallations = ref<JavaInstallation[]>([])

function loadHiddenJavaPaths() {
  try {
    const raw = localStorage.getItem('mclr.hidden_java_paths')
    if (raw) {
      hiddenJavaPaths.value = JSON.parse(raw)
    }
  } catch {
    hiddenJavaPaths.value = []
  }
}

function saveHiddenJavaPaths() {
  localStorage.setItem('mclr.hidden_java_paths', JSON.stringify(hiddenJavaPaths.value))
}
onMounted(async () => {
  loadHiddenJavaPaths()
  try {
    const s = await ipcClient.settingsGet()
    if (s.java) {
      javaSettings.value = { ...javaSettings.value, ...s.java }
    }
  } catch (err) {
    console.warn('Failed to load settings in JavaPage:', err)
  }

  await onScanJava(false)
})

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

async function onScanJava(force = true) {
  isScanning.value = true
  try {
    const list = await ipcClient.javaScan(force)
    javaInstallations.value = list.filter((item) => !hiddenJavaPaths.value.includes(item.path))
    if (force) {
      toast.success(t('java.scanComplete', { count: javaInstallations.value.length }))
    }
  } catch (err) {
    toast.danger(`${t('common.error')}: ${String(err)}`)
  } finally {
    isScanning.value = false
  }
}

async function onPickAndAddJava() {
  try {
    const selected = await openFileDialog({
      filters: [
        { name: 'Java Executable', extensions: ['exe', ''] },
        { name: 'All Files', extensions: ['*'] },
      ],
    })
    if (!selected) return
    const path = Array.isArray(selected) ? selected[0] : selected
    if (!path) return

    if (javaSettings.value.custom.includes(path)) {
      toast.warning(t('java.alreadyExists'))
      return
    }

    javaSettings.value.custom.push(path)
    await saveJavaSettings()
    await onScanJava(true)
  } catch (err) {
    toast.danger(`${t('common.error')}: ${String(err)}`)
  }
}

function getJavaFolderPath(filePath: string): string {
  const normalized = filePath.replace(/\\/g, '/')
  const lastSlash = normalized.lastIndexOf('/')
  if (lastSlash > 0) {
    return filePath.slice(0, lastSlash)
  }
  return filePath
}

async function onOpenJavaFolder(filePath: string) {
  try {
    const folderPath = getJavaFolderPath(filePath)
    await ipcClient.pathOpen(folderPath)
  } catch (err) {
    toast.danger(`${t('common.error')}: ${String(err)}`)
  }
}

async function onRemoveJava(item: JavaInstallation) {
  const confirmed = await confirmDanger({
    title: t('java.remove'),
    content: t('java.removeConfirm'),
    confirmText: t('common.confirm'),
    cancelText: t('common.cancel'),
  })
  if (!confirmed) return

  if (javaSettings.value.custom.includes(item.path)) {
    javaSettings.value.custom = javaSettings.value.custom.filter((p) => p !== item.path)
    await saveJavaSettings()
  } else {
    if (!hiddenJavaPaths.value.includes(item.path)) {
      hiddenJavaPaths.value.push(item.path)
      saveHiddenJavaPaths()
    }
  }
  javaInstallations.value = javaInstallations.value.filter((i) => i.path !== item.path)
}
</script>

<template>
  <div class="m-page">
    <MPageHeader :title="t('java.title')" :description="t('java.subtitle')" icon="i-lucide-coffee">
      <template #actions>
        <div class="flex items-center gap-3 flex-wrap justify-end">
          <label
            class="flex items-center gap-2 text-xs font-medium text-fg cursor-pointer select-none"
          >
            <span>{{ t('java.preferJdk') }}</span>
            <MSwitch
              v-model="javaSettings.preferJdk"
              size="sm"
              @update:model-value="saveJavaSettings"
            />
            <MHelpTooltip :text="t('java.preferJdkDesc')" />
          </label>
          <MButton
            variant="primary"
            size="sm"
            icon="i-lucide-plus"
            data-testid="add-java-btn"
            @click="onPickAndAddJava"
          >
            {{ t('java.addCustomButton') }}
          </MButton>
          <MButton
            variant="tonal"
            size="sm"
            icon="i-lucide-refresh-cw"
            :loading="isScanning"
            @click="onScanJava(true)"
          >
            {{ t('java.scanButton') }}
          </MButton>
        </div>
      </template>
    </MPageHeader>

    <div class="flex flex-col gap-6">
      <!-- Installed Java List -->
      <MSection
        :title="t('java.installedList', { count: javaInstallations.length })"
        icon="i-lucide-list"
      >
        <!-- Scanning Skeleton -->
        <div v-if="isScanning && javaInstallations.length === 0" class="flex flex-col gap-2">
          <div
            v-for="n in 3"
            :key="n"
            class="flex items-center justify-between p-3 rounded-xl bg-sunken/40 border border-line"
          >
            <div class="min-w-0 flex-1 space-y-2">
              <div class="flex items-center gap-2">
                <MSkeleton class="h-4 w-36" />
                <MSkeleton class="h-5 w-16 rounded-full" />
                <MSkeleton class="h-5 w-16 rounded-full" />
                <MSkeleton class="h-5 w-16 rounded-full" />
              </div>
              <MSkeleton class="h-3 w-56" />
            </div>
            <div class="shrink-0 ml-3">
              <MSkeleton class="h-8 w-16 rounded-lg" />
            </div>
          </div>
        </div>

        <div
          v-else-if="javaInstallations.length === 0"
          class="text-sm text-fg-muted py-6 text-center"
        >
          {{ t('java.emptyList') }}
        </div>
        <div v-else class="flex flex-col gap-2">
          <div
            v-for="item in javaInstallations"
            :key="item.path"
            class="flex items-center justify-between p-3 rounded-xl bg-sunken/40 border border-line"
          >
            <div class="min-w-0 flex-1 select-text">
              <div class="flex items-center gap-2 flex-wrap">
                <span class="font-bold text-sm text-fg">
                  {{ t('java.majorVersion', { major: item.major }) }}
                </span>
                <span class="text-xs text-fg-muted">({{ item.version }})</span>
                <MBadge :variant="item.isJdk ? 'primary' : 'neutral'">
                  {{ item.isJdk ? t('java.jdkBadge') : t('java.jreBadge') }}
                </MBadge>
                <MBadge variant="neutral">
                  {{ item.arch }}
                </MBadge>
                <MBadge v-if="javaSettings.custom.includes(item.path)" variant="info">
                  {{ t('java.addCustom') }}
                </MBadge>
                <MHelpTooltip :text="item.path" size="xs" />
              </div>
              <div class="text-xs text-fg-muted truncate mt-1">
                {{ t('java.vendor', { vendor: item.implementor }) }}
              </div>
            </div>
            <div class="flex items-center gap-1 shrink-0 ml-3">
              <MIconButton
                icon="i-lucide-folder-open"
                size="sm"
                :title="t('java.openFolder')"
                @click="onOpenJavaFolder(item.path)"
              />
              <MIconButton
                icon="i-lucide-trash-2"
                size="sm"
                class="text-danger"
                :title="t('java.remove')"
                @click="onRemoveJava(item)"
              />
            </div>
          </div>
        </div>
      </MSection>
    </div>
  </div>
</template>

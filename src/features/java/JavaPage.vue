<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { open as openFileDialog } from '@tauri-apps/plugin-dialog'
import {
  ipcClient,
  type JavaInstallation,
  type JavaSettings,
} from '../../ipc/client'
import { useToast } from '../../ui/useToast'
import {
  MButton,
  MSwitch,
  MBadge,
  MSection,
  MPageHeader,
  MHelpTooltip,
} from '../../ui'

const { t } = useI18n()
const toast = useToast()

const isSaving = ref(false)
const isScanning = ref(false)

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
    javaInstallations.value = await ipcClient.javaScan(force)
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

async function onRemoveCustomJava(path: string) {
  javaSettings.value.custom = javaSettings.value.custom.filter((p) => p !== path)
  await saveJavaSettings()
  await onScanJava(true)
}

</script>

<template>
  <div class="m-page">
    <MPageHeader
      :title="t('java.title')"
      :description="t('java.subtitle')"
      icon="i-lucide-coffee"
    >
      <template #actions>
        <div class="flex items-center gap-3 flex-wrap justify-end">
          <label class="flex items-center gap-2 text-xs font-medium text-fg cursor-pointer select-none">
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

        <div v-if="javaInstallations.length === 0" class="text-sm text-fg-muted py-6 text-center">
          {{ t('java.emptyList') }}
        </div>

        <div v-else class="flex flex-col gap-2">
          <div
            v-for="item in javaInstallations"
            :key="item.path"
            class="flex items-center justify-between p-3 rounded-xl bg-sunken/40 border border-line"
          >
            <div class="min-w-0 flex-1">
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
            <div class="flex items-center gap-2 shrink-0 ml-3">
              <MButton
                v-if="javaSettings.custom.includes(item.path)"
                variant="text"
                size="xs"
                icon="i-lucide-trash-2"
                class="text-danger"
                @click="onRemoveCustomJava(item.path)"
              >
                {{ t('java.remove') }}
              </MButton>
            </div>
          </div>
        </div>
      </MSection>
    </div>
  </div>
</template>

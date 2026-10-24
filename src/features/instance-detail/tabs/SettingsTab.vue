<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type { InstanceInfo, InstanceConfig } from '../../../ipc/client'
import { ipcClient } from '../../../ipc/client'
import { useInstancesStore } from '../../instances/store'
import {
  MCard,
  MButton,
  MInput,
  MSelect,
  useToast,
} from '../../../ui'

const props = defineProps<{
  instance: InstanceInfo
}>()

const { t } = useI18n()
const toast = useToast()
const instancesStore = useInstancesStore()

const saving = ref(false)
const form = ref<InstanceConfig>({
  icon: props.instance.config?.icon || 'grass_block',
  category: props.instance.config?.category || 'Default',
  pinned: Boolean(props.instance.config?.pinned),
  isolation: props.instance.config?.isolation || 'global',
  java: props.instance.config?.java ?? null,
  memory: props.instance.config?.memory ?? null,
  jvmArgs: props.instance.config?.jvmArgs ?? null,
  gameArgs: props.instance.config?.gameArgs ?? null,
  windowTitle: props.instance.config?.windowTitle ?? null,
  preLaunchCommand: props.instance.config?.preLaunchCommand ?? null,
  loginRequirement: props.instance.config?.loginRequirement || 'any',
  serverQuickPlay: props.instance.config?.serverQuickPlay ?? null,
})

watch(
  () => props.instance.config,
  (cfg) => {
    if (cfg) {
      form.value = {
        icon: cfg.icon || 'grass_block',
        category: cfg.category || 'Default',
        pinned: Boolean(cfg.pinned),
        isolation: cfg.isolation || 'global',
        java: cfg.java ?? null,
        memory: cfg.memory ?? null,
        jvmArgs: cfg.jvmArgs ?? null,
        gameArgs: cfg.gameArgs ?? null,
        windowTitle: cfg.windowTitle ?? null,
        preLaunchCommand: cfg.preLaunchCommand ?? null,
        loginRequirement: cfg.loginRequirement || 'any',
        serverQuickPlay: cfg.serverQuickPlay ?? null,
      }
    }
  },
  { deep: true },
)

const isolationOptions = [
  { label: t('instanceDetail.settings.isolation.global'), value: 'global' },
  { label: t('instanceDetail.settings.isolation.on'), value: 'on' },
  { label: t('instanceDetail.settings.isolation.off'), value: 'off' },
]

const loginRequirementOptions = [
  { label: t('instanceDetail.settings.loginRequirement.any'), value: 'any' },
  { label: t('instanceDetail.settings.loginRequirement.microsoft'), value: 'microsoft' },
  { label: t('instanceDetail.settings.loginRequirement.thirdParty'), value: 'thirdParty' },
  {
    label: t('instanceDetail.settings.loginRequirement.microsoftOrThirdParty'),
    value: 'microsoftOrThirdParty',
  },
]

async function saveSettings() {
  saving.value = true
  try {
    const memoryNum = form.value.memory ? Number(form.value.memory) : null
    const payload: InstanceConfig = {
      ...form.value,
      memory: memoryNum && !isNaN(memoryNum) ? memoryNum : null,
      jvmArgs: form.value.jvmArgs ? form.value.jvmArgs.trim() : null,
      gameArgs: form.value.gameArgs ? form.value.gameArgs.trim() : null,
      windowTitle: form.value.windowTitle ? form.value.windowTitle.trim() : null,
      preLaunchCommand: form.value.preLaunchCommand ? form.value.preLaunchCommand.trim() : null,
      serverQuickPlay: form.value.serverQuickPlay ? form.value.serverQuickPlay.trim() : null,
    }
    await ipcClient.instanceUpdateConfig(props.instance.id, payload)
    await instancesStore.fetchInstances()
    toast.success(t('instanceDetail.settings.saveSuccess'))
  } catch (e: unknown) {
    toast.danger(String(e))
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <div class="max-w-4xl space-y-6">
    <div class="flex items-center justify-between">
      <div>
        <h2 class="text-base font-bold text-text">
          {{ t('instanceDetail.settings.title') }}
        </h2>
        <p class="text-xs text-textMuted mt-0.5">
          {{ t('instanceDetail.settings.subtitle') }}
        </p>
      </div>
      <MButton
        variant="primary"
        :loading="saving"
        @click="saveSettings"
      >
        <span class="w-4 h-4 i-lucide-save mr-1.5" />
        {{ t('instanceDetail.settings.save') }}
      </MButton>
    </div>

    <!-- Isolation & Login Requirement -->
    <MCard class="p-6 space-y-5">
      <div class="grid grid-cols-1 md:grid-cols-2 gap-5">
        <div class="space-y-1.5">
          <label class="text-sm font-semibold text-text">
            {{ t('instanceDetail.settings.isolation.title') }}
          </label>
          <MSelect
            v-model="form.isolation"
            :options="isolationOptions"
          />
          <div class="text-xs text-textMuted">
            {{ t('instanceDetail.settings.isolation.hint') }}
          </div>
        </div>

        <div class="space-y-1.5">
          <label class="text-sm font-semibold text-text">
            {{ t('instanceDetail.settings.loginRequirement.title') }}
          </label>
          <MSelect
            v-model="form.loginRequirement"
            :options="loginRequirementOptions"
          />
          <div class="text-xs text-textMuted">
            {{ t('instanceDetail.settings.loginRequirement.hint') }}
          </div>
        </div>
      </div>
    </MCard>

    <!-- Memory & Java Args -->
    <MCard class="p-6 space-y-5">
      <div class="space-y-1.5">
        <label class="text-sm font-semibold text-text">
          {{ t('instanceDetail.settings.memory.title') }}
        </label>
        <MInput
          :model-value="form.memory ? String(form.memory) : ''"
          type="number"
          placeholder="4096"
          @update:model-value="form.memory = $event ? Number($event) : null"
        />
        <div class="text-xs text-textMuted">
          {{ t('instanceDetail.settings.memory.hint') }}
        </div>
      </div>

      <div class="space-y-1.5">
        <label class="text-sm font-semibold text-text">
          {{ t('instanceDetail.settings.jvmArgs.title') }}
        </label>
        <MInput
          :model-value="form.jvmArgs || ''"
          :placeholder="t('instanceDetail.settings.jvmArgs.placeholder')"
          @update:model-value="form.jvmArgs = $event"
        />
      </div>

      <div class="space-y-1.5">
        <label class="text-sm font-semibold text-text">
          {{ t('instanceDetail.settings.gameArgs.title') }}
        </label>
        <MInput
          :model-value="form.gameArgs || ''"
          :placeholder="t('instanceDetail.settings.gameArgs.placeholder')"
          @update:model-value="form.gameArgs = $event"
        />
      </div>
    </MCard>

    <!-- Window Title & Pre-launch Command & Quick Play -->
    <MCard class="p-6 space-y-5">
      <div class="space-y-1.5">
        <label class="text-sm font-semibold text-text">
          {{ t('instanceDetail.settings.windowTitle.title') }}
        </label>
        <MInput
          :model-value="form.windowTitle || ''"
          :placeholder="t('instanceDetail.settings.windowTitle.placeholder')"
          @update:model-value="form.windowTitle = $event"
        />
      </div>

      <div class="space-y-1.5">
        <label class="text-sm font-semibold text-text">
          {{ t('instanceDetail.settings.preLaunchCommand.title') }}
        </label>
        <MInput
          :model-value="form.preLaunchCommand || ''"
          :placeholder="t('instanceDetail.settings.preLaunchCommand.placeholder')"
          @update:model-value="form.preLaunchCommand = $event"
        />
      </div>

      <div class="space-y-1.5">
        <label class="text-sm font-semibold text-text">
          {{ t('instanceDetail.settings.serverQuickPlay.title') }}
        </label>
        <MInput
          :model-value="form.serverQuickPlay || ''"
          :placeholder="t('instanceDetail.settings.serverQuickPlay.placeholder')"
          @update:model-value="form.serverQuickPlay = $event"
        />
        <div class="text-xs text-textMuted">
          {{ t('instanceDetail.settings.serverQuickPlay.hint') }}
        </div>
      </div>
    </MCard>
  </div>
</template>

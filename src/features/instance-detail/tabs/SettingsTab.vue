<script setup lang="ts">
import { ref, watch, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import type { InstanceInfo, InstanceConfig } from '../../../ipc/client'
import { ipcClient } from '../../../ipc/client'
import { useInstancesStore } from '../../instances/store'
import {
  MSection,
  MSettingRow,
  MButton,
  MInput,
  MSelect,
  MSegmented,
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
  pinned: Boolean(props.instance.config?.pinned || props.instance.config?.starred),
  starred: Boolean(props.instance.config?.starred || props.instance.config?.pinned),
  cardType: props.instance.config?.cardType ?? null,
  isolation: props.instance.config?.isolation === 'instance' ? 'on' : (props.instance.config?.isolation || 'global'),
  java: props.instance.config?.java ?? null,
  memory: props.instance.config?.memory ?? null,
  jvmArgs: props.instance.config?.jvmArgs ?? null,
  gameArgs: props.instance.config?.gameArgs ?? null,
  windowTitle: props.instance.config?.windowTitle ?? null,
  preLaunchCommand: props.instance.config?.preLaunchCommand ?? null,
  loginRequirement: props.instance.config?.loginRequirement || 'any',
  serverQuickPlay: props.instance.config?.serverQuickPlay ?? null,
})

const javaList = ref<{ label: string; value: string | null }[]>([
  { label: t('instanceDetail.settings.useGlobal'), value: null },
])

watch(
  () => props.instance.config,
  (cfg) => {
    if (cfg) {
      form.value = {
        icon: cfg.icon || 'grass_block',
        category: cfg.category || 'Default',
        pinned: Boolean(cfg.pinned || cfg.starred),
        starred: Boolean(cfg.starred || cfg.pinned),
        cardType: cfg.cardType ?? null,
        isolation: cfg.isolation === 'instance' ? 'on' : (cfg.isolation || 'global'),
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

onMounted(async () => {
  try {
    const list = await ipcClient.javaScan()
    javaList.value = [
      { label: t('instanceDetail.settings.useGlobal'), value: null },
      ...list.map((j) => ({
        label: `${j.implementor || 'Java'} ${j.major} (${j.version})`,
        value: j.path,
      })),
    ]
  } catch {
    // fallback
  }
})

const isolationOptions = [
  { label: t('instanceDetail.settings.isolation.global'), value: 'global' },
  { label: t('instanceDetail.settings.isolation.on'), value: 'on' },
  { label: t('instanceDetail.settings.isolation.off'), value: 'off' },
]

const loginRequirementOptions = [
  { label: t('instanceDetail.settings.loginRequirement.any'), value: 'any' },
  { label: t('instanceDetail.settings.loginRequirement.microsoft'), value: 'microsoft' },
  { label: t('instanceDetail.settings.loginRequirement.thirdParty'), value: 'thirdParty' },
  { label: t('instanceDetail.settings.loginRequirement.microsoftOrThirdParty'), value: 'microsoftOrThirdParty' },
]
async function saveSettings() {
  saving.value = true
  try {
    await ipcClient.instanceUpdateConfig(props.instance.id, form.value)
    await instancesStore.fetchInstances()
    toast.success(t('common.save'))
  } catch (e: unknown) {
    toast.danger(String(e))
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <div class="space-y-6">
    <div class="flex items-center justify-between">
      <div>
        <h2 class="text-base font-bold text-fg">
          {{ t('instanceDetail.settings.title') }}
        </h2>
        <p class="text-xs text-fg-muted mt-0.5">
          {{ t('instanceDetail.settings.subtitle') }}
        </p>
      </div>
      <MButton
        variant="primary"
        size="sm"
        icon="i-lucide-save"
        :loading="saving"
        @click="saveSettings"
      >
        {{ t('instanceDetail.settings.save') }}
      </MButton>
    </div>

    <!-- Isolation & Login Requirement Section -->
    <MSection :title="t('instanceDetail.settings.isolation.title')" icon="i-lucide-shield">
      <div class="divide-y divide-line">
        <MSettingRow
          :label="t('instanceDetail.settings.isolation.title')"
          :hint="t('instanceDetail.settings.isolation.hint')"
        >
          <MSegmented
            v-model="form.isolation"
            :options="isolationOptions"
            size="sm"
          />
        </MSettingRow>

        <MSettingRow
          :label="t('instanceDetail.settings.javaLabel')"
          :hint="t('instanceDetail.settings.javaHint')"
        >
          <MSelect
            :model-value="form.java || ''"
            :options="javaList.map(j => ({ label: j.label, value: j.value || '' }))"
            size="sm"
            class="w-72"
            @update:model-value="form.java = $event ? String($event) : null"
          />
        </MSettingRow>

        <MSettingRow
          :label="t('instanceDetail.settings.loginRequirement.title')"
          :hint="t('instanceDetail.settings.loginRequirement.hint')"
        >
          <MSelect
            v-model="form.loginRequirement"
            :options="loginRequirementOptions"
            size="sm"
            class="w-56"
          />
        </MSettingRow>
      </div>
    </MSection>

    <!-- Memory & Java Args Section -->
    <MSection :title="t('instanceDetail.settings.memory.title')" icon="i-lucide-cpu">
      <div class="divide-y divide-line">
        <MSettingRow
          :label="t('instanceDetail.settings.memory.title')"
          :hint="t('instanceDetail.settings.memory.hint')"
        >
          <MInput
            :model-value="form.memory ? String(form.memory) : ''"
            type="number"
            placeholder="4096"
            size="sm"
            class="w-56"
            @update:model-value="form.memory = $event ? Number($event) : null"
          />
        </MSettingRow>

        <MSettingRow :label="t('instanceDetail.settings.jvmArgs.title')">
          <MInput
            :model-value="form.jvmArgs || ''"
            :placeholder="t('instanceDetail.settings.jvmArgs.placeholder')"
            size="sm"
            class="w-72"
            @update:model-value="form.jvmArgs = $event"
          />
        </MSettingRow>

        <MSettingRow :label="t('instanceDetail.settings.gameArgs.title')">
          <MInput
            :model-value="form.gameArgs || ''"
            :placeholder="t('instanceDetail.settings.gameArgs.placeholder')"
            size="sm"
            class="w-72"
            @update:model-value="form.gameArgs = $event"
          />
        </MSettingRow>
      </div>
    </MSection>

    <!-- Window Title & Pre-launch Command & Quick Play -->
    <MSection :title="t('instanceDetail.settings.windowTitle.title')" icon="i-lucide-app-window">
      <div class="divide-y divide-line">
        <MSettingRow :label="t('instanceDetail.settings.windowTitle.title')">
          <MInput
            :model-value="form.windowTitle || ''"
            :placeholder="t('instanceDetail.settings.windowTitle.placeholder')"
            size="sm"
            class="w-72"
            @update:model-value="form.windowTitle = $event"
          />
        </MSettingRow>

        <MSettingRow :label="t('instanceDetail.settings.preLaunchCommand.title')">
          <MInput
            :model-value="form.preLaunchCommand || ''"
            :placeholder="t('instanceDetail.settings.preLaunchCommand.placeholder')"
            size="sm"
            class="w-72"
            @update:model-value="form.preLaunchCommand = $event"
          />
        </MSettingRow>

        <MSettingRow
          :label="t('instanceDetail.settings.serverQuickPlay.title')"
          :hint="t('instanceDetail.settings.serverQuickPlay.hint')"
        >
          <MInput
            :model-value="form.serverQuickPlay || ''"
            :placeholder="t('instanceDetail.settings.serverQuickPlay.placeholder')"
            size="sm"
            class="w-72"
            @update:model-value="form.serverQuickPlay = $event"
          />
        </MSettingRow>
      </div>
    </MSection>
  </div>
</template>

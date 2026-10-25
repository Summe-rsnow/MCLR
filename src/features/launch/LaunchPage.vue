<script setup lang="ts">
import { onMounted, ref, computed } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useLaunchStore } from './store'
import { useInstancesStore } from '../instances/store'
import { useAccountsStore } from '../accounts/store'
import { useToast } from '../../ui/useToast'
import { ipcClient } from '../../ipc/client'
import {
  MPageHeader,
  MSection,
  MSettingRow,
  MCard,
  MButton,
  MSelect,
  MAvatar,
  MBadge,
  MProgress,
  MLoaderIcon,
  MBrandIcon,
  getAccountKindMeta,
} from '../../ui'

const { t } = useI18n()
const router = useRouter()
const toast = useToast()

const launchStore = useLaunchStore()
const instancesStore = useInstancesStore()
const accountsStore = useAccountsStore()

const isRepairing = ref(false)

onMounted(async () => {
  await Promise.all([
    instancesStore.fetchInstances(),
    accountsStore.fetchAccounts(),
    launchStore.fetchProcesses(),
  ])
  await launchStore.startListening()
})

const instanceOptions = computed(() => {
  return instancesStore.instances.map((i) => ({
    label: `${i.name} (${i.mcVersion})`,
    value: i.id,
  }))
})

const accountOptions = computed(() => {
  return accountsStore.accounts.map((a) => ({
    label: `${a.username} (${a.kind})`,
    value: a.id,
  }))
})

const stageLabel = computed(() => {
  return t(`launch.stage.${launchStore.currentStage}`)
})

const accountMeta = computed(() => {
  return getAccountKindMeta(launchStore.selectedAccount?.kind)
})

async function onLaunchClick() {
  if (launchStore.isLaunching) return
  try {
    await launchStore.startLaunch()
    toast.show({
      message: t('launch.stage.running'),
      type: 'success',
    })
  } catch (err: unknown) {
    if (launchStore.javaMissingMajor) {
      toast.show({
        message: t('launch.javaMissing.title'),
        type: 'danger',
      })
    } else {
      const msg = err instanceof Error ? err.message : String(err)
      toast.show({
        message: msg,
        type: 'danger',
      })
    }
  }
}

async function onRepairClick() {
  if (!launchStore.selectedInstance) return
  isRepairing.value = true
  try {
    await ipcClient.installRepair(launchStore.selectedInstance.id)
    toast.show({
      message: t('launch.repairSuccess'),
      type: 'success',
    })
  } catch (err: unknown) {
    toast.show({
      message: String(err),
      type: 'danger',
    })
  } finally {
    isRepairing.value = false
  }
}

async function onDownloadJavaClick() {
  try {
    const comp =
      launchStore.javaMissingMajor === '8'
        ? 'jre-legacy'
        : launchStore.javaMissingMajor === '21'
          ? 'java-runtime-delta'
          : 'java-runtime-gamma'
    await launchStore.downloadJavaRuntime(comp)
    toast.show({
      message: t('launch.stage.idle'),
      type: 'success',
    })
  } catch (err: unknown) {
    toast.show({
      message: String(err),
      type: 'danger',
    })
  }
}

function onAccountSelectChange(id: string | number) {
  accountsStore.selectAccount(String(id))
}

function onInstanceSelectChange(id: string | number) {
  launchStore.selectInstance(String(id))
}
</script>

<template>
  <div class="m-page">
    <!-- Header -->
    <MPageHeader
      :title="t('launch.title')"
      :description="launchStore.activeProcess ? `${t('launch.running')} (PID: ${launchStore.activeProcess.processId})` : undefined"
      icon="i-lucide-rocket"
    >
      <template #actions>
        <MButton
          v-if="launchStore.activeProcess"
          variant="tonal"
          size="sm"
          icon="i-lucide-terminal"
          @click="router.push('/logs')"
        >
          {{ t('launch.runningCard.viewLogs') }}
        </MButton>
        <MButton
          variant="text"
          size="sm"
          icon="i-lucide-sliders-horizontal"
          @click="router.push('/settings/game')"
        >
          {{ t('settingsGame.title') }}
        </MButton>
      </template>
    </MPageHeader>

    <!-- Java Missing Warning Banner -->
    <div
      v-if="launchStore.javaMissingMajor"
      class="bg-danger-soft text-danger-fg border border-danger/20 rounded-xl p-4 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4"
    >
      <div class="flex items-start gap-3">
        <div class="w-8 h-8 rounded-lg bg-danger/20 text-danger flex items-center justify-center shrink-0">
          <i class="i-lucide-alert-triangle text-lg" />
        </div>
        <div>
          <h4 class="font-semibold text-danger text-sm">
            {{ t('launch.javaMissing.title') }}
          </h4>
          <p class="text-xs text-fg-muted mt-0.5">
            {{ t('launch.javaMissing.message', { major: launchStore.javaMissingMajor }) }}
          </p>
        </div>
      </div>
      <MButton
        variant="danger"
        size="sm"
        :loading="launchStore.isDownloadingJava"
        icon="i-lucide-download"
        class="shrink-0"
        @click="onDownloadJavaClick"
      >
        {{ t('launch.javaMissing.downloadAction') }}
      </MButton>
    </div>

    <!-- Hero Card -->
    <MCard variant="tinted" padding="lg" class="relative overflow-hidden">
      <div class="flex flex-col md:flex-row items-start md:items-center justify-between gap-6">
        <div class="flex items-center gap-5 min-w-0">
          <MLoaderIcon
            :kind="launchStore.selectedInstance?.kind || (launchStore.selectedInstance?.loaders?.[0]?.toLowerCase() ?? 'vanilla')"
            :size="56"
          />
          <div class="min-w-0 space-y-1.5">
            <div class="flex items-center gap-2">
              <h2 class="text-2xl font-bold text-fg truncate">
                {{ instancesStore.instances.length === 0 ? t('instances.empty.title') : (launchStore.selectedInstance?.name || t('launch.noInstance')) }}
              </h2>
              <MButton
                v-if="launchStore.selectedInstance"
                variant="text"
                size="xs"
                icon="i-lucide-wrench"
                :loading="isRepairing"
                @click="onRepairClick"
              >
                {{ t('launch.repairInstance') }}
              </MButton>
            </div>
            <div v-if="launchStore.selectedInstance" class="flex flex-wrap items-center gap-2">
              <MBadge variant="primary">
                {{ launchStore.selectedInstance.mcVersion }}
              </MBadge>
              <MBadge
                v-if="launchStore.selectedInstance.loaders.length > 0"
                variant="neutral"
              >
                {{ launchStore.selectedInstance.loaders[0] }}
              </MBadge>
              <MBadge v-else variant="neutral">
                Vanilla
              </MBadge>
            </div>
          </div>
        </div>

        <!-- Launch CTA inside hero -->
        <div class="flex flex-col items-center sm:items-end w-full md:w-auto shrink-0 gap-2">
          <MButton
            v-if="instancesStore.instances.length === 0"
            size="lg"
            icon="i-lucide-download"
            variant="primary"
            class="h-14 px-10 rounded-2xl text-lg font-bold m-refract w-full sm:w-auto shadow-2"
            @click="router.push('/download')"
          >
            {{ t('launch.button.download') }}
          </MButton>
          <MButton
            v-else
            size="lg"
            icon="i-lucide-play"
            class="h-14 px-10 rounded-2xl text-lg font-bold m-refract w-full sm:w-auto shadow-2"
            :loading="launchStore.isLaunching"
            :disabled="!launchStore.selectedInstance || !launchStore.selectedAccount"
            @click="onLaunchClick"
          >
            <span v-if="launchStore.isLaunching">{{ stageLabel }}</span>
            <span v-else>{{ t('launch.launchButton') }}</span>
          </MButton>
          <div v-if="launchStore.isLaunching" class="w-full sm:w-56 flex flex-col gap-1 items-center sm:items-end">
            <MProgress :value="launchStore.launchProgress" size="sm" class="w-full" />
            <span class="text-xs text-fg-muted font-mono">{{ stageLabel }}</span>
          </div>
        </div>
      </div>
    </MCard>

    <!-- Selection Grid -->
    <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
      <!-- Instance Selection Section -->
      <MSection :title="t('launch.selectInstance')" icon="i-lucide-box">
        <template #actions>
          <MButton
            variant="text"
            size="xs"
            icon="i-lucide-plus"
            @click="router.push('/download')"
          >
            {{ t('launch.createInstance') }}
          </MButton>
        </template>
        <div class="space-y-4">
          <MSettingRow :label="t('launch.selectInstance')">
            <MSelect
              v-if="instancesStore.instances.length > 0"
              :model-value="launchStore.selectedInstance?.id || ''"
              :options="instanceOptions"
              size="sm"
              class="w-56"
              @update:model-value="onInstanceSelectChange"
            />
            <MButton
              v-else
              variant="tonal"
              size="sm"
              icon="i-lucide-plus"
              @click="router.push('/download')"
            >
              {{ t('launch.createInstance') }}
            </MButton>
          </MSettingRow>
        </div>
      </MSection>

      <!-- Account Selection Section -->
      <MSection :title="t('launch.selectAccount')" icon="i-lucide-user-round">
        <template #actions>
          <MButton
            variant="text"
            size="xs"
            icon="i-lucide-user-plus"
            @click="router.push('/accounts')"
          >
            {{ t('launch.addAccount') }}
          </MButton>
        </template>
        <div class="space-y-4">
          <!-- Account Preview Card -->
          <MCard v-if="launchStore.selectedAccount" variant="well" padding="sm" class="flex items-center gap-3">
            <MAvatar
              :name="launchStore.selectedAccount.username"
              :src="launchStore.selectedAccount.skinHeadDataUrl || undefined"
              :size="36"
            />
            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-1.5">
                <span class="font-semibold text-sm text-fg truncate">
                  {{ launchStore.selectedAccount.username }}
                </span>
                <MBrandIcon
                  v-if="accountMeta.brand"
                  :name="accountMeta.brand"
                  class="w-3.5 h-3.5 text-primary shrink-0"
                />
                <i
                  v-else
                  :class="[accountMeta.icon, 'text-xs text-fg-muted shrink-0']"
                />
              </div>
              <div class="flex items-center gap-2 text-xs text-fg-muted">
                <span class="capitalize">{{ launchStore.selectedAccount.kind }}</span>
                <span v-if="launchStore.selectedAccount.serverName" class="truncate">
                  • {{ launchStore.selectedAccount.serverName }}
                </span>
              </div>
            </div>
          </MCard>

          <MSettingRow :label="t('launch.selectAccount')">
            <MSelect
              v-if="accountsStore.accounts.length > 0"
              :model-value="launchStore.selectedAccount?.id || ''"
              :options="accountOptions"
              size="sm"
              class="w-56"
              @update:model-value="onAccountSelectChange"
            />
            <MButton
              v-else
              variant="tonal"
              size="sm"
              icon="i-lucide-user-plus"
              @click="router.push('/accounts')"
            >
              {{ t('launch.addAccount') }}
            </MButton>
          </MSettingRow>
        </div>
      </MSection>
    </div>
  </div>
</template>

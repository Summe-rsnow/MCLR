<script setup lang="ts">
import { onMounted, ref, computed } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useLaunchStore } from './store'
import { useInstancesStore } from '../instances/store'
import { useAccountsStore } from '../accounts/store'
import { useToast } from '../../ui/useToast'
import { ipcClient } from '../../ipc/client'
import MCard from '../../ui/MCard.vue'
import MButton from '../../ui/MButton.vue'
import MSelect from '../../ui/MSelect.vue'
import MAvatar from '../../ui/MAvatar.vue'
import MBadge from '../../ui/MBadge.vue'
import MProgress from '../../ui/MProgress.vue'

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
  <div class="h-full flex flex-col p-6 max-w-4xl mx-auto gap-6 overflow-y-auto">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-bold tracking-tight text-text">
          {{ t('launch.title') }}
        </h1>
        <p v-if="launchStore.activeProcess" class="text-xs text-success mt-1 flex items-center gap-1.5">
          <span class="w-2 h-2 rounded-full bg-success animate-pulse inline-block" />
          {{ t('launch.running') }} (PID: {{ launchStore.activeProcess.processId }})
        </p>
      </div>

      <div class="flex items-center gap-2">
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
      </div>
    </div>

    <!-- Java Missing Banner -->
    <div
      v-if="launchStore.javaMissingMajor"
      class="p-4 rounded-xl bg-danger/10 border border-danger/30 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4"
    >
      <div class="flex items-start gap-3">
        <div class="w-8 h-8 rounded-lg bg-danger/20 text-danger flex items-center justify-center shrink-0">
          <i class="i-lucide-alert-triangle text-lg" />
        </div>
        <div>
          <h4 class="font-semibold text-danger text-sm">
            {{ t('launch.javaMissing.title') }}
          </h4>
          <p class="text-xs text-text-muted mt-0.5">
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

    <!-- Selection Cards Grid -->
    <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
      <!-- Instance Card -->
      <MCard class="p-5 flex flex-col justify-between h-48 border border-outline/20">
        <div>
          <div class="flex items-center justify-between mb-3">
            <span class="text-xs font-medium text-text-muted uppercase tracking-wider">
              {{ t('launch.selectInstance') }}
            </span>
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

          <div v-if="launchStore.selectedInstance" class="flex items-start gap-3">
            <div class="w-12 h-12 rounded-xl bg-primary/10 text-primary flex items-center justify-center shrink-0">
              <i class="i-lucide-box text-2xl" />
            </div>
            <div class="min-w-0 flex-1">
              <h3 class="font-bold text-lg text-text truncate">
                {{ launchStore.selectedInstance.name }}
              </h3>
              <div class="flex items-center gap-2 mt-1">
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
          <div v-else class="text-sm text-text-muted py-4">
            {{ t('launch.noInstance') }}
          </div>
        </div>

        <div class="pt-2">
          <MSelect
            v-if="instancesStore.instances.length > 0"
            :model-value="launchStore.selectedInstance?.id || ''"
            :options="instanceOptions"
            size="sm"
            @update:model-value="onInstanceSelectChange"
          />
          <MButton
            v-else
            variant="tonal"
            size="sm"
            icon="i-lucide-plus"
            block
            @click="router.push('/download')"
          >
            {{ t('launch.createInstance') }}
          </MButton>
        </div>
      </MCard>

      <!-- Account Card -->
      <MCard class="p-5 flex flex-col justify-between h-48 border border-outline/20">
        <div>
          <div class="flex items-center justify-between mb-3">
            <span class="text-xs font-medium text-text-muted uppercase tracking-wider">
              {{ t('launch.selectAccount') }}
            </span>
            <MButton
              variant="text"
              size="xs"
              icon="i-lucide-user-plus"
              @click="router.push('/accounts')"
            >
              {{ t('launch.addAccount') }}
            </MButton>
          </div>

          <div v-if="launchStore.selectedAccount" class="flex items-start gap-3">
            <MAvatar
              :name="launchStore.selectedAccount.username"
              :src="launchStore.selectedAccount.skinHeadDataUrl || undefined"
              :size="48"
            />
            <div class="min-w-0 flex-1">
              <h3 class="font-bold text-lg text-text truncate">
                {{ launchStore.selectedAccount.username }}
              </h3>
              <div class="flex items-center gap-2 mt-1">
                <MBadge
                  :variant="
                    launchStore.selectedAccount.kind === 'microsoft'
                      ? 'primary'
                      : launchStore.selectedAccount.kind === 'authlib'
                        ? 'warning'
                        : 'neutral'
                  "
                >
                  {{ launchStore.selectedAccount.kind }}
                </MBadge>
                <span
                  v-if="launchStore.selectedAccount.serverName"
                  class="text-xs text-text-muted truncate"
                >
                  {{ launchStore.selectedAccount.serverName }}
                </span>
              </div>
            </div>
          </div>
          <div v-else class="text-sm text-text-muted py-4">
            {{ t('launch.noAccount') }}
          </div>
        </div>

        <div class="pt-2">
          <MSelect
            v-if="accountsStore.accounts.length > 0"
            :model-value="launchStore.selectedAccount?.id || ''"
            :options="accountOptions"
            size="sm"
            @update:model-value="onAccountSelectChange"
          />
          <MButton
            v-else
            variant="tonal"
            size="sm"
            icon="i-lucide-user-plus"
            block
            @click="router.push('/accounts')"
          >
            {{ t('launch.addAccount') }}
          </MButton>
        </div>
      </MCard>
    </div>

    <!-- Big Launch Button Area -->
    <div class="flex flex-col items-center justify-center py-6 gap-4">
      <MButton
        variant="primary"
        size="lg"
        :loading="launchStore.isLaunching"
        :disabled="!launchStore.selectedInstance || !launchStore.selectedAccount"
        icon="i-lucide-play"
        class="w-72 h-16 text-lg font-bold shadow-lg"
        @click="onLaunchClick"
      >
        <span v-if="launchStore.isLaunching">{{ stageLabel }}</span>
        <span v-else>{{ t('launch.launchButton') }}</span>
      </MButton>

      <div v-if="launchStore.isLaunching" class="w-72 flex flex-col gap-1.5 items-center">
        <MProgress :value="launchStore.launchProgress" class="w-full h-2" />
        <span class="text-xs text-text-muted">{{ stageLabel }}</span>
      </div>
    </div>
  </div>
</template>

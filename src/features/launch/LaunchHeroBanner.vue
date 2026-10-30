<script setup lang="ts">
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useLaunchStore } from './store'
import { useInstancesStore } from '../instances/store'
import { useToast } from '../../ui/useToast'
import { useDialog } from '../../ui/useDialog'
import { ipcClient } from '../../ipc/client'
import { awaitTask } from '../tasks/awaitTask'
import { useErrorText } from '../../composables/useErrorText'
import { MPageHeader, MCard, MButton, MBadge, MLoaderIcon, MProgress } from '../../ui'

defineProps<{
  isRepairing: boolean
}>()

const emit = defineEmits<{
  'update:isRepairing': [val: boolean]
  pickFolder: []
  contextmenu: [ev: MouseEvent]
}>()

const { t } = useI18n()
const router = useRouter()
const toast = useToast()
const dialog = useDialog()
const { formatError } = useErrorText()
const launchStore = useLaunchStore()
const instancesStore = useInstancesStore()

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
  emit('update:isRepairing', true)
  try {
    // The command only queues the task: verification and the downloads that
    // follow happen in the background, so success must wait for the terminal state.
    const taskId = await ipcClient.installRepair(launchStore.selectedInstance.id)
    const outcome = await awaitTask(taskId)
    if (outcome.state === 'failed' || outcome.state === 'canceled') {
      toast.show({
        message: outcome.error ? formatError(outcome.error) : t('launch.repairFailed'),
        type: 'danger',
      })
      return
    }
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
    emit('update:isRepairing', false)
  }
}

function onGoToJavaClick() {
  void router.push('/java')
}

async function onForceKillGame() {
  const proc = launchStore.activeProcess
  if (!proc) return

  const confirmed = await dialog.confirmDanger({
    title: t('logs.killConfirmTitle'),
    content: t('logs.killConfirmMessage', { pid: proc.processId }),
    confirmText: t('launch.runningCard.killProcess'),
  })

  if (!confirmed) return

  try {
    await launchStore.killProcess(proc.processId)
    toast.show({
      message: t('launch.runningCard.killProcess'),
      type: 'success',
    })
  } catch (err: unknown) {
    toast.show({
      message: String(err),
      type: 'danger',
    })
  }
}
</script>

<template>
  <div class="space-y-6">
    <!-- Header -->
    <MPageHeader
      :title="t('launch.title')"
      :description="
        launchStore.activeProcess
          ? `${t('launch.running')} (PID: ${launchStore.activeProcess.processId})`
          : undefined
      "
      icon="i-lucide-rocket"
    >
      <template #actions>
        <template v-if="launchStore.activeProcess">
          <MButton variant="tonal" size="sm" icon="i-lucide-terminal" @click="router.push('/logs')">
            {{ t('launch.runningCard.viewLogs') }}
          </MButton>
          <MButton
            variant="danger"
            size="sm"
            icon="i-lucide-skull"
            data-testid="hero-force-kill-btn"
            @click="onForceKillGame"
          >
            {{ t('launch.runningCard.killProcess') }}
          </MButton>
        </template>
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
      class="bg-danger-soft text-danger-fg border border-danger/20 rounded-xl p-4 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4 select-text"
    >
      <div class="flex items-start gap-3">
        <div
          class="w-8 h-8 rounded-lg bg-danger/20 text-danger flex items-center justify-center shrink-0"
        >
          <i class="i-lucide-triangle-alert text-lg" />
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
        icon="i-lucide-wrench"
        class="shrink-0"
        @click="onGoToJavaClick"
      >
        {{ t('launch.javaMissing.manageAction') }}
      </MButton>
    </div>

    <!-- Hero Card -->
    <MCard
      variant="tinted"
      padding="lg"
      class="relative"
      data-testid="launch-hero-card"
      @contextmenu="launchStore.selectedInstance && emit('contextmenu', $event)"
    >
      <div class="flex flex-col md:flex-row items-start md:items-center justify-between gap-6">
        <div class="flex items-center gap-5 min-w-0">
          <MLoaderIcon
            :kind="
              launchStore.selectedInstance?.kind ||
              (launchStore.selectedInstance?.loaders?.[0]?.toLowerCase() ?? 'vanilla')
            "
            :icon="launchStore.selectedInstance?.config?.icon"
            :size="56"
          />
          <div class="min-w-0 space-y-1.5">
            <div class="flex items-center gap-2">
              <h2 class="text-2xl font-bold text-fg truncate">
                {{
                  instancesStore.instances.length === 0
                    ? t('instances.empty.title')
                    : launchStore.selectedInstance?.name || t('launch.noInstance')
                }}
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
              <MBadge v-if="launchStore.selectedInstance.loaders.length > 0" variant="neutral">
                {{ launchStore.selectedInstance.loaders[0] }}
              </MBadge>
              <MBadge v-else variant="neutral"> Vanilla </MBadge>
            </div>
          </div>
        </div>

        <!-- Launch CTA inside hero -->
        <div class="flex flex-col items-center sm:items-end w-full md:w-auto shrink-0 gap-2">
          <!-- Warning / Guide prompt if no game folder configured -->
          <div
            v-if="!instancesStore.currentFolder"
            class="flex items-center gap-2 p-2 rounded-xl bg-warning/10 border border-warning/20 text-xs text-warning"
            data-testid="launch-no-folder-warning"
          >
            <span class="w-4 h-4 i-lucide-triangle-alert shrink-0" />
            <span>{{ t('launch.noFolderWarning') }}</span>
            <MButton
              variant="tonal"
              size="xs"
              icon="i-lucide-folder-plus"
              @click="emit('pickFolder')"
            >
              {{ t('launch.selectFolder') }}
            </MButton>
          </div>

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
            :disabled="
              !instancesStore.currentFolder ||
              !launchStore.selectedInstance ||
              !launchStore.selectedAccount
            "
            @click="onLaunchClick"
          >
            <span v-if="launchStore.isLaunching">{{ stageLabel }}</span>
            <span v-else>{{ t('launch.launchButton') }}</span>
          </MButton>
          <div
            v-if="launchStore.isLaunching"
            class="w-full sm:w-56 flex flex-col gap-1 items-center sm:items-end"
          >
            <MProgress :value="launchStore.launchProgress" size="sm" class="w-full" />
            <span class="text-xs text-fg-muted font-mono">{{ stageLabel }}</span>
          </div>
        </div>
      </div>
    </MCard>
  </div>
</template>

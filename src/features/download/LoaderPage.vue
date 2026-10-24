<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import {
  useInstallStore,
  type SelectedLoader,
  isLoaderCompatible,
} from './store'
import {
  ipcClient,
  type LoaderKind,
  type LoaderVersionEntry,
} from '../../ipc/client'
import {
  MButton,
  MIconButton,
  MCard,
  MInput,
  MSelect,
  MBadge,
  MDialog,
  MEmpty,
  useToast,
} from '../../ui'

const props = withDefaults(
  defineProps<{
    loader?: LoaderKind
  }>(),
  {
    loader: undefined,
  },
)

const route = useRoute()
const router = useRouter()
const { t } = useI18n()
const installStore = useInstallStore()
const toast = useToast()

const activeLoader = computed<LoaderKind>(() => {
  if (props.loader) return props.loader
  const param = route.params.loader as string | undefined
  if (param) return param as LoaderKind
  return 'fabric'
})

const selectedMcVersion = ref('')
const versions = ref<LoaderVersionEntry[]>([])
const loadingVersions = ref(false)

const installDialogOpen = ref(false)
const targetLoaderVersion = ref<LoaderVersionEntry | null>(null)
const instanceName = ref('')
const installing = ref(false)
const installError = ref('')

const loaderDisplayName = computed(() => {
  return t(`download.loaders.${activeLoader.value}`)
})

const compatibleMcVersions = computed(() => {
  return installStore.manifest.versions.filter((v) =>
    isLoaderCompatible(activeLoader.value as SelectedLoader, v.id),
  )
})

const mcOptions = computed(() => {
  return compatibleMcVersions.value.map((v) => ({
    label: `${v.id} (${v.versionType})`,
    value: v.id,
  }))
})

async function fetchLoaderList() {
  if (!selectedMcVersion.value) {
    versions.value = []
    return
  }
  loadingVersions.value = true
  try {
    versions.value = await ipcClient.loadersVersions(
      activeLoader.value,
      selectedMcVersion.value,
    )
  } catch {
    versions.value = []
    toast.danger(t('error.unknown'))
  } finally {
    loadingVersions.value = false
  }
}

async function onMcVersionChange(val: string | number) {
  selectedMcVersion.value = String(val)
  await fetchLoaderList()
}

onMounted(async () => {
  if (installStore.manifest.versions.length === 0) {
    await installStore.fetchManifest()
  }
  if (compatibleMcVersions.value.length > 0) {
    selectedMcVersion.value = compatibleMcVersions.value[0].id
    await fetchLoaderList()
  }
})

function openInstall(ver: LoaderVersionEntry) {
  targetLoaderVersion.value = ver
  instanceName.value = `${selectedMcVersion.value}-${loaderDisplayName.value}-${ver.version}`
  installError.value = ''
  installDialogOpen.value = true
}

const isNameValid = computed(() => {
  const trimmed = instanceName.value.trim()
  if (!trimmed) return false
  return /^[^\\/:*?"<>|!;]+$/.test(trimmed)
})

async function handleConfirmInstall() {
  if (!targetLoaderVersion.value || !isNameValid.value) {
    installError.value = t('download.dialog.nameError')
    return
  }
  installing.value = true
  installError.value = ''
  try {
    await ipcClient.loadersInstall(
      selectedMcVersion.value,
      instanceName.value.trim(),
      activeLoader.value,
      targetLoaderVersion.value.version,
      null,
    )
    toast.success(t('download.dialog.installStarted'))
    installDialogOpen.value = false
  } catch (err: unknown) {
    const code = err && typeof err === 'object' && 'code' in err ? String(err.code) : 'unknown'
    installError.value = t(`error.${code}`) || code
  } finally {
    installing.value = false
  }
}

function formatDate(iso?: string | null): string {
  if (!iso) return ''
  try {
    const d = new Date(iso)
    return d.toLocaleDateString()
  } catch {
    return iso
  }
}
</script>

<template>
  <div class="h-full flex flex-col p-6 overflow-hidden gap-6">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <div>
        <div class="flex items-center gap-3">
          <MIconButton
            icon="i-lucide-arrow-left"
            :title="t('common.cancel')"
            @click="router.push('/download')"
          />
          <h1 class="text-2xl font-bold text-[rgb(var(--c-text))]">
            {{ t('download.loaderPage.title', { name: loaderDisplayName }) }}
          </h1>
        </div>
        <p class="text-sm text-[rgb(var(--c-text-muted))] mt-1 ml-11">
          {{ t('download.loaderPage.subtitle', { name: loaderDisplayName }) }}
        </p>
      </div>

      <MIconButton
        icon="i-lucide-refresh-cw"
        :loading="loadingVersions"
        :title="t('download.loaderPage.refreshVersions')"
        @click="fetchLoaderList"
      />
    </div>

    <!-- MC Version Select Bar -->
    <MCard class="p-4 flex flex-wrap items-center justify-between gap-4 border border-[rgb(var(--c-outline)/0.2)]">
      <div class="flex items-center gap-3 flex-1 min-w-[280px]">
        <label class="text-sm font-medium text-[rgb(var(--c-text))] shrink-0">
          {{ t('download.loaderPage.selectMc') }}:
        </label>
        <div class="flex-1 max-w-xs">
          <MSelect
            :model-value="selectedMcVersion"
            :options="mcOptions"
            @update:model-value="onMcVersionChange"
          />
        </div>
      </div>
    </MCard>

    <!-- Version List -->
    <div class="flex-1 overflow-y-auto pr-1">
      <MCard v-if="versions.length === 0" class="p-8">
        <MEmpty
          :title="t('download.loaderPage.emptyVersions', { name: loaderDisplayName })"
          icon="i-lucide-package-x"
        />
      </MCard>

      <div v-else class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-3">
        <MCard
          v-for="v in versions"
          :key="v.version"
          class="p-4 flex items-center justify-between gap-3 border border-[rgb(var(--c-outline)/0.2)] hover:border-[rgb(var(--c-primary)/0.5)] transition-all duration-200"
        >
          <div class="min-w-0">
            <div class="flex items-center gap-2">
              <span class="font-semibold text-base font-mono text-[rgb(var(--c-text))] truncate">
                {{ v.version }}
              </span>
              <MBadge
                :variant="v.stable ? 'primary' : 'warning'"
                size="sm"
              >
                {{ v.stable ? t('download.loaderPage.badgeStable') : t('download.loaderPage.badgeSnapshot') }}
              </MBadge>
            </div>
            <div v-if="v.releaseTime" class="text-xs text-[rgb(var(--c-text-muted))] mt-1 truncate">
              {{ t('download.versionItem.releasedAt', { date: formatDate(v.releaseTime) }) }}
            </div>
          </div>

          <MButton
            variant="tonal"
            size="sm"
            icon="i-lucide-download"
            @click="openInstall(v)"
          >
            {{ t('download.loaderPage.installButton') }}
          </MButton>
        </MCard>
      </div>
    </div>

    <!-- Install Dialog -->
    <MDialog
      v-model:open="installDialogOpen"
      :title="t('download.dialog.installTitle')"
    >
      <div v-if="targetLoaderVersion" class="flex flex-col gap-4 py-2">
        <div>
          <label class="text-xs font-medium text-[rgb(var(--c-text-muted))] mb-1 block">
            {{ t('download.dialog.version') }}
          </label>
          <div class="p-2.5 rounded bg-[rgb(var(--c-surface-variant)/0.4)] font-mono text-sm text-[rgb(var(--c-text))]">
            {{ selectedMcVersion }} - {{ loaderDisplayName }} {{ targetLoaderVersion.version }}
          </div>
        </div>

        <div>
          <label class="text-xs font-medium text-[rgb(var(--c-text-muted))] mb-1 block">
            {{ t('download.dialog.instanceName') }}
          </label>
          <MInput
            v-model="instanceName"
            :placeholder="t('download.dialog.namePlaceholder')"
          />
        </div>

        <div v-if="installError" class="text-xs text-[rgb(var(--c-danger))]">
          {{ installError }}
        </div>
      </div>
      <template #footer>
        <MButton variant="text" @click="installDialogOpen = false">
          {{ t('common.cancel') }}
        </MButton>
        <MButton
          variant="primary"
          :loading="installing"
          :disabled="!isNameValid"
          @click="handleConfirmInstall"
        >
          {{ t('download.dialog.installAction') }}
        </MButton>
      </template>
    </MDialog>
  </div>
</template>

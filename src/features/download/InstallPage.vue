<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  useInstallStore,
  type SelectedLoader,
} from './store'
import type { VersionManifestEntry } from '../../ipc/client'
import {
  MButton,
  MIconButton,
  MCard,
  MInput,
  MSelect,
  MSegmented,
  MBadge,
  MDialog,
  MEmpty,
  MPageHeader,
  MLoaderIcon,
  useToast,
} from '../../ui'
import { useRouter } from 'vue-router'

const { t } = useI18n()
const installStore = useInstallStore()
const toast = useToast()
const router = useRouter()

const installing = ref(false)
const installError = ref('')

onMounted(async () => {
  if (installStore.manifest.versions.length === 0) {
    await installStore.fetchManifest()
  }
})

const filterOptions = computed(() => [
  { label: t('download.filter.release'), value: 'release' },
  { label: t('download.filter.snapshot'), value: 'snapshot' },
  { label: t('download.filter.old'), value: 'old' },
])


const loaderPickerOptions = computed(() => {
  return installStore.compatibleLoaders.map((k) => ({
    label: t(`download.loaders.${k}`),
    value: k,
  }))
})

const loaderVersionSelectOptions = computed(() => {
  return installStore.loaderVersions.map((v) => ({
    label: `${v.version} ${v.stable ? `(${t('download.loaders.stable')})` : `(${t('download.loaders.preview')})`}`,
    value: v.version,
  }))
})

const optifineComboOptions = computed(() => {
  const base = [{ label: t('download.loaders.optifineNone'), value: '' }]
  const opts = installStore.optifineComboVersions.map((v) => ({
    label: `OptiFine ${v.version}`,
    value: v.version,
  }))
  return [...base, ...opts]
})

async function onLoaderPick(loader: SelectedLoader) {
  await installStore.selectLoader(loader)
}

function handleVersionTypeBadge(type: string): { label: string; variant: 'success' | 'neutral' | 'warning' } {
  if (type === 'release') {
    return { label: t('download.versionItem.release'), variant: 'success' }
  }
  if (type === 'snapshot') {
    return { label: t('download.versionItem.snapshot'), variant: 'warning' }
  }
  return { label: t('download.versionItem.oldBeta'), variant: 'neutral' }
}

function formatDate(iso: string): string {
  try {
    const d = new Date(iso)
    return d.toLocaleDateString()
  } catch {
    return iso
  }
}

async function handleRefresh() {
  try {
    await installStore.fetchManifest(true)
    toast.success(t('common.refresh'))
  } catch {
    toast.danger(t('error.unknown'))
  }
}

function openInstallDialog(v: VersionManifestEntry) {
  installError.value = ''
  installStore.openInstallDialog(v)
}

async function handleConfirmInstall() {
  if (!installStore.isNameValid) {
    installError.value = t('download.dialog.nameError')
    return
  }

  installing.value = true
  installError.value = ''
  try {
    await installStore.install()
    toast.success(t('download.dialog.installStarted'))
  } catch (err: unknown) {
    const code = err && typeof err === 'object' && 'code' in err ? String(err.code) : 'unknown'
    installError.value = t(`error.${code}`) || code
  } finally {
    installing.value = false
  }
}
</script>

<template>
  <div class="m-page">
    <!-- Header -->
    <MPageHeader
      :title="t('download.title')"
      icon="i-lucide-cloud-download"
    >
      <template #actions>
        <MIconButton
          icon="i-lucide-refresh-cw"
          :loading="installStore.loading"
          :title="t('download.refresh')"
          @click="handleRefresh"
        />
      </template>
    </MPageHeader>

    <div v-if="installStore.manifest.latestRelease || installStore.manifest.latestSnapshot" class="text-xs text-fg-muted -mt-4 flex items-center gap-3">
      <span v-if="installStore.manifest.latestRelease">
        {{ t('download.latestRelease') }}:
        <span class="font-mono font-medium text-primary">
          {{ installStore.manifest.latestRelease }}
        </span>
      </span>
      <span v-if="installStore.manifest.latestSnapshot">
        · {{ t('download.latestSnapshot') }}:
        <span class="font-mono font-medium text-fg">
          {{ installStore.manifest.latestSnapshot }}
        </span>
      </span>
    </div>
    <!-- Filter Bar -->
    <div class="flex flex-wrap items-center justify-between gap-4">
      <MSegmented
        v-model="installStore.filterType"
        :options="filterOptions"
      />

      <div class="w-72">
        <MInput
          v-model="installStore.searchQuery"
          icon="i-lucide-search"
          :placeholder="t('download.search')"
        />
      </div>
    </div>

    <!-- Dedicated Loader Pages Quick Navigation -->
    <div class="flex items-center gap-2 overflow-x-auto pb-1 text-xs text-fg-muted">
      <span class="shrink-0 font-medium text-fg">
        {{ t('download.loaders.title') }}:
      </span>
      <button
        type="button"
        class="px-2.5 py-1 rounded-lg bg-sunken text-fg hover:bg-primary-soft hover:text-primary transition shrink-0"
        @click="router.push('/download/fabric')"
      >
        {{ t('download.tabs.fabric') }}
      </button>
      <button
        type="button"
        class="px-2.5 py-1 rounded-lg bg-sunken text-fg hover:bg-primary-soft hover:text-primary transition shrink-0"
        @click="router.push('/download/forge')"
      >
        {{ t('download.tabs.forge') }}
      </button>
      <button
        type="button"
        class="px-2.5 py-1 rounded-lg bg-sunken text-fg hover:bg-primary-soft hover:text-primary transition shrink-0"
        @click="router.push('/download/neoforge')"
      >
        {{ t('download.tabs.neoforge') }}
      </button>
      <button
        type="button"
        class="px-2.5 py-1 rounded-lg bg-sunken text-fg hover:bg-primary-soft hover:text-primary transition shrink-0"
        @click="router.push('/download/optifine')"
      >
        {{ t('download.tabs.optifine') }}
      </button>
      <button
        type="button"
        class="px-2.5 py-1 rounded-lg bg-sunken text-fg hover:bg-primary-soft hover:text-primary transition shrink-0"
        @click="router.push('/download/legacyfabric')"
      >
        {{ t('download.tabs.legacyfabric') }}
      </button>
      <button
        type="button"
        class="px-2.5 py-1 rounded-lg bg-sunken text-fg hover:bg-primary-soft hover:text-primary transition shrink-0"
        @click="router.push('/download/liteloader')"
      >
        {{ t('download.tabs.liteloader') }}
      </button>
      <button
        type="button"
        class="px-2.5 py-1 rounded-lg bg-sunken text-fg hover:bg-primary-soft hover:text-primary transition shrink-0"
        @click="router.push('/download/labymod')"
      >
        {{ t('download.tabs.labymod') }}
      </button>
      <button
        type="button"
        class="px-2.5 py-1 rounded-lg bg-sunken text-fg hover:bg-primary-soft hover:text-primary transition shrink-0"
        @click="router.push('/download/cleanroom')"
      >
        {{ t('download.tabs.cleanroom') }}
      </button>
    </div>

    <!-- Versions List -->
    <div class="flex-1 overflow-y-auto pr-1">
      <MCard v-if="installStore.filteredVersions.length === 0" class="p-8">
        <MEmpty
          :title="t('download.empty')"
          icon="i-lucide-box"
        />
      </MCard>

      <div v-else class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-3">
        <MCard
          v-for="v in installStore.filteredVersions"
          :key="v.id"
          hoverable
          class="p-4 flex items-center justify-between gap-3"
        >
          <div class="min-w-0">
            <div class="flex items-center gap-2">
              <span class="font-semibold text-base font-mono text-fg truncate">
                {{ v.id }}
              </span>
              <MBadge
                :variant="handleVersionTypeBadge(v.versionType).variant"
                size="sm"
              >
                {{ handleVersionTypeBadge(v.versionType).label }}
              </MBadge>
            </div>
            <div class="text-xs text-fg-muted mt-1 truncate">
              {{ t('download.versionItem.releasedAt', { date: formatDate(v.releaseTime) }) }}
            </div>
          </div>

          <MButton
            variant="tonal"
            size="sm"
            icon="i-lucide-download"
            @click="openInstallDialog(v)"
          >
            {{ t('download.dialog.installAction') }}
          </MButton>
        </MCard>
      </div>
    </div>
    <!-- Install Dialog -->
    <MDialog
      v-model:open="installStore.installDialogOpen"
      :title="t('download.dialog.installTitle')"
    >
      <div v-if="installStore.selectedVersion" class="flex flex-col gap-4 py-2">
        <div class="flex items-center gap-3 p-3 rounded-xl m-well">
          <MLoaderIcon :kind="installStore.selectedLoader === 'none' ? 'vanilla' : installStore.selectedLoader" :size="40" />
          <div class="min-w-0">
            <div class="text-xs font-medium text-fg-muted">
              {{ t('download.dialog.version') }}
            </div>
            <div class="font-mono text-sm font-semibold text-fg truncate">
              {{ installStore.selectedVersion.id }} ({{ installStore.selectedVersion.versionType }})
            </div>
          </div>
        </div>

        <!-- Loader Selection Chips -->
        <div>
          <label class="text-xs font-medium text-fg-muted mb-1.5 block">
            {{ t('download.loaders.title') }}
          </label>
          <div class="flex flex-wrap gap-1.5">
            <button
              v-for="opt in loaderPickerOptions"
              :key="opt.value"
              type="button"
              class="px-2.5 py-1 text-xs rounded-lg transition cursor-pointer border"
              :class="
                installStore.selectedLoader === opt.value
                  ? 'bg-primary text-primary-on border-primary font-medium'
                  : 'bg-sunken text-fg border-line hover:border-line-strong'
              "
              @click="onLoaderPick(opt.value)"
            >
              {{ opt.label }}
            </button>
          </div>
        </div>
        <!-- Loader Version Dropdown -->
        <div v-if="installStore.selectedLoader !== 'none'">
          <label class="text-xs font-medium text-fg-muted mb-1 block">
            {{ t('download.loaders.version') }}
          </label>
          <div v-if="installStore.loadingLoaderVersions" class="text-xs text-fg-muted py-1">
            {{ t('download.loaders.loadingVersions') }}
          </div>
          <div v-else-if="loaderVersionSelectOptions.length === 0" class="text-xs text-warning py-1">
            {{ t('download.loaders.noVersions') }}
          </div>
          <MSelect
            v-else
            v-model="installStore.selectedLoaderVersion"
            :options="loaderVersionSelectOptions"
          />
        </div>

        <!-- OptiFine Combo Selector (for Fabric or Forge) -->
        <div v-if="installStore.selectedLoader === 'fabric' || installStore.selectedLoader === 'forge'">
          <label class="text-xs font-medium text-fg-muted mb-1 block">
            {{ t('download.loaders.optifineCombo') }}
          </label>
          <MSelect
            v-model="installStore.selectedOptifineVersion"
            :options="optifineComboOptions"
          />
        </div>

        <div>
          <label class="text-xs font-medium text-fg-muted mb-1 block">
            {{ t('download.dialog.instanceName') }}
          </label>
          <MInput
            v-model="installStore.instanceName"
            :placeholder="t('download.dialog.namePlaceholder')"
          />
        </div>

        <div v-if="installError" class="text-xs text-danger">
          {{ installError }}
        </div>
      </div>
      <template #footer>
        <MButton variant="text" @click="installStore.installDialogOpen = false">
          {{ t('common.cancel') }}
        </MButton>
        <MButton
          variant="primary"
          :loading="installing"
          :disabled="!installStore.isNameValid"
          @click="handleConfirmInstall"
        >
          {{ t('download.dialog.installAction') }}
        </MButton>
      </template>
    </MDialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useInstallStore } from './store'
import type { VersionManifestEntry } from '../../ipc/client'
import {
  MButton,
  MIconButton,
  MCard,
  MInput,
  MSegmented,
  MBadge,
  MDialog,
  MEmpty,
  useToast,
} from '../../ui'

const { t } = useI18n()
const installStore = useInstallStore()
const toast = useToast()

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

function handleVersionTypeBadge(type: string): { label: string; variant: 'primary' | 'neutral' | 'warning' } {
  if (type === 'release') {
    return { label: t('download.versionItem.release'), variant: 'primary' }
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
  <div class="h-full flex flex-col p-6 overflow-hidden gap-6">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-bold text-[rgb(var(--c-text))]">
          {{ t('download.title') }}
        </h1>
        <p class="text-sm text-[rgb(var(--c-text-muted))] mt-1 flex items-center gap-3">
          <span v-if="installStore.manifest.latestRelease">
            {{ t('download.latestRelease') }}:
            <span class="font-mono font-medium text-[rgb(var(--c-primary))]">
              {{ installStore.manifest.latestRelease }}
            </span>
          </span>
          <span v-if="installStore.manifest.latestSnapshot">
            · {{ t('download.latestSnapshot') }}:
            <span class="font-mono font-medium text-[rgb(var(--c-text))]">
              {{ installStore.manifest.latestSnapshot }}
            </span>
          </span>
        </p>
      </div>

      <MIconButton
        icon="i-lucide-refresh-cw"
        :loading="installStore.loading"
        :title="t('download.refresh')"
        @click="handleRefresh"
      />
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
          class="p-4 flex items-center justify-between gap-3 border border-[rgb(var(--c-outline)/0.2)] hover:border-[rgb(var(--c-primary)/0.5)] transition-all duration-200"
        >
          <div class="min-w-0">
            <div class="flex items-center gap-2">
              <span class="font-semibold text-base font-mono text-[rgb(var(--c-text))] truncate">
                {{ v.id }}
              </span>
              <MBadge
                :variant="handleVersionTypeBadge(v.versionType).variant"
                size="sm"
              >
                {{ handleVersionTypeBadge(v.versionType).label }}
              </MBadge>
            </div>
            <div class="text-xs text-[rgb(var(--c-text-muted))] mt-1 truncate">
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
        <div>
          <label class="text-xs font-medium text-[rgb(var(--c-text-muted))] mb-1 block">
            {{ t('download.dialog.version') }}
          </label>
          <div class="p-2.5 rounded bg-[rgb(var(--c-surface-variant)/0.4)] font-mono text-sm text-[rgb(var(--c-text))]">
            {{ installStore.selectedVersion.id }} ({{ installStore.selectedVersion.versionType }})
          </div>
        </div>

        <div>
          <label class="text-xs font-medium text-[rgb(var(--c-text-muted))] mb-1 block">
            {{ t('download.dialog.instanceName') }}
          </label>
          <MInput
            v-model="installStore.instanceName"
            :placeholder="t('download.dialog.namePlaceholder')"
          />
        </div>

        <div v-if="installError" class="text-xs text-[rgb(var(--c-danger))]">
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

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { createResource } from '../../composables/useResource'
import { ipcClient, type VersionManifestEntry, type VersionManifestList } from '../../ipc/client'
import { MButton, MCard, MEmpty, MInput, MPageHeader, MSkeleton } from '../../ui'
import InstallConfigurator from './InstallConfigurator.vue'
import VersionCategoryTabs, { type LatestVersionItem } from './components/VersionCategoryTabs.vue'
import { classifyVersion } from './compat'
import { useInstancesStore } from '../instances/store'

const instancesStore = useInstancesStore()
const { t } = useI18n()

// Manifest resource with stale-while-revalidate & skeleton
const manifestResource = createResource<VersionManifestList>({
  key: 'version_manifest',
  fetcher: () => ipcClient.installManifest(false),
})

onMounted(async () => {
  await instancesStore.loadSettings()
  manifestResource.refresh()
})

const currentStep = ref<1 | 2>(1)
const stepDirection = ref<'forward' | 'back'>('forward')
const selectedVersion = ref('')
const searchQuery = ref('')

// Group collapse states
const openLatest = ref(true)
const openRelease = ref(true)
const openSnapshot = ref(false)
const openOld = ref(false)
const openAprilFools = ref(false)

const allVersions = computed<VersionManifestEntry[]>(() => {
  return manifestResource.data.value?.versions ?? []
})

// Filtered by search query
const filteredVersions = computed(() => {
  const query = searchQuery.value.trim().toLowerCase()
  if (!query) return allVersions.value
  return allVersions.value.filter(
    (v) => v.id.toLowerCase().includes(query) || v.releaseTime.toLowerCase().includes(query),
  )
})

// Group 1: Latest
const latestVersions = computed<LatestVersionItem[]>(() => {
  const manifest = manifestResource.data.value
  if (!manifest) return []
  const list: LatestVersionItem[] = []
  const rel = allVersions.value.find((v) => v.id === manifest.latestRelease)
  if (rel) list.push({ ...rel, category: 'release' })
  const snap = allVersions.value.find((v) => v.id === manifest.latestSnapshot)
  if (snap && snap.id !== rel?.id) list.push({ ...snap, category: 'snapshot' })
  return list
})

// Group 2: Release
const releaseVersions = computed(() => {
  return filteredVersions.value.filter((v) => classifyVersion(v) === 'release')
})

// Group 3: Snapshot
const snapshotVersions = computed(() => {
  return filteredVersions.value.filter((v) => classifyVersion(v) === 'snapshot')
})

// Group 4: Old
const oldVersions = computed(() => {
  return filteredVersions.value.filter((v) => classifyVersion(v) === 'old')
})

// Group 5: April Fools
const aprilFoolsVersions = computed(() => {
  return filteredVersions.value.filter((v) => classifyVersion(v) === 'aprilFools')
})

async function onPickAndSetGameFolder() {
  try {
    const dialog = await import('@tauri-apps/plugin-dialog')
    const selected = await dialog.open({ directory: true })
    if (!selected) return
    const path = Array.isArray(selected) ? selected[0] : selected
    if (!path) return
    const parts = path.replace(/\\/g, '/').split('/').filter(Boolean)
    const name = parts[parts.length - 1] || '.minecraft'
    await instancesStore.addFolder(name, path)
  } catch (err) {
    console.error('Failed to select game folder:', err)
  }
}

async function selectVersion(id: string) {
  if (!instancesStore.currentFolder) {
    await onPickAndSetGameFolder()
    if (!instancesStore.currentFolder) return
  }
  selectedVersion.value = id
  stepDirection.value = 'forward'
  currentStep.value = 2
}

function handleBack() {
  stepDirection.value = 'back'
  currentStep.value = 1
}
</script>

<template>
  <div class="p-6 max-w-5xl mx-auto flex flex-col gap-6">
    <MPageHeader
      :title="t('download.title')"
      :description="currentStep === 1 ? t('download.step1') : t('download.step2')"
    />

    <Transition :name="stepDirection === 'forward' ? 'm-tab-forward' : 'm-tab-back'" mode="out-in">
      <!-- STEP 1: Version List -->
      <div v-if="currentStep === 1" key="step1" class="flex flex-col gap-6">
        <!-- No game folder prompt banner -->
        <div
          v-if="!instancesStore.currentFolder"
          class="p-4 rounded-xl border border-warning/30 bg-warning/10 flex items-center justify-between gap-4"
        >
          <div class="flex items-center gap-3">
            <span class="w-6 h-6 i-lucide-alert-triangle text-warning shrink-0" />
            <div>
              <div class="font-semibold text-sm text-fg">{{ t('download.noFolderWarning') }}</div>
              <div class="text-xs text-fg-muted mt-0.5">{{ t('download.noFolderDesc') }}</div>
            </div>
          </div>
          <MButton
            variant="primary"
            size="sm"
            icon="i-lucide-folder-plus"
            @click="onPickAndSetGameFolder"
          >
            {{ t('download.selectFolder') }}
          </MButton>
        </div>
        <!-- Search bar -->
        <div class="flex items-center gap-4">
          <MInput v-model="searchQuery" :placeholder="t('download.search')" class="flex-1">
            <template #prefix>
              <span class="i-lucide-search text-fg-muted" />
            </template>
          </MInput>
          <MButton
            variant="text"
            size="md"
            :loading="manifestResource.refreshing.value"
            @click="manifestResource.refresh(true)"
          >
            <span class="i-lucide-refresh-cw mr-2" />
            {{ t('download.refresh') }}
          </MButton>
        </div>

        <!-- Skeleton loading state -->
        <div
          v-if="manifestResource.loading.value"
          class="flex flex-col gap-4"
          data-testid="install-skeleton"
        >
          <MCard v-for="i in 3" :key="i" class="p-4 flex flex-col gap-3">
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-3">
                <MSkeleton width="1.5rem" height="1.5rem" class="rounded" />
                <MSkeleton width="8rem" height="1.25rem" />
              </div>
              <MSkeleton width="1.25rem" height="1.25rem" class="rounded" />
            </div>
            <div class="flex flex-col gap-2 pt-2 border-t border-white/5">
              <div v-for="j in 2" :key="j" class="flex items-center justify-between py-1">
                <MSkeleton width="6rem" height="1rem" />
                <MSkeleton width="4rem" height="0.875rem" />
              </div>
            </div>
          </MCard>
        </div>

        <!-- Empty search result -->
        <MEmpty
          v-else-if="filteredVersions.length === 0"
          :title="t('download.empty')"
          icon="i-lucide-box"
        />

        <!-- Groups list -->
        <VersionCategoryTabs
          v-else
          v-model:open-latest="openLatest"
          v-model:open-release="openRelease"
          v-model:open-snapshot="openSnapshot"
          v-model:open-old="openOld"
          v-model:open-april-fools="openAprilFools"
          :search-query="searchQuery"
          :latest-versions="latestVersions"
          :release-versions="releaseVersions"
          :snapshot-versions="snapshotVersions"
          :old-versions="oldVersions"
          :april-fools-versions="aprilFoolsVersions"
          @select-version="selectVersion"
        />
      </div>

      <!-- STEP 2: Configurator -->
      <div v-else-if="currentStep === 2" key="step2">
        <InstallConfigurator
          mode="create"
          :mc-version="selectedVersion"
          @back="handleBack"
          @success="() => {}"
        />
      </div>
    </Transition>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { createResource } from '../../composables/useResource'
import {
  ipcClient,
  type VersionManifestEntry,
  type VersionManifestList,
} from '../../ipc/client'
import {
  MBadge,
  MButton,
  MCard,
  MCollapse,
  MEmpty,
  MInput,
  MPageHeader,
  MSkeletonList,
  MVirtualList,
} from '../../ui'
import InstallConfigurator from './InstallConfigurator.vue'
import { classifyVersion, getMcFoolName } from './compat'
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
    (v) =>
      v.id.toLowerCase().includes(query) ||
      v.releaseTime.toLowerCase().includes(query),
  )
})

interface LatestVersionItem extends VersionManifestEntry {
  category: 'release' | 'snapshot'
}

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
  return filteredVersions.value.filter(
    (v) => classifyVersion(v) === 'release',
  )
})

// Group 3: Snapshot
const snapshotVersions = computed(() => {
  return filteredVersions.value.filter(
    (v) => classifyVersion(v) === 'snapshot',
  )
})

// Group 4: Old
const oldVersions = computed(() => {
  return filteredVersions.value.filter(
    (v) => classifyVersion(v) === 'old',
  )
})

// Group 5: April Fools
const aprilFoolsVersions = computed(() => {
  return filteredVersions.value.filter(
    (v) => classifyVersion(v) === 'aprilFools',
  )
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

function formatDate(iso: string): string {
  try {
    return new Date(iso).toLocaleDateString()
  } catch {
    return iso
  }
}
</script>

<template>
  <div class="p-6 max-w-5xl mx-auto flex flex-col gap-6">
    <MPageHeader
      :title="t('download.title')"
      :description="
        currentStep === 1 ? t('download.step1') : t('download.step2')
      "
    />

    <Transition
      :name="stepDirection === 'forward' ? 'm-tab-forward' : 'm-tab-back'"
      mode="out-in"
    >
      <!-- STEP 1: Version List -->
      <div v-if="currentStep === 1" key="step1" class="flex flex-col gap-6">
        <!-- No game folder prompt banner -->
        <div
          v-if="!instancesStore.currentFolder"
          class="p-4 rounded-2xl border border-warning/30 bg-warning/10 flex flex-col sm:flex-row sm:items-center justify-between gap-4"
          data-testid="install-no-folder-prompt"
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
          <MInput
            v-model="searchQuery"
            :placeholder="t('download.search')"
            class="flex-1"
          >
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
        <div v-if="manifestResource.loading.value" class="flex flex-col gap-4">
          <MSkeletonList variant="card" :rows="6" />
        </div>

        <!-- Empty search result -->
        <MEmpty
          v-else-if="filteredVersions.length === 0"
          :title="t('download.empty')"
          icon="i-lucide-box"
        />

        <!-- Groups list -->
        <div v-else class="flex flex-col gap-4">
          <!-- Latest Group -->
          <MCard v-if="!searchQuery && latestVersions.length > 0" class="overflow-hidden">
            <button
              type="button"
              class="w-full p-4 flex items-center justify-between text-left hover:bg-white/5 transition-colors cursor-pointer"
              @click="openLatest = !openLatest"
            >
              <div class="flex items-center gap-3">
                <span class="i-lucide-sparkles text-xl text-primary" />
                <span class="font-bold text-base">{{ t('download.groups.latest') }}</span>
              </div>
              <span
                :class="[
                  'i-lucide-chevron-down text-lg text-fg-muted transition-transform duration-200',
                  openLatest ? 'rotate-180' : '',
                ]"
              />
            </button>
            <MCollapse :open="openLatest">
              <div class="divide-y divide-white/5 border-t border-white/5">
                <div
                  v-for="v in latestVersions"
                  :key="v.id"
                  class="p-4 flex items-center justify-between hover:bg-white/5 transition-colors cursor-pointer group"
                  @click="selectVersion(v.id)"
                >
                  <div class="flex items-center gap-3">
                    <div
                      :class="[
                        'w-10 h-10 rounded-lg flex items-center justify-center shrink-0 border transition-all duration-200 group-hover:scale-105 shadow-sm',
                        v.category === 'release'
                          ? 'bg-emerald-500/15 text-emerald-500 border-emerald-500/30'
                          : 'bg-amber-500/15 text-amber-500 border-amber-500/30',
                      ]"
                    >
                      <span
                        :class="[
                          'text-xl',
                          v.category === 'release' ? 'i-lucide-box' : 'i-lucide-command',
                        ]"
                      />
                    </div>
                    <div class="flex flex-col">
                      <div class="flex items-center gap-2">
                        <span class="font-bold text-base group-hover:text-primary transition-colors">
                          {{ v.id }}
                        </span>
                        <MBadge
                          size="sm"
                          :variant="v.category === 'release' ? 'primary' : 'neutral'"
                        >
                          {{ v.category === 'release' ? t('download.groups.release') : t('download.groups.snapshot') }}
                        </MBadge>
                      </div>
                      <span class="text-xs text-fg-muted mt-0.5">
                        {{
                          v.category === 'release'
                            ? t('download.latestReleaseDesc', { date: formatDate(v.releaseTime) })
                            : t('download.latestSnapshotDesc', { date: formatDate(v.releaseTime) })
                        }}
                      </span>
                    </div>
                  </div>
                  <MButton variant="text" size="sm">
                    <span class="i-lucide-arrow-right" />
                  </MButton>
                </div>
              </div>
            </MCollapse>
          </MCard>

          <!-- Release Group -->
          <MCard v-if="releaseVersions.length > 0" class="overflow-hidden">
            <button
              type="button"
              class="w-full p-4 flex items-center justify-between text-left hover:bg-white/5 transition-colors cursor-pointer"
              @click="openRelease = !openRelease"
            >
              <div class="flex items-center gap-3">
                <span class="i-lucide-circle-check text-xl text-emerald-400" />
                <span class="font-bold text-base">{{ t('download.groups.release') }} ({{ releaseVersions.length }})</span>
              </div>
              <span
                :class="[
                  'i-lucide-chevron-down text-lg text-fg-muted transition-transform duration-200',
                  openRelease ? 'rotate-180' : '',
                ]"
              />
            </button>
            <MCollapse :open="openRelease">
              <div class="border-t border-white/5">
                <MVirtualList
                  :items="releaseVersions"
                  :item-key="(v) => v.id"
                  :estimate-size="49"
                  :overscan="10"
                  max-height="65vh"
                >
                  <template #item="{ item: v }">
                    <div
                      class="px-4 py-3 flex items-center justify-between hover:bg-white/5 transition-colors cursor-pointer group border-b border-white/5"
                      @click="selectVersion(v.id)"
                    >
                      <div class="flex items-center gap-3">
                        <span class="font-semibold text-sm group-hover:text-primary transition-colors">
                          {{ v.id }}
                        </span>
                        <span class="text-xs text-fg-muted">
                          {{ formatDate(v.releaseTime) }}
                        </span>
                      </div>
                      <span class="i-lucide-chevron-right text-fg-muted group-hover:text-primary transition-colors" />
                    </div>
                  </template>
                </MVirtualList>
              </div>
            </MCollapse>
          </MCard>

          <!-- Snapshot Group -->
          <MCard v-if="snapshotVersions.length > 0" class="overflow-hidden">
            <button
              type="button"
              class="w-full p-4 flex items-center justify-between text-left hover:bg-white/5 transition-colors cursor-pointer"
              @click="openSnapshot = !openSnapshot"
            >
              <div class="flex items-center gap-3">
                <span class="i-lucide-flask-conical text-xl text-amber-400" />
                <span class="font-bold text-base">{{ t('download.groups.snapshot') }} ({{ snapshotVersions.length }})</span>
              </div>
              <span
                :class="[
                  'i-lucide-chevron-down text-lg text-fg-muted transition-transform duration-200',
                  openSnapshot ? 'rotate-180' : '',
                ]"
              />
            </button>
            <MCollapse :open="openSnapshot">
              <div class="border-t border-white/5">
                <MVirtualList
                  :items="snapshotVersions"
                  :item-key="(v) => v.id"
                  :estimate-size="49"
                  :overscan="10"
                  max-height="65vh"
                >
                  <template #item="{ item: v }">
                    <div
                      class="px-4 py-3 flex items-center justify-between hover:bg-white/5 transition-colors cursor-pointer group border-b border-white/5"
                      @click="selectVersion(v.id)"
                    >
                      <div class="flex items-center gap-3">
                        <span class="font-semibold text-sm group-hover:text-primary transition-colors">
                          {{ v.id }}
                        </span>
                        <span class="text-xs text-fg-muted">
                          {{ formatDate(v.releaseTime) }}
                        </span>
                      </div>
                      <span class="i-lucide-chevron-right text-fg-muted group-hover:text-primary transition-colors" />
                    </div>
                  </template>
                </MVirtualList>
              </div>
            </MCollapse>
          </MCard>

          <!-- Old Group -->
          <MCard v-if="oldVersions.length > 0" class="overflow-hidden">
            <button
              type="button"
              class="w-full p-4 flex items-center justify-between text-left hover:bg-white/5 transition-colors cursor-pointer"
              @click="openOld = !openOld"
            >
              <div class="flex items-center gap-3">
                <span class="i-lucide-archive text-xl text-stone-400" />
                <span class="font-bold text-base">{{ t('download.groups.old') }} ({{ oldVersions.length }})</span>
              </div>
              <span
                :class="[
                  'i-lucide-chevron-down text-lg text-fg-muted transition-transform duration-200',
                  openOld ? 'rotate-180' : '',
                ]"
              />
            </button>
            <MCollapse :open="openOld">
              <div class="border-t border-white/5">
                <MVirtualList
                  :items="oldVersions"
                  :item-key="(v) => v.id"
                  :estimate-size="49"
                  :overscan="10"
                  max-height="65vh"
                >
                  <template #item="{ item: v }">
                    <div
                      class="px-4 py-3 flex items-center justify-between hover:bg-white/5 transition-colors cursor-pointer group border-b border-white/5"
                      @click="selectVersion(v.id)"
                    >
                      <div class="flex items-center gap-3">
                        <span class="font-semibold text-sm group-hover:text-primary transition-colors">
                          {{ v.id }}
                        </span>
                        <MBadge size="sm" variant="neutral">
                          {{ v.versionType }}
                        </MBadge>
                        <span class="text-xs text-fg-muted">
                          {{ formatDate(v.releaseTime) }}
                        </span>
                      </div>
                      <span class="i-lucide-chevron-right text-fg-muted group-hover:text-primary transition-colors" />
                    </div>
                  </template>
                </MVirtualList>
              </div>
            </MCollapse>
          </MCard>

          <!-- April Fools Group -->
          <MCard v-if="aprilFoolsVersions.length > 0" class="overflow-hidden">
            <button
              type="button"
              class="w-full p-4 flex items-center justify-between text-left hover:bg-white/5 transition-colors cursor-pointer"
              @click="openAprilFools = !openAprilFools"
            >
              <div class="flex items-center gap-3">
                <span class="i-lucide-party-popper text-xl text-purple-400" />
                <span class="font-bold text-base">{{ t('download.groups.aprilFools') }} ({{ aprilFoolsVersions.length }})</span>
              </div>
              <span
                :class="[
                  'i-lucide-chevron-down text-lg text-fg-muted transition-transform duration-200',
                  openAprilFools ? 'rotate-180' : '',
                ]"
              />
            </button>
            <MCollapse :open="openAprilFools">
              <div class="border-t border-white/5">
                <MVirtualList
                  :items="aprilFoolsVersions"
                  :item-key="(v) => v.id"
                  :estimate-size="49"
                  :overscan="10"
                  max-height="65vh"
                >
                  <template #item="{ item: v }">
                    <div
                      class="px-4 py-3 flex items-center justify-between hover:bg-white/5 transition-colors cursor-pointer group border-b border-white/5"
                      @click="selectVersion(v.id)"
                    >
                      <div class="flex items-center gap-3">
                        <span class="font-semibold text-sm group-hover:text-primary transition-colors">
                          {{ v.id }}
                        </span>
                        <span v-if="getMcFoolName(v.id)" class="text-xs text-purple-400 font-medium">
                          {{ getMcFoolName(v.id) }}
                        </span>
                        <span class="text-xs text-fg-muted">
                          {{ formatDate(v.releaseTime) }}
                        </span>
                      </div>
                      <span class="i-lucide-chevron-right text-fg-muted group-hover:text-primary transition-colors" />
                    </div>
                  </template>
                </MVirtualList>
              </div>
            </MCollapse>
          </MCard>
        </div>
      </div>
      <!-- STEP 2: Configurator -->
      <div v-else-if="currentStep === 2" key="step2">
        <InstallConfigurator
          mode="create"
          :mc-version="selectedVersion"
          @back="handleBack"
          @success="handleBack"
        />
      </div>
    </Transition>
  </div>
</template>

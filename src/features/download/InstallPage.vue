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
import { isAprilFools } from './compat'

const { t } = useI18n()

// Manifest resource with stale-while-revalidate & skeleton
const manifestResource = createResource<VersionManifestList>({
  key: 'version_manifest',
  fetcher: () => ipcClient.installManifest(false),
})

onMounted(() => {
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

// Group 1: Latest
const latestVersions = computed(() => {
  const manifest = manifestResource.data.value
  if (!manifest) return []
  const list: VersionManifestEntry[] = []
  const rel = allVersions.value.find((v) => v.id === manifest.latestRelease)
  if (rel) list.push(rel)
  const snap = allVersions.value.find((v) => v.id === manifest.latestSnapshot)
  if (snap && snap.id !== rel?.id) list.push(snap)
  return list
})

// Group 2: Release
const releaseVersions = computed(() => {
  return filteredVersions.value.filter(
    (v) => v.versionType === 'release' && !isAprilFools(v.id, v.releaseTime),
  )
})

// Group 3: Snapshot
const snapshotVersions = computed(() => {
  return filteredVersions.value.filter(
    (v) => v.versionType === 'snapshot' && !isAprilFools(v.id, v.releaseTime),
  )
})

// Group 4: Old
const oldVersions = computed(() => {
  return filteredVersions.value.filter(
    (v) =>
      (v.versionType === 'old_beta' || v.versionType === 'old_alpha') &&
      !isAprilFools(v.id, v.releaseTime),
  )
})

// Group 5: April Fools
const aprilFoolsVersions = computed(() => {
  return filteredVersions.value.filter((v) =>
    isAprilFools(v.id, v.releaseTime),
  )
})

function selectVersion(id: string) {
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
                <MBadge size="sm" variant="primary">{{ latestVersions.length }}</MBadge>
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
                    <span class="font-semibold text-base group-hover:text-primary transition-colors">
                      {{ v.id }}
                    </span>
                    <MBadge
                      size="sm"
                      :variant="v.versionType === 'release' ? 'primary' : 'neutral'"
                    >
                      {{ v.versionType === 'release' ? t('download.configurator.releaseBadge') : t('download.configurator.snapshotBadge') }}
                    </MBadge>
                    <span class="text-xs text-fg-muted">
                      {{ formatDate(v.releaseTime) }}
                    </span>
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
                <span class="i-lucide-check-circle text-xl text-emerald-400" />
                <span class="font-bold text-base">{{ t('download.groups.release') }}</span>
                <MBadge size="sm" variant="neutral">{{ releaseVersions.length }}</MBadge>
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
                  :estimate-size="56"
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
                <span class="font-bold text-base">{{ t('download.groups.snapshot') }}</span>
                <MBadge size="sm" variant="neutral">{{ snapshotVersions.length }}</MBadge>
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
                  :estimate-size="56"
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

          <!-- April Fools Group -->
          <MCard v-if="aprilFoolsVersions.length > 0" class="overflow-hidden">
            <button
              type="button"
              class="w-full p-4 flex items-center justify-between text-left hover:bg-white/5 transition-colors cursor-pointer"
              @click="openAprilFools = !openAprilFools"
            >
              <div class="flex items-center gap-3">
                <span class="i-lucide-party-popper text-xl text-purple-400" />
                <span class="font-bold text-base">{{ t('download.groups.aprilFools') }}</span>
                <MBadge size="sm" variant="neutral">{{ aprilFoolsVersions.length }}</MBadge>
              </div>
              <span
                :class="[
                  'i-lucide-chevron-down text-lg text-fg-muted transition-transform duration-200',
                  openAprilFools ? 'rotate-180' : '',
                ]"
              />
            </button>
            <MCollapse :open="openAprilFools">
              <div class="border-t border-white/5 divide-y divide-white/5">
                <div
                  v-for="v in aprilFoolsVersions"
                  :key="v.id"
                  class="px-4 py-3 flex items-center justify-between hover:bg-white/5 transition-colors cursor-pointer group"
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
                <span class="font-bold text-base">{{ t('download.groups.old') }}</span>
                <MBadge size="sm" variant="neutral">{{ oldVersions.length }}</MBadge>
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
                  :estimate-size="56"
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

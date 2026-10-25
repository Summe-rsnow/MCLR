<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useCompStore } from './store'
import { useInstancesStore } from '../instances/store'
import type { CompType, CompPlatform, CompSort } from '../../ipc/client'
import {
  MTabs,
  MSegmented,
  MInput,
  MSelect,
  MButton,
  MIconButton,
  MCard,
  MBadge,
  MEmpty,
  MSkeleton,
  MPageHeader,
  MBrandIcon,
  compTypeMeta,
  type CompTypeKey,
} from '../../ui'
import ModpackImportDialog from './ModpackImportDialog.vue'

const showImportDialog = ref(false)
const router = useRouter()
const route = useRoute()
const { t } = useI18n()
const compStore = useCompStore()

const instancesStore = useInstancesStore()

const targetInstanceId = ref<string>('')
const targetInstance = computed(() => {
  if (!targetInstanceId.value) return null
  return (
    instancesStore.instances.find(
      (i) => i.id === targetInstanceId.value || i.name === targetInstanceId.value,
    ) || null
  )
})

function isForgeUnder114(mcVer: string): boolean {
  const parts = mcVer.split('.').map((p) => parseInt(p, 10))
  if (parts.length >= 2 && parts[0] === 1) {
    return (parts[1] ?? 0) < 14
  }
  return false
}

function clearTargetInstance() {
  targetInstanceId.value = ''
  const q = { ...route.query }
  delete q.instance
  router.replace({ query: q }).catch(() => {})
}
const tabs = computed(() => [
  { key: 'mod', label: t('comp.tabs.mod') },
  { key: 'modpack', label: t('comp.tabs.modpack') },
  { key: 'resourcepack', label: t('comp.tabs.resourcepack') },
  { key: 'shader', label: t('comp.tabs.shader') },
  { key: 'datapack', label: t('comp.tabs.datapack') },
  { key: 'world', label: t('comp.tabs.world') },
  { key: 'favorites', label: t('comp.tabs.favorites') },
])

const platformOptions = computed(() => [
  { label: t('comp.platform.modrinth'), value: 'modrinth' },
  { label: t('comp.platform.curseforge'), value: 'curseforge' },
])

const sortOptions = computed(() => [
  { label: t('comp.search.sorts.relevance'), value: 'relevance' },
  { label: t('comp.search.sorts.downloads'), value: 'downloads' },
  { label: t('comp.search.sorts.follows'), value: 'follows' },
  { label: t('comp.search.sorts.newest'), value: 'newest' },
  { label: t('comp.search.sorts.updated'), value: 'updated' },
])

const mcVersionOptions = computed(() => [
  { label: t('comp.search.allMcVersions'), value: '' },
  { label: '1.21', value: '1.21' },
  { label: '1.20.4', value: '1.20.4' },
  { label: '1.20.2', value: '1.20.2' },
  { label: '1.20.1', value: '1.20.1' },
  { label: '1.19.4', value: '1.19.4' },
  { label: '1.18.2', value: '1.18.2' },
  { label: '1.16.5', value: '1.16.5' },
  { label: '1.12.2', value: '1.12.2' },
  { label: '1.7.10', value: '1.7.10' },
])

const loaderOptions = computed(() => [
  { label: t('comp.search.allLoaders'), value: '' },
  { label: 'Fabric', value: 'fabric' },
  { label: 'Forge', value: 'forge' },
  { label: 'NeoForge', value: 'neoforge' },
  { label: 'Quilt', value: 'quilt' },
  { label: 'OptiFine', value: 'optifine' },
  { label: 'Iris', value: 'iris' },
])

const searchDebounceTimer = ref<ReturnType<typeof setTimeout> | null>(null)

function onSearchInput(val: string): void {
  compStore.query = val
  if (searchDebounceTimer.value) {
    clearTimeout(searchDebounceTimer.value)
  }
  searchDebounceTimer.value = setTimeout(() => {
    if (compStore.activeTab !== 'favorites') {
      compStore.search(true)
    }
  }, 350)
}

function onTabChange(tabValue: string): void {
  compStore.activeTab = tabValue as CompType | 'favorites'
  if (tabValue === 'favorites') {
    if (router) router.replace('/comp/favorites').catch(() => {})
  } else {
    compStore.compType = tabValue as CompType
    if (router) router.replace(`/comp/${tabValue}`).catch(() => {})
    compStore.search(true)
  }
}

function onPlatformChange(val: string): void {
  compStore.platform = val as CompPlatform
  if (compStore.activeTab !== 'favorites') {
    compStore.search(true)
  }
}

function onSortChange(val: string | number): void {
  compStore.sort = String(val) as CompSort
  if (compStore.activeTab !== 'favorites') {
    compStore.search(true)
  }
}

function onMcVersionChange(val: string | number): void {
  const s = String(val)
  compStore.mcVersion = s || null
  if (compStore.activeTab !== 'favorites') {
    compStore.search(true)
  }
}

function onLoaderChange(val: string | number): void {
  const s = String(val)
  compStore.loader = s || null
  if (compStore.activeTab !== 'favorites') {
    compStore.search(true)
  }
}

function navigateToProject(platform: string, id: string): void {
  const q: Record<string, string> = {}
  if (targetInstanceId.value) q.instance = targetInstanceId.value
  if (router) router.push({ path: `/comp/project/${platform}/${id}`, query: q }).catch(() => {})
}

function formatNumber(num: number): string {
  if (num >= 1000000) {
    return (num / 1000000).toFixed(1) + 'M'
  }
  if (num >= 1000) {
    return (num / 1000).toFixed(1) + 'K'
  }
  return num.toString()
}

onMounted(async () => {
  if (instancesStore.instances.length === 0 && typeof instancesStore.fetchInstances === 'function') {
    await instancesStore.fetchInstances()
  }

  if (route?.query?.instance) {
    targetInstanceId.value = String(route.query.instance)
    const inst = targetInstance.value
    if (inst) {
      compStore.mcVersion = inst.mcVersion || null
      const primaryLoader = inst.loaders[0]?.toLowerCase()
      if (primaryLoader === 'forge' && isForgeUnder114(inst.mcVersion)) {
        compStore.loader = null
      } else {
        compStore.loader = primaryLoader || null
      }
    }
  }

  if (route?.query?.q) {
    compStore.query = String(route.query.q)
  }
  await compStore.loadFavorites()
  const path = route?.path || '/comp/mod'
  if (path === '/comp/favorites') {
    compStore.activeTab = 'favorites'
  } else {
    const match = path.match(/^\/comp\/(mod|modpack|resourcepack|shader|datapack|world)/)
    if (match && match[1]) {
      compStore.activeTab = match[1] as CompType
      compStore.compType = match[1] as CompType
    } else {
      compStore.activeTab = 'mod'
      compStore.compType = 'mod'
    }
    await compStore.search(true)
  }
})

watch(
  () => route?.path,
  (newPath) => {
    if (!newPath) return
    if (newPath === '/comp/favorites') {
      compStore.activeTab = 'favorites'
    } else {
      const match = newPath.match(/^\/comp\/(mod|modpack|resourcepack|shader|datapack|world)/)
      if (match && match[1]) {
        const nextType = match[1] as CompType
        if (compStore.activeTab !== nextType || compStore.compType !== nextType) {
          compStore.activeTab = nextType
          compStore.compType = nextType
          compStore.search(true)
        }
      }
    }
  },
)
</script>

<template>
  <div class="m-page">
    <MPageHeader
      :title="t('comp.title')"
      :description="t('comp.subtitle')"
      icon="i-lucide-store"
    >
      <template #actions>
        <div v-if="compStore.activeTab === 'modpack'">
          <MButton
            variant="primary"
            size="sm"
            icon="i-lucide-file-up"
            @click="showImportDialog = true"
          >
            {{ t('comp.modpackImport.button') }}
          </MButton>
        </div>
      </template>
    </MPageHeader>

    <!-- Tabs Bar -->
    <div class="border-b border-line pb-2">
      <MTabs
        :model-value="compStore.activeTab"
        :tabs="tabs"
        @update:model-value="onTabChange"
      />
    </div>

    <!-- Filter Bar (visible when not in favorites tab) -->
    <MCard
      v-if="compStore.activeTab !== 'favorites'"
      padding="sm"
      class="flex flex-wrap items-center gap-3 sticky top-0 z-10"
    >
      <div
        v-if="targetInstance"
        class="flex items-center space-x-1.5 px-2.5 py-1 rounded-full bg-primary/10 border border-primary/30 text-xs text-primary font-medium shrink-0"
      >
        <span class="i-lucide-download w-3.5 h-3.5" />
        <span>{{ t('comp.downloadForInstance', { instance: targetInstance.name }) }}</span>
        <button
          class="hover:opacity-75 transition ml-1"
          @click="clearTargetInstance"
        >
          <span class="i-lucide-x w-3.5 h-3.5" />
        </button>
      </div>
      <MSegmented
        :model-value="compStore.platform"
        :options="platformOptions"
        @update:model-value="onPlatformChange"
      />

      <div class="flex-1 min-w-[200px]">
        <MInput
          :model-value="compStore.query"
          :placeholder="t('comp.search.placeholder')"
          clearable
          @update:model-value="onSearchInput"
        />
      </div>

      <div class="w-36">
        <MSelect
          :model-value="compStore.mcVersion || ''"
          :options="mcVersionOptions"
          @update:model-value="onMcVersionChange"
        />
      </div>

      <div class="w-36">
        <MSelect
          :model-value="compStore.loader || ''"
          :options="loaderOptions"
          @update:model-value="onLoaderChange"
        />
      </div>

      <div class="w-36">
        <MSelect
          :model-value="compStore.sort"
          :options="sortOptions"
          @update:model-value="onSortChange"
        />
      </div>
    </MCard>
    <!-- Content Area: Favorites Tab -->
    <div v-if="compStore.activeTab === 'favorites'" class="flex-1 space-y-3">
      <div v-if="compStore.favorites.length === 0" class="py-16">
        <MEmpty
          :title="t('comp.favorites.empty')"
          :description="t('comp.favorites.emptyDesc')"
        />
      </div>
      <div v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        <MCard
          v-for="fav in compStore.favorites"
          :key="fav.platform + '-' + fav.projectId"
          hoverable
          class="cursor-pointer p-4 flex flex-col justify-between"
          @click="navigateToProject(fav.platform, fav.projectId)"
        >
          <div class="flex items-start space-x-3">
            <img
              v-if="fav.iconUrl"
              :src="fav.iconUrl"
              class="w-14 h-14 rounded-xl object-cover bg-sunken shrink-0"
              alt=""
             loading="lazy" decoding="async"/>
            <div
              v-else
              class="w-14 h-14 rounded-xl bg-primary-soft text-primary-soft-fg flex items-center justify-center shrink-0"
            >
              <span :class="compTypeMeta[fav.compType as CompTypeKey]?.icon || 'i-lucide-box'" class="w-7 h-7 inline-block" />
            </div>
            <div class="flex-1 min-w-0">
              <div class="flex items-center justify-between">
                <h4 class="font-bold text-base text-fg truncate">{{ fav.title }}</h4>
                <MIconButton
                  icon="i-lucide-heart-crack"
                  size="sm"
                  class="text-danger hover:opacity-80"
                  @click.stop="compStore.toggleFavorite(fav)"
                />
              </div>
              <div class="flex items-center space-x-2 mt-1">
                <MBadge variant="neutral">
                  <MBrandIcon v-if="fav.platform === 'modrinth' || fav.platform === 'curseforge'" :name="fav.platform" class="w-3.5 h-3.5 mr-1" />
                  {{ fav.platform }}
                </MBadge>
                <MBadge variant="neutral">{{ fav.compType }}</MBadge>
              </div>
            </div>
          </div>
        </MCard>
      </div>
    </div>

    <!-- Content Area: Regular Search Hits -->
    <div v-else class="flex-1 space-y-3">
      <!-- Loading Skeletons -->
      <div
        v-if="compStore.loading && compStore.hits.length === 0"
        class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 py-4"
      >
        <MSkeleton v-for="n in 6" :key="n" class="h-36 rounded-xl" />
      </div>

      <!-- Empty State -->
      <div
        v-else-if="!compStore.loading && compStore.hits.length === 0"
        class="py-16"
      >
        <MEmpty :title="t('comp.search.noResults')" />
      </div>

      <!-- Results Grid -->
      <div v-else class="space-y-4">
        <div class="text-xs text-fg-muted">
          {{ t('comp.search.totalHits', { count: compStore.totalHits.toString() }) }}
        </div>

        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          <MCard
            v-for="project in compStore.hits"
            :key="project.platform + '-' + project.id"
            hoverable
            class="cursor-pointer p-4 flex flex-col justify-between"
            @click="navigateToProject(project.platform, project.id)"
          >
            <div>
              <div class="flex items-start space-x-3">
                <img
                  v-if="project.iconUrl"
                  :src="project.iconUrl"
                  class="w-14 h-14 rounded-xl object-cover bg-sunken shrink-0"
                  alt=""
                 loading="lazy" decoding="async"/>
                <div
                  v-else
                  class="w-14 h-14 rounded-xl bg-primary-soft text-primary-soft-fg flex items-center justify-center shrink-0"
                >
                  <span :class="compTypeMeta[compStore.activeTab as CompTypeKey]?.icon || 'i-lucide-box'" class="w-7 h-7 inline-block" />
                </div>

                <div class="flex-1 min-w-0">
                  <div class="flex items-center justify-between">
                    <h3 class="font-bold text-base text-fg truncate">{{ project.title }}</h3>
                    <MIconButton
                      icon="i-lucide-heart"
                      size="sm"
                      :class="
                        compStore.isFavorite(project.platform, project.id)
                          ? 'text-danger hover:opacity-80'
                          : 'text-fg-muted hover:text-fg'
                      "
                      @click.stop="compStore.toggleFavorite(project)"
                    />
                  </div>
                  <div class="text-xs text-fg-muted mt-0.5 truncate flex items-center gap-1.5">
                    <MBrandIcon v-if="project.platform === 'modrinth' || project.platform === 'curseforge'" :name="project.platform" class="w-3 h-3 text-fg-muted" />
                    <span>{{ t('comp.project.author', { author: project.author }) }}</span>
                  </div>
                </div>
              </div>

              <p class="text-xs text-fg-muted mt-3 line-clamp-2 h-8 leading-4">
                {{ project.summary }}
              </p>
            </div>

            <div class="mt-4 pt-3 border-t border-line flex items-center justify-between text-xs text-fg-muted">
              <div class="flex items-center space-x-3">
                <span class="flex items-center space-x-1">
                  <i class="i-lucide-download text-sm" />
                  <span>{{ formatNumber(project.downloads) }}</span>
                </span>
                <span class="flex items-center space-x-1">
                  <i class="i-lucide-heart text-sm" />
                  <span>{{ formatNumber(project.follows) }}</span>
                </span>
              </div>
              <div class="flex items-center space-x-1">
                <MBadge
                  v-for="cat in project.categories.slice(0, 2)"
                  :key="cat"
                  variant="neutral"
                >
                  {{ cat }}
                </MBadge>
              </div>
            </div>
          </MCard>
        </div>

        <!-- Load More Button -->
        <div
          v-if="compStore.hits.length < compStore.totalHits"
          class="flex justify-center pt-4"
        >
          <MButton
            variant="tonal"
            :loading="compStore.loading"
            @click="compStore.loadMore()"
          >
            {{ t('comp.search.loadMore') }}
          </MButton>
        </div>
      </div>
    </div>

    <!-- Modpack Import Dialog -->
    <ModpackImportDialog
      :open="showImportDialog"
      @update:open="showImportDialog = $event"
    />
  </div>
</template>

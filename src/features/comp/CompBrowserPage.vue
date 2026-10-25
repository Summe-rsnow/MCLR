<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useCompStore } from './store'
import { useInstancesStore } from '../instances/store'
import { useLaunchStore } from '../launch/store'
import type { CompType, CompPlatform, CompSort } from '../../ipc/client'
import {
  MTabs,
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
  useToast,
} from '../../ui'
import type { CompFavoriteEntry, CompProject } from '../../ipc/client'

const router = useRouter()
const route = useRoute()
const { t } = useI18n()
const toast = useToast()
const compStore = useCompStore()
const instancesStore = useInstancesStore()
const launchStore = useLaunchStore()
const targetInstanceId = ref<string>('')

function isForgeUnder114(mcVer: string): boolean {
  const parts = mcVer.split('.').map((p) => parseInt(p, 10))
  if (parts.length >= 2 && parts[0] === 1) {
    return (parts[1] ?? 0) < 14
  }
  return false
}
function applyTargetInstance(inst: { kind?: string; mcVersion?: string; loaders?: string[] } | null, triggerSearch = true) {
  if (inst) {
    compStore.mcVersion = inst.mcVersion || null
    let primaryLoader: string | null = null
    const raw = (inst.kind && inst.kind.toLowerCase() !== 'vanilla' ? inst.kind : inst.loaders?.[0])?.toLowerCase()
    if (raw) {
      for (const known of ['fabric', 'forge', 'neoforge', 'quilt', 'optifine', 'iris']) {
        if (raw.includes(known)) {
          primaryLoader = known
          break
        }
      }
      if (!primaryLoader) {
        primaryLoader = raw.split(/\s+/)[0] || null
      }
    }
    if (primaryLoader === 'forge' && inst.mcVersion && isForgeUnder114(inst.mcVersion)) {
      compStore.loader = null
    } else {
      compStore.loader = primaryLoader
    }
  }
  if (triggerSearch && compStore.activeTab !== 'favorites') {
    compStore.search(true)
  }
}

function onTargetInstanceChange(val: string | number) {
  const id = String(val || '')
  targetInstanceId.value = id
  if (id) {
    const inst = instancesStore.instances.find((i) => i.id === id || i.name === id) || null
    applyTargetInstance(inst, true)
    if (router && route) {
      router.replace({ query: { ...route.query, instance: id } }).catch(() => {})
    }
  } else {
    clearTargetInstance()
  }
}

function clearTargetInstance() {
  targetInstanceId.value = ''
  if (router && route) {
    const q = { ...route.query }
    delete q.instance
    router.replace({ query: q }).catch(() => {})
  }
}

const targetInstanceOptions = computed(() => [
  { label: t('comp.search.noTargetInstance'), value: '' },
  ...instancesStore.instances.map((i) => ({
    label: `${i.name} (${i.mcVersion})`,
    value: i.id,
  })),
])
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
  { label: t('comp.platform.all'), value: 'all' },
  { label: t('comp.platform.modrinth'), value: 'modrinth' },
  { label: t('comp.platform.curseforge'), value: 'curseforge' },
])
const categoryOptions = computed(() => [
  { label: t('comp.search.categories.all'), value: '' },
  { label: t('comp.search.categories.technology'), value: 'technology' },
  { label: t('comp.search.categories.magic'), value: 'magic' },
  { label: t('comp.search.categories.adventure'), value: 'adventure' },
  { label: t('comp.search.categories.optimization'), value: 'optimization' },
  { label: t('comp.search.categories.utility'), value: 'utility' },
  { label: t('comp.search.categories.decoration'), value: 'decoration' },
  { label: t('comp.search.categories.storage'), value: 'storage' },
  { label: t('comp.search.categories.equipment'), value: 'equipment' },
  { label: t('comp.search.categories.food'), value: 'food' },
  { label: t('comp.search.categories.worldgen'), value: 'worldgen' },
  { label: t('comp.search.categories.library'), value: 'library' },
  { label: t('comp.search.categories.shaderpacks'), value: 'shaderpacks' },
  { label: t('comp.search.categories.quests'), value: 'quests' },
])

const sortOptions = computed(() => [
  { label: t('comp.search.sorts.default'), value: 'relevance' },
  { label: t('comp.search.sorts.relevance'), value: 'relevance' },
  { label: t('comp.search.sorts.downloads'), value: 'downloads' },
  { label: t('comp.search.sorts.follows'), value: 'follows' },
  { label: t('comp.search.sorts.newest'), value: 'newest' },
  { label: t('comp.search.sorts.updated'), value: 'updated' },
])

const searchPlaceholder = computed(() => t('comp.search.placeholder'))
const standardMcVersions = [
  '1.21',
  '1.20.4',
  '1.20.2',
  '1.20.1',
  '1.19.4',
  '1.18.2',
  '1.16.5',
  '1.12.2',
  '1.7.10',
]

const mcVersionOptions = computed(() => {
  const versions = new Set<string>()
  for (const inst of instancesStore.instances) {
    if (inst.mcVersion?.trim()) {
      versions.add(inst.mcVersion.trim())
    }
  }
  for (const v of standardMcVersions) {
    versions.add(v)
  }

  const sorted = Array.from(versions).sort((a, b) => {
    const pa = a.split('.').map((n) => parseInt(n, 10) || 0)
    const pb = b.split('.').map((n) => parseInt(n, 10) || 0)
    for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
      const na = pa[i] ?? 0
      const nb = pb[i] ?? 0
      if (na !== nb) return nb - na
    }
    return b.localeCompare(a)
  })

  return [
    { label: t('comp.search.allMcVersions'), value: '' },
    ...sorted.map((v) => ({ label: v, value: v })),
  ]
})

const loaderOptions = computed(() => [
  { label: t('comp.search.allLoaders'), value: '' },
  { label: 'Fabric', value: 'fabric' },
  { label: 'Forge', value: 'forge' },
  { label: 'NeoForge', value: 'neoforge' },
  { label: 'Quilt', value: 'quilt' },
  { label: 'OptiFine', value: 'optifine' },
  { label: 'Iris', value: 'iris' },
])


function onTabChange(tabValue: string): void {
  compStore.activeTab = tabValue as CompType | 'favorites'
  const query = route?.query ? { ...route.query } : {}
  if (tabValue === 'favorites') {
    if (router) router.replace({ path: '/comp/favorites', query }).catch(() => {})
  } else {
    compStore.compType = tabValue as CompType
    if (router) router.replace({ path: `/comp/${tabValue}`, query }).catch(() => {})
    compStore.search(true)
  }
}

function onPlatformChange(val: string | number): void {
  compStore.platform = String(val) as CompPlatform | 'all'
  if (compStore.activeTab !== 'favorites') {
    compStore.search(true)
  }
}

async function handleToggleFavorite(project: CompProject | CompFavoriteEntry) {
  const isFav = compStore.isFavorite(
    project.platform,
    'downloads' in project ? project.id : project.projectId,
  )
  await compStore.toggleFavorite(project)
  if (!isFav) {
    toast.success(t('comp.favorite.added'))
  } else {
    toast.info(t('comp.favorite.removed'))
  }
}

function onSortChange(val: string | number): void {
  compStore.sort = String(val) as CompSort
  if (compStore.activeTab !== 'favorites') {
    compStore.search(true)
  }
}
function onCategoryChange(val: string | number): void {
  const s = String(val)
  compStore.category = s || null
  if (compStore.activeTab !== 'favorites') {
    compStore.search(true)
  }
}

function onRefresh(): void {
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


  let defaultInstId = ''
  if (route?.query?.instance) {
    defaultInstId = String(route.query.instance)
  } else if (launchStore.selectedInstance?.id) {
    defaultInstId = launchStore.selectedInstance.id
  } else if (instancesStore.instances.length > 0 && instancesStore.instances[0]?.id) {
    defaultInstId = instancesStore.instances[0].id
  }

  if (defaultInstId) {
    targetInstanceId.value = defaultInstId
    const inst = instancesStore.instances.find((i) => i.id === defaultInstId || i.name === defaultInstId) || null
    applyTargetInstance(inst, false)
  }

  if (route?.query?.q) {
    compStore.query = String(route.query.q)
  }
  await compStore.loadFavorites()
  const path = route?.path || ''
  const routeType = (route?.params?.type as string) || (path.match(/^\/comp\/(mod|modpack|resourcepack|shader|datapack|world)/)?.[1])
  if (path === '/comp/favorites' || routeType === 'favorites') {
    compStore.activeTab = 'favorites'
  } else if (routeType) {
    compStore.activeTab = routeType as CompType
    compStore.compType = routeType as CompType
    await compStore.search(true)
  } else {
    compStore.activeTab = 'mod'
    compStore.compType = 'mod'
    await compStore.search(true)
  }
})

watch(
  () => route?.params?.type || route?.path,
  () => {
    const path = route?.path || ''
    const routeType = (route?.params?.type as string) || (path.match(/^\/comp\/(mod|modpack|resourcepack|shader|datapack|world)/)?.[1])
    if (path === '/comp/favorites' || routeType === 'favorites') {
      if (compStore.activeTab !== 'favorites') {
        compStore.activeTab = 'favorites'
      }
    } else if (routeType) {
      const nextType = routeType as CompType
      if (compStore.activeTab !== nextType || compStore.compType !== nextType) {
        compStore.activeTab = nextType
        compStore.compType = nextType
        compStore.search(true)
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
    />

    <!-- Tabs Bar -->
    <MTabs
      :model-value="compStore.activeTab"
      :tabs="tabs"
      @update:model-value="onTabChange"
    />

    <!-- Filter Bar (visible when not in favorites tab) -->
    <MCard
      v-if="compStore.activeTab !== 'favorites'"
      padding="sm"
      class="flex flex-col gap-2.5 sticky top-0 z-10"
    >
      <!-- Row 1: Filters -->
      <div class="flex flex-wrap items-center gap-2.5">
        <!-- 来源 (Platform) -->
        <div class="w-32 shrink-0">
          <MSelect
            :model-value="compStore.platform"
            :options="platformOptions"
            size="sm"
            @update:model-value="onPlatformChange"
          />
        </div>

        <!-- 标签/分类 (Category) -->
        <div class="w-36 shrink-0">
          <MSelect
            :model-value="compStore.category || ''"
            :options="categoryOptions"
            size="sm"
            @update:model-value="onCategoryChange"
          />
        </div>

        <!-- 排序方式 (Sort) + 刷新按钮 -->
        <div class="flex items-center gap-1.5 shrink-0">
          <div class="w-36">
            <MSelect
              :model-value="compStore.sort"
              :options="sortOptions"
              size="sm"
              @update:model-value="onSortChange"
            />
          </div>
          <MIconButton
            icon="i-lucide-rotate-cw"
            size="sm"
            variant="tonal"
            :loading="compStore.loading"
            :title="t('common.refresh')"
            @click="onRefresh"
          />
        </div>

        <!-- 目标实例/版本 (Target Instance / Version) -->
        <div class="flex items-center gap-1.5 shrink-0">
          <MSelect
            :model-value="targetInstanceId"
            :options="targetInstanceOptions"
            size="sm"
            class="w-48"
            @update:model-value="onTargetInstanceChange"
          />
          <MSelect
            :model-value="compStore.mcVersion || ''"
            :options="mcVersionOptions"
            size="sm"
            class="w-36"
            @update:model-value="onMcVersionChange"
          />
        </div>

        <!-- 加载器 (Loader) -->
        <div class="w-36 shrink-0">
          <MSelect
            :model-value="compStore.loader || ''"
            :options="loaderOptions"
            size="sm"
            @update:model-value="onLoaderChange"
          />
        </div>
      </div>

      <!-- Row 2: Search Bar -->
      <div class="w-full">
        <MInput
          v-model="compStore.query"
          :placeholder="searchPlaceholder"
          icon="i-lucide-search"
          clearable
          @keydown.enter="compStore.search(true)"
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
                  @click.stop="handleToggleFavorite(fav)"
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
                      :icon="
                        compStore.isFavorite(project.platform, project.id)
                          ? 'i-lucide-heart fill-current text-rose-500'
                          : 'i-lucide-heart text-fg-muted hover:text-rose-500'
                      "
                      size="sm"
                      :class="
                        compStore.isFavorite(project.platform, project.id)
                          ? 'i-lucide-heart fill-current text-rose-500'
                          : 'i-lucide-heart text-fg-muted hover:text-rose-500'
                      "
                      @click.stop="handleToggleFavorite(project)"
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

  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useCompStore } from './store'
import { useInstancesStore } from '../instances/store'
import { useLaunchStore } from '../launch/store'
import type { CompType, CompPlatform, CompSort } from '../../ipc/client'
import {
  COMP_CATEGORIES_BY_TYPE,
  getLoaderDefsForType,
  isCategorySupportedOnPlatform,
  isPlatformSupportedForType,
  isSortSupportedOnPlatform,
} from './constants'
import {
  MTabs,
  MIconButton,
  MCard,
  MBadge,
  MEmpty,
  MSkeleton,
  MPageHeader,
  MPagination,
  MBrandIcon,
  MImage,
  compTypeMeta,
  type CompTypeKey,
  type SelectOption,
  getInstanceIconMeta,
  useToast,
} from '../../ui'
import type { CompFavoriteEntry, CompProject } from '../../ipc/client'
import CompSearchToolbar from './components/CompSearchToolbar.vue'
import CompProjectCard from './components/CompProjectCard.vue'
import CompQuickDownloadDialog from './components/CompQuickDownloadDialog.vue'

const router = useRouter()
const route = useRoute()
const { t } = useI18n()
const toast = useToast()
const compStore = useCompStore()
const instancesStore = useInstancesStore()
const launchStore = useLaunchStore()
const targetInstanceId = computed({
  get: () => compStore.targetInstanceId,
  set: (value: string) => {
    compStore.targetInstanceId = value
  },
})

function isForgeUnder114(mcVer: string): boolean {
  const parts = mcVer.split('.').map((p) => parseInt(p, 10))
  if (parts.length >= 2 && parts[0] === 1) {
    return (parts[1] ?? 0) < 14
  }
  return false
}

/**
 * Modpacks become instances of their own, so their search must never be bound to an
 * existing instance's version/loader — that would filter out every modpack that does not
 * match one already-installed game.
 */
const isInstanceScopedTab = computed(() => compStore.activeTab !== 'modpack')

function defaultInstanceId(): string {
  const homeInstId = launchStore.selectedInstance?.id || launchStore.selectedInstanceId || ''
  if (homeInstId) {
    const inst = instancesStore.instances.find((i) => i.id === homeInstId)
    if (inst) {
      if (compStore.compType !== 'mod' || inst.modable) return inst.id
    }
  }
  if (compStore.compType === 'mod') {
    const firstModable = instancesStore.instances.find((i) => i.modable)
    if (firstModable) return firstModable.id
  }
  return instancesStore.instances[0]?.id ?? ''
}

function extractInstanceLoader(inst: { kind?: string; loaders?: string[] }): string | null {
  const candidates: string[] = []
  if (inst.kind) {
    const k = inst.kind.toLowerCase()
    if (k !== 'vanilla' && k !== 'moddedmultiple' && k !== 'unknown') {
      candidates.push(k)
    }
  }
  if (Array.isArray(inst.loaders)) {
    for (const l of inst.loaders) {
      if (l) candidates.push(l.toLowerCase())
    }
  }

  // Priority search for primary mod loaders:
  // CRITICAL: 'neoforge' MUST be checked before 'forge'!
  // 'legacyfabric' MUST be checked before 'fabric'!
  const loaderPriorities = [
    'neoforge',
    'legacyfabric',
    'fabric',
    'quilt',
    'cleanroom',
    'forge',
    'liteloader',
    'optifine',
    'labymod',
    'iris',
  ]

  for (const known of loaderPriorities) {
    for (const candidate of candidates) {
      if (candidate.includes(known)) {
        if (known === 'legacyfabric') return 'fabric'
        return known
      }
    }
  }

  if (candidates.length > 0) {
    return candidates[0].split(/\s+/)[0] || null
  }
  return null
}

function applyTargetInstance(
  inst: { kind?: string; mcVersion?: string; loaders?: string[] } | null,
  triggerSearch = true,
) {
  if (!isInstanceScopedTab.value) return
  if (inst) {
    compStore.mcVersion = inst.mcVersion || null
    const primaryLoader = extractInstanceLoader(inst)
    // Only mods and modpacks are loadable artifacts: for every other tab the instance only
    // contributes its game version.
    if (compStore.compType !== 'mod' && compStore.compType !== 'modpack') {
      compStore.loader = null
    } else if (primaryLoader === 'forge' && inst.mcVersion && isForgeUnder114(inst.mcVersion)) {
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
  compStore.targetInstanceChosen = true
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
  compStore.targetInstanceChosen = true
  if (router && route) {
    const q = { ...route.query }
    delete q.instance
    router.replace({ query: q }).catch(() => {})
  }
}

const targetInstanceOptions = computed(() => {
  const isModTab = compStore.activeTab === 'mod'
  const availableInstances = isModTab
    ? instancesStore.instances.filter((i) => i.modable)
    : instancesStore.instances

  return [
    { label: t('comp.search.noTargetInstance'), value: '', icon: 'i-lucide-box-select' },
    ...availableInstances.map((i) => {
      const kind = i.kind || (i.loaders?.[0]?.toLowerCase() ?? 'vanilla')
      const meta = getInstanceIconMeta(i.config?.icon, kind)
      return {
        label: `${i.name} (${i.mcVersion})`,
        value: i.id,
        image: meta.image,
        icon: meta.icon,
      }
    }),
  ]
})

const tabs = computed(() => [
  { key: 'mod', label: t('comp.tabs.mod') },
  { key: 'modpack', label: t('comp.tabs.modpack') },
  { key: 'resourcepack', label: t('comp.tabs.resourcepack') },
  { key: 'shader', label: t('comp.tabs.shader') },
  { key: 'datapack', label: t('comp.tabs.datapack') },
  { key: 'world', label: t('comp.tabs.world') },
  { key: 'favorites', label: t('comp.tabs.favorites') },
])

const platformOptions = computed<SelectOption[]>(() => [
  { label: t('comp.platform.all'), value: 'all' },
  {
    label: t('comp.platform.modrinth'),
    value: 'modrinth',
    disabled: !isPlatformSupportedForType(compStore.compType, 'modrinth'),
  },
  {
    label: t('comp.platform.curseforge'),
    value: 'curseforge',
    disabled: !isPlatformSupportedForType(compStore.compType, 'curseforge'),
  },
])

/** Categories are scoped to the active tab, and each platform only supports a subset. */
const categoryOptions = computed<SelectOption[]>(() => [
  { label: t('comp.search.categories.all'), value: '' },
  ...COMP_CATEGORIES_BY_TYPE[compStore.compType].map((def) => ({
    label: t(def.labelKey),
    value: def.value,
    disabled: !isCategorySupportedOnPlatform(compStore.compType, def.value, compStore.platform),
  })),
])

const sortOptions = computed<SelectOption[]>(() =>
  [
    { label: t('comp.search.sorts.default'), value: 'relevance' },
    { label: t('comp.search.sorts.downloads'), value: 'downloads' },
    { label: t('comp.search.sorts.follows'), value: 'follows' },
    { label: t('comp.search.sorts.newest'), value: 'newest' },
    { label: t('comp.search.sorts.updated'), value: 'updated' },
  ].map((option) => ({
    ...option,
    disabled: !isSortSupportedOnPlatform(option.value as CompSort, compStore.platform),
  })),
)

const searchPlaceholder = computed(() => t('comp.search.placeholder'))
const standardMcVersions = [
  '26.3',
  '1.21.4',
  '1.21.1',
  '1.21',
  '1.20.6',
  '1.20.4',
  '1.20.2',
  '1.20.1',
  '1.19.4',
  '1.18.2',
  '1.16.5',
  '1.12.2',
  '1.10.2',
  '1.8.9',
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

const loaderDefs = computed(() => getLoaderDefsForType(compStore.compType))
const showLoader = computed(() => loaderDefs.value.length > 0)
/**
 * Disabled only when the selected platform supports none of this tab's loaders — e.g. shader
 * loaders are Modrinth-only, while mod loaders exist on CurseForge too.
 */
const loaderDisabled = computed(() => {
  if (!showLoader.value || compStore.platform === 'all') return false
  return !loaderDefs.value.some((def) => def.platforms.includes(compStore.platform as CompPlatform))
})

const loaderOptions = computed<SelectOption[]>(() => {
  const allOption: SelectOption = {
    label: t('comp.search.allLoaders'),
    value: '',
    icon: 'i-lucide-layers',
  }
  const options: SelectOption[] = [allOption]
  for (const def of loaderDefs.value) {
    options.push({
      label: def.labelKey ? t(def.labelKey) : (def.label ?? def.value),
      value: def.value,
      icon: def.icon,
      image: def.image,
      disabled: loaderDisabled.value,
    })
  }
  return options
})

function onTabChange(tabValue: string): void {
  compStore.activeTab = tabValue as CompType | 'favorites'
  const query = route?.query ? { ...route.query } : {}
  if (tabValue === 'mod') {
    const curInst = instancesStore.instances.find((i) => i.id === targetInstanceId.value)
    if (curInst && !curInst.modable) {
      clearTargetInstance()
      delete query.instance
    }
  }
  // A modpack unpacks into a brand-new instance, so nothing about it may be inherited
  // from an installed instance: start from an unconstrained search.
  if (tabValue === 'modpack') {
    clearTargetInstance()
    delete query.instance
    compStore.mcVersion = null
    compStore.loader = null
  }
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
  const isFav = compStore.hasFavorite(
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
    // Refresh must bypass the page cache, otherwise it would re-render the same snapshot.
    compStore.search(false, true)
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
const quickDownloadOpen = ref(false)
const quickDownloadProject = ref<CompProject | null>(null)

function onProjectCardClick(platform: string, id: string): void {
  const proj = compStore.hits.find((h) => h.platform === platform && h.id === id)
  if (proj) {
    quickDownloadProject.value = proj
    quickDownloadOpen.value = true
  } else {
    navigateToProject(platform, id)
  }
}

function onViewDetails(project: CompProject): void {
  navigateToProject(project.platform, project.id)
}

onMounted(async () => {
  if (
    instancesStore.instances.length === 0 &&
    typeof instancesStore.fetchInstances === 'function'
  ) {
    await instancesStore.fetchInstances()
  }

  await compStore.loadFavorites()
  await compStore.loadDisplaySettings()
  const path = route?.path || ''
  const routeType =
    (route?.params?.type as string) ||
    path.match(/^\/comp\/(mod|modpack|resourcepack|shader|datapack|world)/)?.[1]
  const routeFavorites = path === '/comp/favorites' || routeType === 'favorites'

  // The sidebar links to `/comp` with no tab, so a bare path resumes whatever tab was being
  // browsed; only an explicit `/comp/<type>` overrides it.
  //
  // A dependency quick-jump arrives as `/comp?q=<mod id>`: that is always a mod search, even
  // when the user last left the page on the favourites tab.
  const previousTab = compStore.activeTab
  const explicitType = routeType && routeType !== 'favorites' ? (routeType as CompType) : undefined
  const hasSearchQuery = Boolean(route?.query?.q)
  const nextTab: CompType | 'favorites' = hasSearchQuery
    ? (explicitType ?? 'mod')
    : routeFavorites
      ? 'favorites'
      : (explicitType ?? previousTab)

  compStore.activeTab = nextTab
  if (nextTab !== 'favorites') {
    compStore.compType = nextTab
  }
  // Same tab as before the page was left: its filters, offset and page cache still apply.
  const sameContext = nextTab === previousTab

  let filtersChanged = false
  if (nextTab !== 'favorites' && isInstanceScopedTab.value) {
    const routeInstanceId = route?.query?.instance ? String(route.query.instance) : ''
    if (routeInstanceId) {
      const inst =
        instancesStore.instances.find(
          (i) => i.id === routeInstanceId || i.name === routeInstanceId,
        ) || null
      targetInstanceId.value = inst ? inst.id : ''
      applyTargetInstance(inst, false)
      compStore.targetInstanceChosen = true
    } else if (!compStore.targetInstanceChosen) {
      const defId = defaultInstanceId()
      const inst = instancesStore.instances.find((i) => i.id === defId) || null
      targetInstanceId.value = inst ? inst.id : ''
      applyTargetInstance(inst, false)
      compStore.targetInstanceChosen = true
    } else if (targetInstanceId.value) {
      const inst = instancesStore.instances.find((i) => i.id === targetInstanceId.value) || null
      applyTargetInstance(inst, false)
    }
  }

  if (route?.query?.q) {
    compStore.query = String(route.query.q)
  }
  if (compStore.activeTab !== 'favorites') {
    // Reopening the same context resumes the page that was showing; a page-cache hit renders
    // synchronously instead of flashing a skeleton.
    await compStore.search(!sameContext || filtersChanged)
  }
})

watch(
  () => route?.params?.type || route?.path,
  () => {
    const path = route?.path || ''
    const routeType =
      (route?.params?.type as string) ||
      path.match(/^\/comp\/(mod|modpack|resourcepack|shader|datapack|world)/)?.[1]
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
    <MPageHeader :title="t('comp.title')" :description="t('comp.subtitle')" icon="i-lucide-store" />

    <!-- Tabs Bar -->
    <MTabs :model-value="compStore.activeTab" :tabs="tabs" @update:model-value="onTabChange" />

    <!-- Filter Bar (visible when not in favorites tab) -->
    <CompSearchToolbar
      v-if="compStore.activeTab !== 'favorites'"
      class="-mt-2.5"
      :comp-type="compStore.compType"
      v-model:query="compStore.query"
      :platform="compStore.platform"
      :category="compStore.category"
      :sort="compStore.sort"
      :target-instance-id="targetInstanceId"
      :mc-version="compStore.mcVersion"
      :loader="compStore.loader"
      :loading="compStore.loading"
      :platform-options="platformOptions"
      :category-options="categoryOptions"
      :sort-options="sortOptions"
      :target-instance-options="targetInstanceOptions"
      :mc-version-options="mcVersionOptions"
      :loader-options="loaderOptions"
      :show-loader="showLoader"
      :loader-disabled="loaderDisabled"
      :search-placeholder="searchPlaceholder"
      @update:platform="onPlatformChange"
      @update:category="onCategoryChange"
      @update:sort="onSortChange"
      @update:target-instance="onTargetInstanceChange"
      @update:mc-version="onMcVersionChange"
      @update:loader="onLoaderChange"
      @refresh="onRefresh"
      @search="compStore.search(true)"
    />

    <!-- Content Area: Favorites Tab -->
    <div v-if="compStore.activeTab === 'favorites'" class="flex-1 space-y-3">
      <div v-if="compStore.favorites.length === 0" class="py-16">
        <MEmpty :title="t('comp.favorites.empty')" :description="t('comp.favorites.emptyDesc')" />
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
            <MImage
              :src="fav.iconUrl"
              :fallback-icon="compTypeMeta[fav.compType as CompTypeKey]?.icon || 'i-lucide-box'"
              class="w-14 h-14 rounded-xl shrink-0"
            />
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
                  <MBrandIcon
                    v-if="fav.platform === 'modrinth' || fav.platform === 'curseforge'"
                    :name="fav.platform"
                    class="w-3.5 h-3.5 mr-1"
                  />
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
    <!-- `pb-20` keeps the floating pagination dock from covering the last card row. -->
    <div v-else class="flex-1 space-y-3 pb-20">
      <!-- Loading Skeletons -->
      <div
        v-if="compStore.loading && compStore.hits.length === 0"
        data-testid="comp-skeleton"
        class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 py-4"
      >
        <MSkeleton v-for="n in 6" :key="n" class="h-36 rounded-xl" />
      </div>

      <!-- Empty State -->
      <div v-else-if="!compStore.loading && compStore.hits.length === 0" class="py-16">
        <MEmpty :title="t('comp.search.noResults')" />
      </div>

      <!-- Results Grid -->
      <div v-else class="space-y-4">
        <div class="text-xs text-fg-muted">
          {{ t('comp.search.totalHits', { count: compStore.totalHits.toString() }) }}
        </div>

        <div data-testid="comp-grid" class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          <CompProjectCard
            v-for="project in compStore.hits"
            :key="project.platform + '-' + project.id"
            :project="project"
            :is-favorite="compStore.hasFavorite(project.platform, project.id)"
            :active-tab="compStore.activeTab"
            :show-zh-name="compStore.showZhName"
            @click="onProjectCardClick"
            @toggle-favorite="handleToggleFavorite"
          />
        </div>

        <!-- Pagination -->
        <MPagination
          v-if="compStore.hits.length > 0 && compStore.pageCount > 1"
          :page="compStore.page"
          :page-count="compStore.pageCount"
          :loading="compStore.loading"
          @update:page="compStore.goToPage"
        />
      </div>
    </div>

    <!-- Quick Download Modal -->
    <CompQuickDownloadDialog
      v-model:open="quickDownloadOpen"
      :project="quickDownloadProject"
      :initial-target-instance-id="targetInstanceId"
      @view-details="onViewDetails"
    />
  </div>
</template>

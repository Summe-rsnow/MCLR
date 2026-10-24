<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useCompStore } from './store'
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
} from '../../ui'
import ModpackImportDialog from './ModpackImportDialog.vue'

const showImportDialog = ref(false)
const router = useRouter()
const route = useRoute()
const { t } = useI18n()
const compStore = useCompStore()

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
  if (router) router.push(`/comp/project/${platform}/${id}`).catch(() => {})
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
  <div class="h-full flex flex-col p-6 space-y-4 overflow-y-auto">
    <!-- Header with Tabs -->
    <div class="flex items-center justify-between border-b border-surface-variant pb-2">
      <MTabs
        :model-value="compStore.activeTab"
        :tabs="tabs"
        @update:model-value="onTabChange"
      />
      <div v-if="compStore.activeTab === 'modpack'">
        <MButton
          variant="primary"
          size="sm"
          @click="showImportDialog = true"
        >
          <span class="i-lucide-file-up mr-1.5 w-4 h-4" />
          {{ t('comp.modpackImport.button') }}
        </MButton>
      </div>
    </div>

    <!-- Filter Bar (visible when not in favorites tab) -->
    <div
      v-if="compStore.activeTab !== 'favorites'"
      class="flex flex-wrap items-center gap-3 bg-surface-variant/20 p-3 rounded-xl border border-outline/20"
    >
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
    </div>

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
          class="cursor-pointer transition-all hover:border-primary p-4 flex flex-col justify-between"
          @click="navigateToProject(fav.platform, fav.projectId)"
        >
          <div class="flex items-start space-x-3">
            <img
              v-if="fav.iconUrl"
              :src="fav.iconUrl"
              class="w-12 h-12 rounded-lg object-cover bg-surface-variant flex-shrink-0"
              alt=""
            />
            <div
              v-else
              class="w-12 h-12 rounded-lg bg-surface-variant flex items-center justify-center flex-shrink-0"
            >
              <i class="i-lucide-box text-xl text-text-muted" />
            </div>
            <div class="flex-1 min-w-0">
              <div class="flex items-center justify-between">
                <h4 class="font-bold text-base truncate">{{ fav.title }}</h4>
                <MIconButton
                  icon="i-lucide-heart-crack"
                  size="sm"
                  class="text-danger hover:opacity-80"
                  @click.stop="compStore.toggleFavorite(fav)"
                />
              </div>
              <div class="flex items-center space-x-2 mt-1">
                <MBadge variant="neutral">{{ fav.platform }}</MBadge>
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
        <div class="text-xs text-text-muted">
          {{ t('comp.search.totalHits', { count: compStore.totalHits.toString() }) }}
        </div>

        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          <MCard
            v-for="project in compStore.hits"
            :key="project.platform + '-' + project.id"
            class="cursor-pointer transition-all hover:border-primary p-4 flex flex-col justify-between"
            @click="navigateToProject(project.platform, project.id)"
          >
            <div>
              <div class="flex items-start space-x-3">
                <img
                  v-if="project.iconUrl"
                  :src="project.iconUrl"
                  class="w-14 h-14 rounded-lg object-cover bg-surface-variant flex-shrink-0"
                  alt=""
                />
                <div
                  v-else
                  class="w-14 h-14 rounded-lg bg-surface-variant flex items-center justify-center flex-shrink-0"
                >
                  <i class="i-lucide-box text-2xl text-text-muted" />
                </div>

                <div class="flex-1 min-w-0">
                  <div class="flex items-center justify-between">
                    <h3 class="font-bold text-base truncate">{{ project.title }}</h3>
                    <MIconButton
                      :icon="compStore.isFavorite(project.platform, project.id) ? 'i-lucide-heart' : 'i-lucide-heart'"
                      size="sm"
                      :class="
                        compStore.isFavorite(project.platform, project.id)
                          ? 'text-danger hover:opacity-80'
                          : 'text-text-muted hover:text-text'
                      "
                      @click.stop="compStore.toggleFavorite(project)"
                    />
                  </div>
                  <div class="text-xs text-text-muted mt-0.5 truncate">
                    {{ t('comp.project.author', { author: project.author }) }}
                  </div>
                </div>
              </div>

              <p class="text-xs text-text-muted mt-3 line-clamp-2 h-8 leading-4">
                {{ project.summary }}
              </p>
            </div>

            <div class="mt-4 pt-3 border-t border-outline/10 flex items-center justify-between text-xs text-text-muted">
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
